//! macOS speaker audio capture using Core Audio Process Tap
//!
//! This module captures system audio output (loopback) on macOS 14.0+.
//! It uses Core Audio's process tap functionality to intercept audio
//! being played to the speakers.

use std::any::TypeId;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

use ca::aggregate_device_keys as agg_keys;
use cidre::{arc, av, cat, cf, core_audio as ca, ns, os};
use futures::Stream as FuturesStream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapCons, HeapProd, HeapRb,
};

use crate::conversion::{f64_to_f32, i16_to_f32, i32_to_f32};
use heronote_audio_core::{AudioError, AudioInput, AudioStream};

/// Device name for the audio tap aggregate device
const TAP_DEVICE_NAME: &str = "Heronote Audio Tap";

/// Number of samples per read chunk from the ring buffer
const SAMPLES_PER_CHUNK: usize = 1024;

/// Ring buffer capacity multiplier to prevent overflow during async delays
/// At 48kHz, this gives ~1.3 seconds of buffer (65536 samples)
const BUFFER_CAPACITY_MULTIPLIER: usize = 64;

/// Default sample rate when device sample rate cannot be determined
const DEFAULT_SAMPLE_RATE: u32 = 48000;

/// Speaker input handler for capturing system audio on macOS
pub struct SpeakerInput {
    tap: ca::TapGuard,
    agg_desc: arc::Retained<cf::DictionaryOf<cf::String, cf::Type>>,
}

/// Internal state for waker coordination between audio callback and async executor
struct WakerState {
    waker: Option<Waker>,
    has_data: bool,
}

/// Context passed to the Core Audio IO proc callback
struct AudioContext {
    format: arc::R<av::AudioFormat>,
    producer: HeapProd<f32>,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
}

impl AudioInput for SpeakerInput {
    type Stream = SpeakerStream;

    /// Create a new SpeakerInput
    ///
    /// Note: On macOS 14.0+, capturing system audio requires the app to have
    /// proper entitlements and may need Screen Recording permission in
    /// System Settings > Privacy & Security > Screen Recording
    fn new() -> Result<Self, AudioError> {
        let tap_desc = ca::TapDesc::with_mono_global_tap_excluding_processes(&ns::Array::new());
        let tap = tap_desc
            .create_process_tap()
            .map_err(|e| AudioError::StreamBuildError(format!("Failed to create process tap: {:?}", e)))?;

        let tap_uid = tap
            .uid()
            .map_err(|e| AudioError::DeviceError(format!("Failed to get tap UID: {:?}", e)))?;

        let sub_tap = cf::DictionaryOf::with_keys_values(
            &[ca::sub_device_keys::uid()],
            &[tap_uid.as_type_ref()],
        );

        let agg_desc = cf::DictionaryOf::with_keys_values(
            &[
                agg_keys::is_private(),
                agg_keys::tap_auto_start(),
                agg_keys::name(),
                agg_keys::uid(),
                agg_keys::tap_list(),
            ],
            &[
                cf::Boolean::value_true().as_type_ref(),
                cf::Boolean::value_false(),
                cf::String::from_str(TAP_DEVICE_NAME).as_ref(),
                &cf::Uuid::new().to_cf_string(),
                &cf::ArrayOf::from_slice(&[sub_tap.as_ref()]),
            ],
        );

        Ok(Self { tap, agg_desc })
    }

    fn sample_rate(&self) -> u32 {
        self.tap
            .asbd()
            .map(|asbd| asbd.sample_rate as u32)
            .unwrap_or(DEFAULT_SAMPLE_RATE)
    }

    /// Start capturing system audio and return a stream of samples
    fn stream(self) -> Result<SpeakerStream, AudioError> {
        let asbd = self
            .tap
            .asbd()
            .map_err(|e| AudioError::DeviceError(format!("Failed to get ASBD: {:?}", e)))?;

        let format = av::AudioFormat::with_asbd(&asbd)
            .ok_or_else(|| AudioError::DeviceError("Failed to create audio format".to_string()))?;

        let buffer_capacity = SAMPLES_PER_CHUNK * BUFFER_CAPACITY_MULTIPLIER;
        let rb = HeapRb::<f32>::new(buffer_capacity);
        let (producer, consumer) = rb.split();

        let waker_state = Arc::new(Mutex::new(WakerState {
            waker: None,
            has_data: false,
        }));

        let current_sample_rate = Arc::new(AtomicU32::new(asbd.sample_rate as u32));
        tracing::info!(sample_rate = asbd.sample_rate, "Speaker capture initialized");

        let mut ctx = Box::new(AudioContext {
            format,
            producer,
            waker_state: waker_state.clone(),
            current_sample_rate: current_sample_rate.clone(),
        });

        let device = self
            .start_device(&mut ctx)
            .map_err(|e| AudioError::StreamError(format!("Failed to start device: {:?}", e)))?;

        Ok(SpeakerStream {
            consumer,
            _device: device,
            _ctx: ctx,
            _tap: self.tap,
            waker_state,
            current_sample_rate,
            read_buffer: vec![0.0f32; SAMPLES_PER_CHUNK],
        })
    }
}

impl SpeakerInput {
    /// Start the aggregate device with IO proc callback
    fn start_device(
        &self,
        ctx: &mut Box<AudioContext>,
    ) -> Result<ca::hardware::StartedDevice<ca::AggregateDevice>, AudioError> {
        extern "C" fn proc(
            device: ca::Device,
            _now: &cat::AudioTimeStamp,
            input_data: &cat::AudioBufList<1>,
            _input_time: &cat::AudioTimeStamp,
            _output_data: &mut cat::AudioBufList<1>,
            _output_time: &cat::AudioTimeStamp,
            ctx: Option<&mut AudioContext>,
        ) -> os::Status {
            let ctx = match ctx {
                Some(c) => c,
                None => return os::Status::NO_ERR,
            };

            // Update sample rate if changed
            let after = device
                .nominal_sample_rate()
                .unwrap_or(ctx.format.absd().sample_rate) as u32;
            let before = ctx.current_sample_rate.load(Ordering::Acquire);

            if before != after {
                ctx.current_sample_rate.store(after, Ordering::Release);
                tracing::info!(before, after, "Sample rate changed");
            }

            // Try to process using AudioPcmBuf first (preferred path)
            if let Some(view) =
                av::AudioPcmBuf::with_buf_list_no_copy(&ctx.format, input_data, None)
            {
                if let Some(data) = view.data_f32_at(0) {
                    process_audio_data(ctx, data);
                    return os::Status::NO_ERR;
                }
            }

            // Fallback to manual buffer processing
            let first_buffer = &input_data.buffers[0];

            if first_buffer.data_bytes_size == 0 || first_buffer.data.is_null() {
                return os::Status::NO_ERR;
            }

            match ctx.format.common_format() {
                av::audio::CommonFormat::PcmF32 => {
                    process_samples(ctx, first_buffer, |sample: f32| sample);
                }
                av::audio::CommonFormat::PcmF64 => {
                    process_samples(ctx, first_buffer, f64_to_f32);
                }
                av::audio::CommonFormat::PcmI32 => {
                    process_samples(ctx, first_buffer, i32_to_f32);
                }
                av::audio::CommonFormat::PcmI16 => {
                    process_samples(ctx, first_buffer, i16_to_f32);
                }
                _ => {}
            }

            os::Status::NO_ERR
        }

        let agg_device = ca::AggregateDevice::with_desc(&self.agg_desc)
            .map_err(|e| AudioError::DeviceError(format!("Failed to create aggregate device: {:?}", e)))?;

        let proc_id = agg_device
            .create_io_proc_id(proc, Some(ctx))
            .map_err(|e| AudioError::StreamBuildError(format!("Failed to create IO proc: {:?}", e)))?;

        let started_device = ca::device_start(agg_device, Some(proc_id))
            .map_err(|e| AudioError::StreamError(format!("Failed to start device: {:?}", e)))?;

        Ok(started_device)
    }
}

// ============================================================================
// Audio processing utilities
// ============================================================================

/// Read samples from an audio buffer as a slice of type T
fn read_samples<T: Copy>(buffer: &cat::AudioBuf) -> Option<&[T]> {
    let byte_count = buffer.data_bytes_size as usize;

    if byte_count == 0 || buffer.data.is_null() {
        return None;
    }

    let sample_count = byte_count / std::mem::size_of::<T>();
    if sample_count == 0 {
        return None;
    }

    Some(unsafe { std::slice::from_raw_parts(buffer.data as *const T, sample_count) })
}

/// Process samples with a conversion function
fn process_samples<T, F>(ctx: &mut AudioContext, buffer: &cat::AudioBuf, mut convert: F)
where
    T: Copy + 'static,
    F: FnMut(T) -> f32,
{
    if let Some(samples) = read_samples::<T>(buffer) {
        if samples.is_empty() {
            return;
        }

        // Fast path for f32 samples
        if TypeId::of::<T>() == TypeId::of::<f32>() {
            let data = unsafe {
                std::slice::from_raw_parts(samples.as_ptr() as *const f32, samples.len())
            };
            process_audio_data(ctx, data);
            return;
        }

        // Convert samples to f32
        let converted: Vec<f32> = samples.iter().map(|s| convert(*s)).collect();
        if !converted.is_empty() {
            process_audio_data(ctx, &converted);
        }
    }
}

/// Push audio data to the ring buffer and wake the async consumer
fn process_audio_data(ctx: &mut AudioContext, data: &[f32]) {
    let pushed = ctx.producer.push_slice(data);

    if pushed < data.len() {
        let dropped = data.len() - pushed;
        tracing::warn!(dropped, "Audio samples dropped due to buffer overflow");
    }

    if pushed > 0 {
        let should_wake = {
            let mut waker_state = ctx.waker_state.lock().unwrap();
            if !waker_state.has_data {
                waker_state.has_data = true;
                waker_state.waker.take()
            } else {
                None
            }
        };

        if let Some(waker) = should_wake {
            waker.wake();
        }
    }
}

// ============================================================================
// SpeakerStream implementation
// ============================================================================

/// Stream of audio samples from system speaker output
pub struct SpeakerStream {
    consumer: HeapCons<f32>,
    _device: ca::hardware::StartedDevice<ca::AggregateDevice>,
    _ctx: Box<AudioContext>,
    _tap: ca::TapGuard,
    waker_state: Arc<Mutex<WakerState>>,
    current_sample_rate: Arc<AtomicU32>,
    read_buffer: Vec<f32>,
}

impl AudioStream for SpeakerStream {
    fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(Ordering::Acquire)
    }
}

impl FuturesStream for SpeakerStream {
    type Item = Vec<f32>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        let popped = this.consumer.pop_slice(&mut this.read_buffer);

        if popped > 0 {
            return Poll::Ready(Some(this.read_buffer[..popped].to_vec()));
        }

        {
            let mut state = this.waker_state.lock().unwrap();
            state.has_data = false;
            state.waker = Some(cx.waker().clone());
        }

        Poll::Pending
    }
}

impl Drop for SpeakerStream {
    fn drop(&mut self) {
        tracing::info!("Speaker stream stopped");
    }
}

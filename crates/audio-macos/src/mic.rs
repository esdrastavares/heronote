use std::pin::Pin;
use std::task::{Context, Poll};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use futures::Stream as FuturesStream;
use tokio::sync::mpsc as tokio_mpsc;

use crate::conversion::{convert_i16_slice_to_f32, convert_i32_slice_to_f32, convert_to_mono};
use crate::device::{get_default_input_device, get_input_device_by_name};
use heronote_audio_core::{AudioError, AudioInput, AudioStream};

/// Microphone input handler for macOS
pub struct MicInput {
    device: cpal::Device,
    config: StreamConfig,
}

impl AudioInput for MicInput {
    type Stream = MicStream;

    fn new() -> Result<Self, AudioError> {
        let device = get_default_input_device()?;
        Self::from_device(device)
    }

    fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    fn stream(self) -> Result<MicStream, AudioError> {
        let (tx, rx) = tokio_mpsc::unbounded_channel::<Vec<f32>>();
        let sample_rate = self.sample_rate();

        let supported_config = self.get_supported_config()?;
        let stream = self.build_stream(&supported_config, tx)?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        Ok(MicStream {
            _stream: stream,
            receiver: rx,
            sample_rate,
        })
    }
}

impl MicInput {
    /// Create a MicInput with a specific device name
    pub fn with_device_name(name: &str) -> Result<Self, AudioError> {
        let device = get_input_device_by_name(name)?;
        Self::from_device(device)
    }

    fn from_device(device: cpal::Device) -> Result<Self, AudioError> {
        let config = device
            .default_input_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))?;

        let config = StreamConfig {
            channels: 1,
            sample_rate: config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self { device, config })
    }

    /// Get the device name
    pub fn device_name(&self) -> Result<String, AudioError> {
        self.device
            .name()
            .map_err(|e| AudioError::DeviceError(e.to_string()))
    }

    /// Get the supported stream configuration from the device
    fn get_supported_config(&self) -> Result<SupportedStreamConfig, AudioError> {
        self.device
            .default_input_config()
            .map_err(|e| AudioError::DeviceError(e.to_string()))
    }

    /// Build the input stream based on the sample format
    ///
    /// This method handles the different sample formats (F32, I16, I32) and
    /// creates the appropriate stream that converts all audio to f32 mono.
    fn build_stream(
        &self,
        supported_config: &SupportedStreamConfig,
        tx: tokio_mpsc::UnboundedSender<Vec<f32>>,
    ) -> Result<Stream, AudioError> {
        let channels = supported_config.channels() as usize;
        let sample_format = supported_config.sample_format();

        let config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let err_fn = |err| {
            tracing::error!("Audio stream error: {}", err);
        };

        match sample_format {
            SampleFormat::F32 => self.build_f32_stream(&config, channels, tx, err_fn),
            SampleFormat::I16 => self.build_i16_stream(&config, channels, tx, err_fn),
            SampleFormat::I32 => self.build_i32_stream(&config, channels, tx, err_fn),
            _ => Err(AudioError::UnsupportedFormat),
        }
    }

    /// Build a stream for F32 sample format
    fn build_f32_stream<E>(
        &self,
        config: &StreamConfig,
        channels: usize,
        tx: tokio_mpsc::UnboundedSender<Vec<f32>>,
        err_fn: E,
    ) -> Result<Stream, AudioError>
    where
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        self.device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono = convert_to_mono(data, channels);
                    send_samples(&tx, mono);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamBuildError(e.to_string()))
    }

    /// Build a stream for I16 sample format
    fn build_i16_stream<E>(
        &self,
        config: &StreamConfig,
        channels: usize,
        tx: tokio_mpsc::UnboundedSender<Vec<f32>>,
        err_fn: E,
    ) -> Result<Stream, AudioError>
    where
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        self.device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let float_data = convert_i16_slice_to_f32(data);
                    let mono = convert_to_mono(&float_data, channels);
                    send_samples(&tx, mono);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamBuildError(e.to_string()))
    }

    /// Build a stream for I32 sample format
    fn build_i32_stream<E>(
        &self,
        config: &StreamConfig,
        channels: usize,
        tx: tokio_mpsc::UnboundedSender<Vec<f32>>,
        err_fn: E,
    ) -> Result<Stream, AudioError>
    where
        E: FnMut(cpal::StreamError) + Send + 'static,
    {
        self.device
            .build_input_stream(
                config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    let float_data = convert_i32_slice_to_f32(data);
                    let mono = convert_to_mono(&float_data, channels);
                    send_samples(&tx, mono);
                },
                err_fn,
                None,
            )
            .map_err(|e| AudioError::StreamBuildError(e.to_string()))
    }
}

/// Send audio samples through the channel with proper error logging
///
/// In audio callbacks, we cannot block or handle errors in a complex way,
/// so we log warnings if the receiver has been dropped (which indicates
/// the stream is being shut down).
fn send_samples(tx: &tokio_mpsc::UnboundedSender<Vec<f32>>, samples: Vec<f32>) {
    if let Err(e) = tx.send(samples) {
        // Only log at debug level since this typically happens during shutdown
        tracing::debug!("Failed to send audio samples (receiver dropped): {}", e);
    }
}

// ============================================================================
// MicStream implementation
// ============================================================================

/// Stream of audio samples from the microphone
pub struct MicStream {
    _stream: Stream,
    receiver: tokio_mpsc::UnboundedReceiver<Vec<f32>>,
    sample_rate: u32,
}

impl AudioStream for MicStream {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl FuturesStream for MicStream {
    type Item = Vec<f32>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

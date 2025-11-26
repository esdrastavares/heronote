//! Linux microphone capture implementation (stub)
//!
//! This module will contain the Linux-specific microphone capture
//! implementation using ALSA or PulseAudio.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream as FuturesStream;
use heronote_audio_core::{AudioError, AudioInput, AudioStream};

/// Microphone input handler for Linux (stub)
///
/// This is a placeholder implementation. The actual Linux microphone
/// capture will use ALSA or PulseAudio for audio input.
pub struct MicInput {
    // Private field to prevent external construction
    _private: (),
}

impl AudioInput for MicInput {
    type Stream = MicStream;

    fn new() -> Result<Self, AudioError> {
        Err(AudioError::PlatformNotSupported(
            "Linux mic capture coming soon".to_string(),
        ))
    }

    fn sample_rate(&self) -> u32 {
        // This method can never be called because `new()` always returns Err,
        // meaning no instance of MicInput can ever exist.
        unreachable!("MicInput cannot be instantiated on Linux (stub)")
    }

    fn stream(self) -> Result<MicStream, AudioError> {
        // This method can never be called because `new()` always returns Err
        unreachable!("MicInput cannot be instantiated on Linux (stub)")
    }
}

/// Stream of audio samples from the microphone (stub)
pub struct MicStream {
    // Private field to prevent external construction
    _private: (),
}

impl AudioStream for MicStream {
    fn sample_rate(&self) -> u32 {
        // This method can never be called because MicStream cannot be created
        unreachable!("MicStream cannot be created on Linux (stub)")
    }
}

impl FuturesStream for MicStream {
    type Item = Vec<f32>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // This method can never be called because MicStream cannot be created
        unreachable!("MicStream cannot be created on Linux (stub)")
    }
}

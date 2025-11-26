//! Windows speaker capture implementation (stub)
//!
//! This module will contain the Windows-specific speaker capture
//! implementation using WASAPI loopback capture.

use std::pin::Pin;
use std::task::{Context, Poll};

use futures::Stream as FuturesStream;
use heronote_audio_core::{AudioError, AudioInput, AudioStream};

/// Speaker input handler for Windows (stub)
///
/// This is a placeholder implementation. The actual Windows speaker
/// capture will use WASAPI loopback for capturing system audio output.
pub struct SpeakerInput {
    // Private field to prevent external construction
    _private: (),
}

impl AudioInput for SpeakerInput {
    type Stream = SpeakerStream;

    fn new() -> Result<Self, AudioError> {
        Err(AudioError::PlatformNotSupported(
            "Windows speaker capture coming soon".to_string(),
        ))
    }

    fn sample_rate(&self) -> u32 {
        // This method can never be called because `new()` always returns Err,
        // meaning no instance of SpeakerInput can ever exist.
        unreachable!("SpeakerInput cannot be instantiated on Windows (stub)")
    }

    fn stream(self) -> Result<SpeakerStream, AudioError> {
        // This method can never be called because `new()` always returns Err
        unreachable!("SpeakerInput cannot be instantiated on Windows (stub)")
    }
}

/// Stream of audio samples from system speaker output (stub)
pub struct SpeakerStream {
    // Private field to prevent external construction
    _private: (),
}

impl AudioStream for SpeakerStream {
    fn sample_rate(&self) -> u32 {
        // This method can never be called because SpeakerStream cannot be created
        unreachable!("SpeakerStream cannot be created on Windows (stub)")
    }
}

impl FuturesStream for SpeakerStream {
    type Item = Vec<f32>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // This method can never be called because SpeakerStream cannot be created
        unreachable!("SpeakerStream cannot be created on Windows (stub)")
    }
}

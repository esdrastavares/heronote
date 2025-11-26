//! Windows audio capture implementation (stub)
//!
//! This crate will contain the Windows-specific audio capture implementation
//! using WASAPI for speaker loopback capture.

mod mic;
mod speaker;
mod device;

pub use heronote_audio_core::{AudioDevice, AudioError, DeviceType, AudioInput, AudioStream};
pub use mic::{MicInput, MicStream};
pub use speaker::{SpeakerInput, SpeakerStream};
pub use device::list_devices;

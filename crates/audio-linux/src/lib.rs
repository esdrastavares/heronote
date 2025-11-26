//! Linux audio capture implementation (stub)
//!
//! This crate will contain the Linux-specific audio capture implementation
//! using ALSA and PulseAudio.

mod mic;
mod speaker;
mod device;

pub use heronote_audio_core::{AudioDevice, AudioError, DeviceType, AudioInput, AudioStream};
pub use mic::{MicInput, MicStream};
pub use speaker::{SpeakerInput, SpeakerStream};
pub use device::list_devices;

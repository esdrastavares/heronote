use heronote_audio_core::{AudioDevice, AudioError};

/// List all available audio devices on Windows
pub fn list_devices() -> Result<Vec<AudioDevice>, AudioError> {
    // TODO: Implement Windows device enumeration using WASAPI
    Err(AudioError::PlatformNotSupported("Windows support coming soon".to_string()))
}

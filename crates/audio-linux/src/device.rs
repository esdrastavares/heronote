use heronote_audio_core::{AudioDevice, AudioError};

/// List all available audio devices on Linux
pub fn list_devices() -> Result<Vec<AudioDevice>, AudioError> {
    // TODO: Implement Linux device enumeration using ALSA/PulseAudio
    Err(AudioError::PlatformNotSupported("Linux support coming soon".to_string()))
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("No audio device found")]
    NoDeviceFound,

    #[error("Device not available: {0}")]
    DeviceNotAvailable(String),

    #[error("Failed to build audio stream: {0}")]
    StreamBuildError(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Device error: {0}")]
    DeviceError(String),

    #[error("Unsupported sample format")]
    UnsupportedFormat,

    #[error("Permission denied for audio capture")]
    PermissionDenied,

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

use crate::error::AudioError;

/// Trait for audio input sources (microphone, speaker loopback)
pub trait AudioInput: Sized {
    type Stream: AudioStream;

    /// Create a new audio input with default device
    fn new() -> Result<Self, AudioError>;

    /// Get the sample rate in Hz
    fn sample_rate(&self) -> u32;

    /// Start capturing and return an audio stream
    fn stream(self) -> Result<Self::Stream, AudioError>;
}

/// Trait for audio streams that produce samples
pub trait AudioStream: futures::Stream<Item = Vec<f32>> {
    /// Get the sample rate of this stream
    fn sample_rate(&self) -> u32;
}

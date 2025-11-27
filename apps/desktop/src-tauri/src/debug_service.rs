//! Debug services for audio capture
//!
//! Provides utilities for:
//! - Writing audio to WAV files
//! - Collecting and broadcasting metrics
//! - Managing debug logs

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::PathBuf;

use chrono::Utc;
use hound::{SampleFormat, WavSpec, WavWriter};
use tauri::{AppHandle, Emitter};

use crate::debug_state::{AudioSource, DebugAudioFile, DebugLogEntry, DebugState};

// ============================================================================
// Constants
// ============================================================================

/// Default WAV file configuration
#[allow(dead_code)]
const WAV_CHANNELS: u16 = 1;
#[allow(dead_code)]
const WAV_BITS_PER_SAMPLE: u16 = 32;

// ============================================================================
// Audio Writer
// ============================================================================

/// WAV file writer for debug audio capture
///
/// Implements the Null Object pattern - if debug is disabled,
/// all operations become no-ops without requiring conditional checks.
#[allow(dead_code)]
pub struct DebugAudioWriter {
    writer: Option<WavWriter<BufWriter<File>>>,
    path: PathBuf,
    source: AudioSource,
    sample_rate: u32,
    samples_written: u64,
}

#[allow(dead_code)]
impl DebugAudioWriter {
    /// Create a new debug audio writer
    ///
    /// If debug mode is disabled or save_audio_files is false,
    /// this returns a writer that does nothing (Null Object pattern).
    pub fn new(
        debug_state: &DebugState,
        source: AudioSource,
        sample_rate: u32,
    ) -> Result<Self, String> {
        let config = debug_state.config();

        // Null object - does nothing when debug is disabled
        if !config.enabled || !config.save_audio_files {
            return Ok(Self::null(source, sample_rate));
        }

        // Create output directory if it doesn't exist
        fs::create_dir_all(&config.audio_output_dir)
            .map_err(|e| format!("Failed to create debug audio directory: {}", e))?;

        // Generate filename with timestamp
        let path = Self::generate_file_path(&config.audio_output_dir, source);
        let writer = Self::create_wav_writer(&path, sample_rate)?;

        tracing::info!(
            path = %path.display(),
            source = %source,
            sample_rate,
            "Debug audio writer created"
        );

        Ok(Self {
            writer: Some(writer),
            path,
            source,
            sample_rate,
            samples_written: 0,
        })
    }

    /// Create a null writer that does nothing
    fn null(source: AudioSource, sample_rate: u32) -> Self {
        Self {
            writer: None,
            path: PathBuf::new(),
            source,
            sample_rate,
            samples_written: 0,
        }
    }

    /// Generate a unique file path for the audio file
    fn generate_file_path(output_dir: &PathBuf, source: AudioSource) -> PathBuf {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S%.3f");
        let filename = format!("{}_{}.wav", source.as_str(), timestamp);
        output_dir.join(filename)
    }

    /// Create a WAV writer with the specified configuration
    fn create_wav_writer(
        path: &PathBuf,
        sample_rate: u32,
    ) -> Result<WavWriter<BufWriter<File>>, String> {
        let spec = WavSpec {
            channels: WAV_CHANNELS,
            sample_rate,
            bits_per_sample: WAV_BITS_PER_SAMPLE,
            sample_format: SampleFormat::Float,
        };

        WavWriter::create(path, spec).map_err(|e| format!("Failed to create WAV file: {}", e))
    }

    /// Write audio samples to the WAV file
    pub fn write_samples(&mut self, samples: &[f32]) -> Result<(), String> {
        if let Some(ref mut writer) = self.writer {
            for &sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|e| format!("Failed to write sample: {}", e))?;
            }
            self.samples_written += samples.len() as u64;
        }
        Ok(())
    }

    /// Check if this writer is active (actually writing to a file)
    pub fn is_active(&self) -> bool {
        self.writer.is_some()
    }

    /// Get the number of samples written
    pub fn samples_written(&self) -> u64 {
        self.samples_written
    }

    /// Finalize the WAV file and register it with the debug state
    pub fn finalize(mut self, debug_state: &DebugState) -> Result<Option<DebugAudioFile>, String> {
        let Some(writer) = self.writer.take() else {
            return Ok(None);
        };

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV file: {}", e))?;

        let file_info = self.create_file_info();
        debug_state.register_file(file_info.clone());

        tracing::info!(
            path = %self.path.display(),
            duration_secs = file_info.duration_secs,
            samples = self.samples_written,
            size_bytes = file_info.size_bytes,
            "Debug audio file saved"
        );

        Ok(Some(file_info))
    }

    /// Create file info struct from current state
    fn create_file_info(&self) -> DebugAudioFile {
        let duration_secs = self.samples_written as f32 / self.sample_rate as f32;
        let size_bytes = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);

        DebugAudioFile {
            path: self.path.clone(),
            source: self.source,
            created_at: Utc::now(),
            duration_secs,
            sample_rate: self.sample_rate,
            size_bytes,
        }
    }
}

// ============================================================================
// Event Emitters
// ============================================================================

/// Tauri event names for debug communication
#[allow(dead_code)]
pub mod events {
    pub const METRICS: &str = "debug:metrics";
    pub const LOG: &str = "debug:log";
    pub const FILE_SAVED: &str = "debug:file-saved";
}

/// Broadcast debug metrics to the frontend
#[allow(dead_code)]
pub fn emit_metrics(app: &AppHandle, debug_state: &DebugState) {
    if !debug_state.is_enabled() {
        return;
    }

    let metrics = debug_state.flat_metrics();
    if let Err(e) = app.emit(events::METRICS, &metrics) {
        tracing::warn!("Failed to emit debug metrics: {}", e);
    }
}

/// Broadcast a debug log entry to the frontend
#[allow(dead_code)]
pub fn emit_log(app: &AppHandle, debug_state: &DebugState, entry: &DebugLogEntry) {
    if !debug_state.is_enabled() {
        return;
    }

    if let Err(e) = app.emit(events::LOG, entry) {
        tracing::warn!("Failed to emit debug log: {}", e);
    }
}

/// Convenience function to emit an info log
#[allow(dead_code)]
pub fn emit_info(app: &AppHandle, debug_state: &DebugState, message: impl Into<String>) {
    emit_log(app, debug_state, &DebugLogEntry::info(message));
}

/// Convenience function to emit a warning log
#[allow(dead_code)]
pub fn emit_warn(app: &AppHandle, debug_state: &DebugState, message: impl Into<String>) {
    emit_log(app, debug_state, &DebugLogEntry::warn(message));
}

/// Convenience function to emit an error log
#[allow(dead_code)]
pub fn emit_error(app: &AppHandle, debug_state: &DebugState, message: impl Into<String>) {
    emit_log(app, debug_state, &DebugLogEntry::error(message));
}

/// Broadcast file saved notification to the frontend
#[allow(dead_code)]
pub fn emit_file_saved(app: &AppHandle, file: &DebugAudioFile) {
    if let Err(e) = app.emit(events::FILE_SAVED, file) {
        tracing::warn!("Failed to emit file saved event: {}", e);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug_state::LogLevel;

    #[test]
    fn test_audio_source_display() {
        assert_eq!(AudioSource::Mic.as_str(), "mic");
        assert_eq!(AudioSource::Speaker.as_str(), "speaker");
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }
}

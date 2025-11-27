//! Debug mode state management
//!
//! Provides thread-safe state for debug mode features.
//! Only active in debug builds or when explicitly enabled.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of log entries to keep in memory
#[allow(dead_code)]
pub const MAX_LOG_ENTRIES: usize = 100;

/// Default application identifier for directory paths
const APP_QUALIFIER: &str = "com";
const APP_ORGANIZATION: &str = "heronote";
const APP_NAME: &str = "app";
const DEBUG_AUDIO_DIR: &str = "debug_audio";

// ============================================================================
// Enums
// ============================================================================

/// Audio source type for debug recording
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioSource {
    Mic,
    Speaker,
}

impl AudioSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mic => "mic",
            Self::Speaker => "speaker",
        }
    }
}

impl std::fmt::Display for AudioSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Log level for debug entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[allow(dead_code)]
impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

// ============================================================================
// Configuration
// ============================================================================

/// Debug mode configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    pub enabled: bool,
    pub save_audio_files: bool,
    pub log_audio_buffers: bool,
    pub log_performance: bool,
    pub audio_output_dir: PathBuf,
}

impl Default for DebugConfig {
    fn default() -> Self {
        let audio_dir =
            directories::ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_NAME)
                .map(|dirs| dirs.data_local_dir().join(DEBUG_AUDIO_DIR))
                .unwrap_or_else(|| PathBuf::from(format!("./{}", DEBUG_AUDIO_DIR)));

        Self {
            enabled: false,
            save_audio_files: true,
            log_audio_buffers: true,
            log_performance: true,
            audio_output_dir: audio_dir,
        }
    }
}

// ============================================================================
// Metrics
// ============================================================================

/// Metrics for a single audio source
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMetrics {
    pub sample_rate: u32,
    pub buffer_usage_percent: f32,
    pub samples_processed: u64,
    pub samples_dropped: u64,
    pub latency_ms: f32,
    pub device_name: Option<String>,
    pub capturing: bool,
}

/// Real-time audio metrics for all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMetrics {
    pub mic: SourceMetrics,
    pub speaker: SourceMetrics,
    pub last_update: DateTime<Utc>,
}

impl Default for AudioMetrics {
    fn default() -> Self {
        Self {
            mic: SourceMetrics::default(),
            speaker: SourceMetrics::default(),
            last_update: Utc::now(),
        }
    }
}

// Backward compatibility: flatten mic/speaker fields for frontend
impl AudioMetrics {
    /// Create a flattened version for JSON serialization (backward compatibility)
    pub fn to_flat(&self) -> FlatAudioMetrics {
        FlatAudioMetrics {
            mic_sample_rate: self.mic.sample_rate,
            speaker_sample_rate: self.speaker.sample_rate,
            mic_buffer_usage_percent: self.mic.buffer_usage_percent,
            speaker_buffer_usage_percent: self.speaker.buffer_usage_percent,
            mic_samples_processed: self.mic.samples_processed,
            speaker_samples_processed: self.speaker.samples_processed,
            mic_samples_dropped: self.mic.samples_dropped,
            speaker_samples_dropped: self.speaker.samples_dropped,
            mic_latency_ms: self.mic.latency_ms,
            speaker_latency_ms: self.speaker.latency_ms,
            mic_device_name: self.mic.device_name.clone(),
            speaker_device_name: self.speaker.device_name.clone(),
            mic_capturing: self.mic.capturing,
            speaker_capturing: self.speaker.capturing,
            last_update: self.last_update,
        }
    }
}

/// Flattened metrics structure for frontend compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatAudioMetrics {
    pub mic_sample_rate: u32,
    pub speaker_sample_rate: u32,
    pub mic_buffer_usage_percent: f32,
    pub speaker_buffer_usage_percent: f32,
    pub mic_samples_processed: u64,
    pub speaker_samples_processed: u64,
    pub mic_samples_dropped: u64,
    pub speaker_samples_dropped: u64,
    pub mic_latency_ms: f32,
    pub speaker_latency_ms: f32,
    pub mic_device_name: Option<String>,
    pub speaker_device_name: Option<String>,
    pub mic_capturing: bool,
    pub speaker_capturing: bool,
    pub last_update: DateTime<Utc>,
}

// ============================================================================
// Debug Files
// ============================================================================

/// Debug audio file info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugAudioFile {
    pub path: PathBuf,
    pub source: AudioSource,
    pub created_at: DateTime<Utc>,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub size_bytes: u64,
}

// ============================================================================
// Log Entry
// ============================================================================

/// Debug log entry for frontend display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

#[allow(dead_code)]
impl DebugLogEntry {
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, message)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }
}

// ============================================================================
// Atomic Counters (for high-frequency updates)
// ============================================================================

/// Thread-safe atomic counters for a single audio source
struct AtomicSourceCounters {
    samples_processed: AtomicU64,
    samples_dropped: AtomicU64,
}

impl Default for AtomicSourceCounters {
    fn default() -> Self {
        Self {
            samples_processed: AtomicU64::new(0),
            samples_dropped: AtomicU64::new(0),
        }
    }
}

impl AtomicSourceCounters {
    #[allow(dead_code)]
    fn add_samples(&self, count: u64) {
        self.samples_processed.fetch_add(count, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    fn add_dropped(&self, count: u64) {
        self.samples_dropped.fetch_add(count, Ordering::Relaxed);
    }

    fn get_samples(&self) -> u64 {
        self.samples_processed.load(Ordering::Relaxed)
    }

    fn get_dropped(&self) -> u64 {
        self.samples_dropped.load(Ordering::Relaxed)
    }

    fn reset(&self) {
        self.samples_processed.store(0, Ordering::Relaxed);
        self.samples_dropped.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// Debug State
// ============================================================================

/// Thread-safe debug state
pub struct DebugState {
    config: RwLock<DebugConfig>,
    metrics: RwLock<AudioMetrics>,
    files: RwLock<Vec<DebugAudioFile>>,
    mic_counters: AtomicSourceCounters,
    speaker_counters: AtomicSourceCounters,
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            config: RwLock::new(DebugConfig::default()),
            metrics: RwLock::new(AudioMetrics::default()),
            files: RwLock::new(Vec::new()),
            mic_counters: AtomicSourceCounters::default(),
            speaker_counters: AtomicSourceCounters::default(),
        }
    }
}

impl DebugState {
    // ========================================================================
    // Configuration
    // ========================================================================

    /// Check if debug mode is enabled
    #[allow(dead_code)]
    pub fn is_enabled(&self) -> bool {
        self.config.read().unwrap().enabled
    }

    /// Enable or disable debug mode
    pub fn set_enabled(&self, enabled: bool) {
        self.config.write().unwrap().enabled = enabled;
    }

    /// Get a copy of the current configuration
    pub fn config(&self) -> DebugConfig {
        self.config.read().unwrap().clone()
    }

    /// Update the configuration
    #[allow(dead_code)]
    pub fn update_config(&self, config: DebugConfig) {
        *self.config.write().unwrap() = config;
    }

    // ========================================================================
    // Metrics
    // ========================================================================

    /// Get a copy of the current metrics with atomic counter values
    pub fn metrics(&self) -> AudioMetrics {
        let mut metrics = self.metrics.read().unwrap().clone();

        // Update with atomic counter values
        metrics.mic.samples_processed = self.mic_counters.get_samples();
        metrics.mic.samples_dropped = self.mic_counters.get_dropped();
        metrics.speaker.samples_processed = self.speaker_counters.get_samples();
        metrics.speaker.samples_dropped = self.speaker_counters.get_dropped();

        metrics
    }

    /// Get flattened metrics for frontend compatibility
    pub fn flat_metrics(&self) -> FlatAudioMetrics {
        self.metrics().to_flat()
    }

    /// Update metrics with a closure
    pub fn update_metrics<F>(&self, f: F)
    where
        F: FnOnce(&mut AudioMetrics),
    {
        let mut metrics = self.metrics.write().unwrap();
        f(&mut metrics);
        metrics.last_update = Utc::now();
    }

    // ========================================================================
    // Counters (high-frequency updates)
    // ========================================================================

    /// Add samples processed for a specific source
    #[allow(dead_code)]
    pub fn add_samples(&self, source: AudioSource, count: u64) {
        match source {
            AudioSource::Mic => self.mic_counters.add_samples(count),
            AudioSource::Speaker => self.speaker_counters.add_samples(count),
        }
    }

    /// Add dropped samples for a specific source
    #[allow(dead_code)]
    pub fn add_dropped(&self, source: AudioSource, count: u64) {
        match source {
            AudioSource::Mic => self.mic_counters.add_dropped(count),
            AudioSource::Speaker => self.speaker_counters.add_dropped(count),
        }
    }

    /// Reset all counters
    pub fn reset_counters(&self) {
        self.mic_counters.reset();
        self.speaker_counters.reset();
    }

    // ========================================================================
    // Files
    // ========================================================================

    /// Register a debug audio file
    #[allow(dead_code)]
    pub fn register_file(&self, file: DebugAudioFile) {
        self.files.write().unwrap().push(file);
    }

    /// List all registered debug audio files
    #[allow(dead_code)]
    pub fn list_files(&self) -> Vec<DebugAudioFile> {
        self.files.read().unwrap().clone()
    }
}

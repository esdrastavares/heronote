//! Thread-safe audio capture state management
//!
//! This module provides the [`AudioState`] struct which manages the lifecycle
//! of audio capture threads using atomic flags.
//!
//! # Thread Safety
//!
//! Audio streams (e.g., `cpal::Stream`) are typically not `Send` or `Sync`,
//! meaning they cannot be shared across threads. To work around this limitation,
//! we use atomic flags to control the capture state:
//!
//! - `*_running`: Indicates whether a capture thread is currently active
//! - `*_stop_signal`: Signals the capture thread to stop gracefully
//!
//! The actual audio stream is created and owned by the capture thread itself,
//! ensuring it never crosses thread boundaries.
//!
//! # Ordering
//!
//! We use `SeqCst` (sequentially consistent) ordering for all atomic operations
//! to ensure the strongest guarantees about operation ordering across threads.
//! While `Release`/`Acquire` might suffice for some operations, `SeqCst`
//! provides simpler reasoning about correctness with negligible performance impact.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Thread-safe audio capture state
///
/// Uses atomic flags instead of storing streams directly because `cpal::Stream`
/// is not `Send` or `Sync`. Each capture operation spawns a thread that owns
/// the stream, and uses these flags to coordinate start/stop operations.
///
/// # Example
///
/// ```ignore
/// let state = AudioState::default();
///
/// // Check if microphone is capturing
/// if state.is_mic_running() {
///     println!("Mic is active");
/// }
///
/// // Signal mic to stop
/// state.signal_mic_stop();
/// ```
pub struct AudioState {
    /// Whether the microphone capture thread is currently running
    mic_running: Arc<AtomicBool>,
    /// Signal to stop the microphone capture thread
    mic_stop_signal: Arc<AtomicBool>,

    /// Whether the speaker capture thread is currently running (macOS only)
    #[cfg(target_os = "macos")]
    speaker_running: Arc<AtomicBool>,
    /// Signal to stop the speaker capture thread (macOS only)
    #[cfg(target_os = "macos")]
    speaker_stop_signal: Arc<AtomicBool>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            mic_running: Arc::new(AtomicBool::new(false)),
            mic_stop_signal: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "macos")]
            speaker_running: Arc::new(AtomicBool::new(false)),
            #[cfg(target_os = "macos")]
            speaker_stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl AudioState {
    // ========================================================================
    // Microphone state management
    // ========================================================================

    /// Check if microphone capture is currently running
    pub fn is_mic_running(&self) -> bool {
        self.mic_running.load(Ordering::SeqCst)
    }

    /// Set the microphone running state
    pub fn set_mic_running(&self, running: bool) {
        self.mic_running.store(running, Ordering::SeqCst);
    }

    /// Get a clone of the mic running flag for use in a capture thread
    pub fn mic_running_handle(&self) -> Arc<AtomicBool> {
        self.mic_running.clone()
    }

    /// Check if a stop signal has been sent to the microphone capture
    #[allow(dead_code)]
    pub fn is_mic_stop_signaled(&self) -> bool {
        self.mic_stop_signal.load(Ordering::SeqCst)
    }

    /// Signal the microphone capture thread to stop
    pub fn signal_mic_stop(&self) {
        self.mic_stop_signal.store(true, Ordering::SeqCst);
    }

    /// Reset the mic stop signal (call before starting capture)
    pub fn reset_mic_stop_signal(&self) {
        self.mic_stop_signal.store(false, Ordering::SeqCst);
    }

    /// Get a clone of the mic stop signal for use in a capture thread
    pub fn mic_stop_signal_handle(&self) -> Arc<AtomicBool> {
        self.mic_stop_signal.clone()
    }

    // ========================================================================
    // Speaker state management (macOS only)
    // ========================================================================

    /// Check if speaker capture is currently running
    #[cfg(target_os = "macos")]
    pub fn is_speaker_running(&self) -> bool {
        self.speaker_running.load(Ordering::SeqCst)
    }

    /// Set the speaker running state
    #[cfg(target_os = "macos")]
    pub fn set_speaker_running(&self, running: bool) {
        self.speaker_running.store(running, Ordering::SeqCst);
    }

    /// Get a clone of the speaker running flag for use in a capture thread
    #[cfg(target_os = "macos")]
    pub fn speaker_running_handle(&self) -> Arc<AtomicBool> {
        self.speaker_running.clone()
    }

    /// Check if a stop signal has been sent to the speaker capture
    #[cfg(target_os = "macos")]
    #[allow(dead_code)]
    pub fn is_speaker_stop_signaled(&self) -> bool {
        self.speaker_stop_signal.load(Ordering::SeqCst)
    }

    /// Signal the speaker capture thread to stop
    #[cfg(target_os = "macos")]
    pub fn signal_speaker_stop(&self) {
        self.speaker_stop_signal.store(true, Ordering::SeqCst);
    }

    /// Reset the speaker stop signal (call before starting capture)
    #[cfg(target_os = "macos")]
    pub fn reset_speaker_stop_signal(&self) {
        self.speaker_stop_signal.store(false, Ordering::SeqCst);
    }

    /// Get a clone of the speaker stop signal for use in a capture thread
    #[cfg(target_os = "macos")]
    pub fn speaker_stop_signal_handle(&self) -> Arc<AtomicBool> {
        self.speaker_stop_signal.clone()
    }
}

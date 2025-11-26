//! Audio capture service layer
//!
//! This module provides utility functions for audio capture management.
//! The actual stream handling is done in the commands module because
//! `cpal::Stream` is not `Send`/`Sync` and must be created and held
//! within the same thread.
//!
//! # Design
//!
//! Due to `cpal::Stream` not being `Send`, we cannot create a fully generic
//! service that handles stream creation. Instead, this module provides:
//!
//! - Constants for capture configuration
//! - The capture loop logic that runs in spawned threads
//! - State management helpers

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Poll interval for checking the stop signal in capture threads
pub const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Run the capture loop until the stop signal is received
///
/// This function blocks the current thread, polling the stop signal
/// at regular intervals. The stream must be kept alive in the calling
/// scope for the duration of this loop.
///
/// # Arguments
///
/// * `stop_signal` - Atomic flag that signals when to stop capturing
/// * `running` - Atomic flag to set to false when capture stops
/// * `name` - Name of the capture source for logging
///
/// # Example
///
/// ```ignore
/// thread::spawn(move || {
///     let stream = create_audio_stream();
///     run_capture_loop(&stop_signal, &running, "Microphone");
///     // stream is dropped here
/// });
/// ```
pub fn run_capture_loop(
    stop_signal: &Arc<AtomicBool>,
    running: &Arc<AtomicBool>,
    name: &str,
) {
    tracing::info!("{} capture started", name);

    // Keep the stream alive until stop signal
    while !stop_signal.load(Ordering::SeqCst) {
        thread::sleep(POLL_INTERVAL);
    }

    // Mark as stopped
    running.store(false, Ordering::SeqCst);
    tracing::info!("{} capture stopped", name);
}

/// Mark capture as failed and log the error
///
/// Helper function to handle capture initialization failures consistently.
pub fn handle_capture_error(running: &Arc<AtomicBool>, name: &str, error: &dyn std::fmt::Display) {
    tracing::error!("Failed to create {} input: {}", name, error);
    running.store(false, Ordering::SeqCst);
}

/// Mark stream creation as failed and log the error
pub fn handle_stream_error(running: &Arc<AtomicBool>, name: &str, error: &dyn std::fmt::Display) {
    tracing::error!("Failed to start {} stream: {}", name, error);
    running.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_interval_is_reasonable() {
        // Polling should be frequent enough to be responsive but not too aggressive
        assert!(POLL_INTERVAL >= Duration::from_millis(50));
        assert!(POLL_INTERVAL <= Duration::from_millis(500));
    }
}

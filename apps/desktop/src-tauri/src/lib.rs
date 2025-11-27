//! Heronote Desktop Application
//!
//! A Tauri-based desktop application for capturing and processing audio
//! from both microphone and system audio sources.
//!
//! # Architecture
//!
//! The application is organized into the following modules:
//!
//! - [`audio_state`]: Thread-safe state management for audio capture
//! - [`audio_service`]: Service layer for audio capture operations
//! - [`commands`]: Tauri command handlers exposed to the frontend
//! - [`debug_state`]: Debug mode state management (debug builds only)
//! - [`debug_service`]: Debug services for metrics and file writing (debug builds only)
//!
//! # Platform Support
//!
//! - **macOS**: Full support for microphone and system audio capture
//! - **Windows**: Microphone capture (system audio coming soon)
//! - **Linux**: Microphone capture (system audio coming soon)

mod audio_service;
mod audio_state;
mod commands;

#[cfg(debug_assertions)]
mod debug_service;
#[cfg(debug_assertions)]
mod debug_state;

use audio_state::AudioState;
use commands::{
    // Audio commands
    is_mic_capturing, is_speaker_capturing, list_audio_devices, start_mic_capture,
    start_speaker_capture, stop_mic_capture, stop_speaker_capture,
    // Permission commands
    check_screen_recording_permission, open_screen_recording_settings,
    request_screen_recording_permission,
    // Debug commands
    get_debug_audio_dir, get_debug_config, get_debug_metrics, is_debug_available,
    list_debug_files, reset_debug_counters, toggle_debug_mode,
};

#[cfg(debug_assertions)]
use debug_state::DebugState;

/// Application entry point
///
/// Initializes the Tauri application with:
/// - Logging via `tracing_subscriber` (enhanced in debug builds)
/// - Audio state management
/// - Debug state management (debug builds only)
/// - Shell plugin for system integration
/// - All audio and debug command handlers
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Enhanced logging for debug builds
    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .init();
        tracing::info!("Debug mode logging enabled");
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .compact()
            .init();
    }

    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AudioState::default());

    // Add debug state only in debug builds
    #[cfg(debug_assertions)]
    {
        builder = builder.manage(DebugState::default());
        tracing::debug!("Debug state initialized");
    }

    builder
        .invoke_handler(tauri::generate_handler![
            // Audio commands
            list_audio_devices,
            start_mic_capture,
            stop_mic_capture,
            is_mic_capturing,
            start_speaker_capture,
            stop_speaker_capture,
            is_speaker_capturing,
            // Permission commands
            check_screen_recording_permission,
            request_screen_recording_permission,
            open_screen_recording_settings,
            // Debug commands
            is_debug_available,
            toggle_debug_mode,
            get_debug_config,
            get_debug_metrics,
            list_debug_files,
            get_debug_audio_dir,
            reset_debug_counters,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

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
//!
//! # Platform Support
//!
//! - **macOS**: Full support for microphone and system audio capture
//! - **Windows**: Microphone capture (system audio coming soon)
//! - **Linux**: Microphone capture (system audio coming soon)

mod audio_service;
mod audio_state;
mod commands;

use audio_state::AudioState;
use commands::{
    is_mic_capturing, is_speaker_capturing, list_audio_devices, start_mic_capture,
    start_speaker_capture, stop_mic_capture, stop_speaker_capture,
};

/// Application entry point
///
/// Initializes the Tauri application with:
/// - Logging via `tracing_subscriber`
/// - Audio state management
/// - Shell plugin for system integration
/// - All audio-related command handlers
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AudioState::default())
        .invoke_handler(tauri::generate_handler![
            list_audio_devices,
            start_mic_capture,
            stop_mic_capture,
            is_mic_capturing,
            start_speaker_capture,
            stop_speaker_capture,
            is_speaker_capturing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

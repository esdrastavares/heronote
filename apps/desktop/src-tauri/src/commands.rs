//! Tauri command handlers for audio operations
//!
//! This module contains all the Tauri-exposed commands for controlling
//! audio capture. Due to `cpal::Stream` not being `Send`/`Sync`, the stream
//! must be created within the capture thread.

use std::thread;

use tauri::State;

use heronote_audio_core::{AudioDevice, AudioInput};

use crate::audio_service::{handle_capture_error, handle_stream_error, run_capture_loop};
use crate::audio_state::AudioState;

#[cfg(target_os = "macos")]
use heronote_audio_macos::{list_devices, MicInput, SpeakerInput};

#[cfg(target_os = "windows")]
use heronote_audio_windows::{list_devices, MicInput};

#[cfg(target_os = "linux")]
use heronote_audio_linux::{list_devices, MicInput};

// ============================================================================
// Device listing
// ============================================================================

/// List all available audio input/output devices
#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_devices().map_err(|e| e.to_string())
}

// ============================================================================
// Microphone capture commands
// ============================================================================

/// Start capturing audio from the default microphone
///
/// # Errors
///
/// Returns an error if:
/// - Microphone capture is already running
/// - The microphone device cannot be accessed
#[tauri::command]
pub fn start_mic_capture(state: State<AudioState>) -> Result<(), String> {
    if state.is_mic_running() {
        return Err("Microphone capture is already running".to_string());
    }

    // Verify device exists before spawning thread
    let _ = MicInput::new().map_err(|e| e.to_string())?;

    // Update state
    state.set_mic_running(true);
    state.reset_mic_stop_signal();

    // Get handles for the capture thread
    let running = state.mic_running_handle();
    let stop_signal = state.mic_stop_signal_handle();

    // Stream must be created inside the thread because cpal::Stream is not Send
    thread::spawn(move || {
        let mic = match MicInput::new() {
            Ok(m) => m,
            Err(e) => {
                handle_capture_error(&running, "Microphone", &e);
                return;
            }
        };

        let _stream = match mic.stream() {
            Ok(s) => s,
            Err(e) => {
                handle_stream_error(&running, "Microphone", &e);
                return;
            }
        };

        // Keep stream alive until stop signal (stream is dropped when function returns)
        run_capture_loop(&stop_signal, &running, "Microphone");
    });

    Ok(())
}

/// Stop the current microphone capture
///
/// # Errors
///
/// Returns an error if microphone capture is not running
#[tauri::command]
pub fn stop_mic_capture(state: State<AudioState>) -> Result<(), String> {
    if !state.is_mic_running() {
        return Err("Microphone capture is not running".to_string());
    }

    state.signal_mic_stop();
    Ok(())
}

/// Check if microphone capture is currently active
#[tauri::command]
pub fn is_mic_capturing(state: State<AudioState>) -> bool {
    state.is_mic_running()
}

// ============================================================================
// Speaker capture commands (macOS only)
// ============================================================================

/// Start capturing system audio output (macOS only)
///
/// # Errors
///
/// Returns an error if:
/// - Speaker capture is already running
/// - System audio capture is not available
/// - Required permissions are not granted
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn start_speaker_capture(state: State<AudioState>) -> Result<(), String> {
    if state.is_speaker_running() {
        return Err("Speaker capture is already running".to_string());
    }

    // Verify we can create speaker input before spawning thread
    let _ = SpeakerInput::new().map_err(|e| e.to_string())?;

    // Update state
    state.set_speaker_running(true);
    state.reset_speaker_stop_signal();

    // Get handles for the capture thread
    let running = state.speaker_running_handle();
    let stop_signal = state.speaker_stop_signal_handle();

    // Stream must be created inside the thread
    thread::spawn(move || {
        let speaker = match SpeakerInput::new() {
            Ok(s) => s,
            Err(e) => {
                handle_capture_error(&running, "Speaker", &e);
                return;
            }
        };

        let _stream = match speaker.stream() {
            Ok(s) => s,
            Err(e) => {
                handle_stream_error(&running, "Speaker", &e);
                return;
            }
        };

        // Keep stream alive until stop signal
        run_capture_loop(&stop_signal, &running, "Speaker");
    });

    Ok(())
}

/// Start speaker capture stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn start_speaker_capture() -> Result<(), String> {
    Err("Speaker capture is only supported on macOS".to_string())
}

/// Stop the current speaker capture (macOS only)
///
/// # Errors
///
/// Returns an error if speaker capture is not running
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn stop_speaker_capture(state: State<AudioState>) -> Result<(), String> {
    if !state.is_speaker_running() {
        return Err("Speaker capture is not running".to_string());
    }

    state.signal_speaker_stop();
    Ok(())
}

/// Stop speaker capture stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn stop_speaker_capture() -> Result<(), String> {
    Err("Speaker capture is only supported on macOS".to_string())
}

/// Check if speaker capture is currently active (macOS only)
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn is_speaker_capturing(state: State<AudioState>) -> bool {
    state.is_speaker_running()
}

/// Speaker capture status stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn is_speaker_capturing() -> bool {
    false
}

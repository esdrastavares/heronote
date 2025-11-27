//! Tauri command handlers for audio operations
//!
//! This module contains all the Tauri-exposed commands for controlling
//! audio capture.

use std::sync::atomic::Ordering;

use futures::StreamExt;
use tauri::State;

use heronote_audio_core::{AudioDevice, AudioInput};

use crate::audio_state::AudioState;

#[cfg(debug_assertions)]
use crate::debug_state::{DebugAudioFile, DebugConfig, DebugState, FlatAudioMetrics};

#[cfg(debug_assertions)]
use std::fs::{self, File};
#[cfg(debug_assertions)]
use std::io::BufWriter;
#[cfg(debug_assertions)]
use std::path::PathBuf;

/// Create a WAV writer for audio capture
#[cfg(debug_assertions)]
fn create_wav_writer(
    output_dir: &PathBuf,
    source: &str,
    sample_rate: u32,
) -> Result<(hound::WavWriter<BufWriter<File>>, PathBuf), String> {
    // Create output directory
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create audio directory: {}", e))?;

    // Generate filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.wav", source, timestamp);
    let path = output_dir.join(&filename);

    // WAV spec: mono, 32-bit float
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let writer = hound::WavWriter::create(&path, spec)
        .map_err(|e| format!("Failed to create WAV file: {}", e))?;

    Ok((writer, path))
}

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

/// Start capturing audio from the default microphone (debug builds)
///
/// Note: MicStream contains cpal::Stream which is not Send, so we must use
/// a blocking thread with spawn_blocking for the async runtime integration.
///
/// # Errors
///
/// Returns an error if:
/// - Microphone capture is already running
/// - The microphone device cannot be accessed
#[cfg(debug_assertions)]
#[tauri::command]
pub fn start_mic_capture(
    audio_state: State<AudioState>,
    debug_state: State<DebugState>,
) -> Result<(), String> {
    use std::thread;

    if audio_state.is_mic_running() {
        return Err("Microphone capture is already running".to_string());
    }

    // Verify device exists before spawning thread
    let mic = MicInput::new().map_err(|e| e.to_string())?;
    let sample_rate = mic.sample_rate();

    // Get debug config for the thread
    let debug_config = debug_state.config();

    // Update state
    audio_state.set_mic_running(true);
    audio_state.reset_mic_stop_signal();

    // Get handles for the capture thread
    let running = audio_state.mic_running_handle();
    let stop_signal = audio_state.mic_stop_signal_handle();

    // Use blocking thread because cpal::Stream (inside MicStream) is not Send
    thread::spawn(move || {
        // Create a local tokio runtime for this thread
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create tokio runtime: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        rt.block_on(async {
            let mic = match MicInput::new() {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to create Microphone input: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let stream = match mic.stream() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to start Microphone stream: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            // Create WAV writer if debug save is enabled
            let mut wav_writer = if debug_config.enabled && debug_config.save_audio_files {
                match create_wav_writer(&debug_config.audio_output_dir, "mic", sample_rate) {
                    Ok((writer, path)) => {
                        tracing::info!(path = %path.display(), "Recording microphone audio to file");
                        Some((writer, path))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create WAV writer: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            tracing::info!("Microphone capture started");
            tokio::pin!(stream);

            // Consume the stream until stop signal
            loop {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    biased;

                    audio = stream.next() => {
                        match audio {
                            Some(samples) => {
                                if let Some((ref mut writer, _)) = wav_writer {
                                    for &sample in &samples {
                                        if let Err(e) = writer.write_sample(sample) {
                                            tracing::warn!("Failed to write sample: {}", e);
                                            break;
                                        }
                                    }
                                }
                                tracing::trace!(samples = samples.len(), "Microphone audio chunk received");
                            }
                            None => {
                                tracing::warn!("Microphone stream ended unexpectedly");
                                break;
                            }
                        }
                    }

                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
                }
            }

            // Finalize WAV file
            if let Some((writer, path)) = wav_writer {
                if let Err(e) = writer.finalize() {
                    tracing::error!("Failed to finalize WAV file: {}", e);
                } else {
                    tracing::info!(path = %path.display(), "Microphone audio file saved");
                }
            }

            running.store(false, Ordering::SeqCst);
            tracing::info!("Microphone capture stopped");
        });
    });

    Ok(())
}

/// Start capturing audio from the default microphone (release builds)
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn start_mic_capture(state: State<AudioState>) -> Result<(), String> {
    use std::thread;

    if state.is_mic_running() {
        return Err("Microphone capture is already running".to_string());
    }

    let _ = MicInput::new().map_err(|e| e.to_string())?;

    state.set_mic_running(true);
    state.reset_mic_stop_signal();

    let running = state.mic_running_handle();
    let stop_signal = state.mic_stop_signal_handle();

    // Use blocking thread because cpal::Stream (inside MicStream) is not Send
    thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create tokio runtime: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        rt.block_on(async {
            let mic = match MicInput::new() {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to create Microphone input: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let stream = match mic.stream() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to start Microphone stream: {}", e);
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            tracing::info!("Microphone capture started");
            tokio::pin!(stream);

            loop {
                if stop_signal.load(Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    biased;

                    audio = stream.next() => {
                        match audio {
                            Some(samples) => {
                                tracing::trace!(samples = samples.len(), "Microphone audio chunk received");
                            }
                            None => {
                                tracing::warn!("Microphone stream ended unexpectedly");
                                break;
                            }
                        }
                    }

                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
                }
            }

            running.store(false, Ordering::SeqCst);
            tracing::info!("Microphone capture stopped");
        });
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
#[cfg(all(target_os = "macos", debug_assertions))]
#[tauri::command]
pub fn start_speaker_capture(
    audio_state: State<AudioState>,
    debug_state: State<DebugState>,
) -> Result<(), String> {
    if audio_state.is_speaker_running() {
        return Err("Speaker capture is already running".to_string());
    }

    // Verify we can create speaker input before spawning task
    let speaker = SpeakerInput::new().map_err(|e| e.to_string())?;
    let sample_rate = speaker.sample_rate();

    // Get debug config for the async task
    let debug_config = debug_state.config();

    // Update state
    audio_state.set_speaker_running(true);
    audio_state.reset_speaker_stop_signal();

    // Get handles for the capture task
    let running = audio_state.speaker_running_handle();
    let stop_signal = audio_state.speaker_stop_signal_handle();

    // Spawn async task to consume the stream
    tauri::async_runtime::spawn(async move {
        let speaker = match SpeakerInput::new() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create Speaker input: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        let stream = match speaker.stream() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to start Speaker stream: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        // Create WAV writer if debug save is enabled
        let mut wav_writer = if debug_config.enabled && debug_config.save_audio_files {
            match create_wav_writer(&debug_config.audio_output_dir, "speaker", sample_rate) {
                Ok((writer, path)) => {
                    tracing::info!(path = %path.display(), "Recording speaker audio to file");
                    Some((writer, path))
                }
                Err(e) => {
                    tracing::warn!("Failed to create WAV writer: {}", e);
                    None
                }
            }
        } else {
            None
        };

        tracing::info!("Speaker capture started");
        tokio::pin!(stream);

        // Consume the stream until stop signal
        loop {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }

            tokio::select! {
                biased;

                audio = stream.next() => {
                    match audio {
                        Some(samples) => {
                            if let Some((ref mut writer, _)) = wav_writer {
                                for &sample in &samples {
                                    if let Err(e) = writer.write_sample(sample) {
                                        tracing::warn!("Failed to write sample: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        None => {
                            tracing::warn!("Speaker stream ended unexpectedly");
                            break;
                        }
                    }
                }

                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
            }
        }

        // Finalize WAV file
        if let Some((writer, path)) = wav_writer {
            if let Err(e) = writer.finalize() {
                tracing::error!("Failed to finalize WAV file: {}", e);
            } else {
                tracing::info!(path = %path.display(), "Speaker audio file saved");
            }
        }

        running.store(false, Ordering::SeqCst);
        tracing::info!("Speaker capture stopped");
    });

    Ok(())
}

/// Start capturing system audio output (macOS release builds)
#[cfg(all(target_os = "macos", not(debug_assertions)))]
#[tauri::command]
pub fn start_speaker_capture(state: State<AudioState>) -> Result<(), String> {
    if state.is_speaker_running() {
        return Err("Speaker capture is already running".to_string());
    }

    let _ = SpeakerInput::new().map_err(|e| e.to_string())?;

    state.set_speaker_running(true);
    state.reset_speaker_stop_signal();

    let running = state.speaker_running_handle();
    let stop_signal = state.speaker_stop_signal_handle();

    tauri::async_runtime::spawn(async move {
        let speaker = match SpeakerInput::new() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create Speaker input: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        let stream = match speaker.stream() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to start Speaker stream: {}", e);
                running.store(false, Ordering::SeqCst);
                return;
            }
        };

        tracing::info!("Speaker capture started");
        tokio::pin!(stream);

        loop {
            if stop_signal.load(Ordering::SeqCst) {
                break;
            }

            tokio::select! {
                biased;

                audio = stream.next() => {
                    match audio {
                        Some(samples) => {
                            tracing::trace!(samples = samples.len(), "Speaker audio chunk received");
                        }
                        None => {
                            tracing::warn!("Speaker stream ended unexpectedly");
                            break;
                        }
                    }
                }

                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {}
            }
        }

        running.store(false, Ordering::SeqCst);
        tracing::info!("Speaker capture stopped");
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

// ============================================================================
// Screen Recording Permission commands (macOS only)
// ============================================================================

/// Check if screen recording permission is granted (required for system audio capture)
/// Note: CGPreflightScreenCaptureAccess checks screen capture, not audio capture.
/// The Process Tap API uses a different permission system that shows its own dialog.
/// We return true here and let the actual capture attempt determine permissions.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn check_screen_recording_permission() -> bool {
    // The Process Tap API handles its own permission requests
    // CGPreflightScreenCaptureAccess is for screen video, not audio
    true
}

/// Check screen recording permission stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn check_screen_recording_permission() -> bool {
    true // Not required on other platforms
}

/// Request screen recording permission (opens system dialog on macOS)
/// Note: The Process Tap API shows its own permission dialog when needed.
#[cfg(target_os = "macos")]
#[tauri::command]
pub fn request_screen_recording_permission() -> bool {
    // The Process Tap API handles its own permission requests
    true
}

/// Request screen recording permission stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub fn request_screen_recording_permission() -> bool {
    true // Not required on other platforms
}

/// Open System Settings to Screen Recording privacy section (macOS only)
#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn open_screen_recording_settings(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;

    // Open System Settings to Privacy & Security > Screen Recording
    app.shell()
        .command("open")
        .args(["x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"])
        .spawn()
        .map_err(|e| format!("Failed to open settings: {}", e))?;

    Ok(())
}

/// Open screen recording settings stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn open_screen_recording_settings() -> Result<(), String> {
    Err("Screen recording settings are only available on macOS".to_string())
}

// ============================================================================
// Debug commands (only available in debug builds)
// ============================================================================

/// Check if debug mode is available (only in dev builds)
#[tauri::command]
pub fn is_debug_available() -> bool {
    cfg!(debug_assertions)
}

/// Toggle debug mode on/off
#[cfg(debug_assertions)]
#[tauri::command]
pub fn toggle_debug_mode(state: State<DebugState>, enabled: bool) -> Result<bool, String> {
    state.set_enabled(enabled);
    tracing::info!(enabled, "Debug mode toggled");
    Ok(enabled)
}

/// Toggle debug mode stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn toggle_debug_mode(_enabled: bool) -> Result<bool, String> {
    Err("Debug mode not available in release builds".to_string())
}

/// Get current debug configuration
#[cfg(debug_assertions)]
#[tauri::command]
pub fn get_debug_config(state: State<DebugState>) -> DebugConfig {
    state.config()
}

/// Get debug config stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn get_debug_config() -> Result<(), String> {
    Err("Debug mode not available in release builds".to_string())
}

/// Get current debug metrics
#[cfg(debug_assertions)]
#[tauri::command]
pub fn get_debug_metrics(
    debug_state: State<DebugState>,
    audio_state: State<AudioState>,
) -> FlatAudioMetrics {
    // Update capture status before getting metrics
    debug_state.update_metrics(|metrics| {
        metrics.mic.capturing = audio_state.is_mic_running();

        #[cfg(target_os = "macos")]
        {
            metrics.speaker.capturing = audio_state.is_speaker_running();
        }
    });

    debug_state.flat_metrics()
}

/// Get debug metrics stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn get_debug_metrics() -> Result<(), String> {
    Err("Debug mode not available in release builds".to_string())
}

/// List all debug audio files by scanning the output directory
#[cfg(debug_assertions)]
#[tauri::command]
pub fn list_debug_files(state: State<DebugState>) -> Vec<DebugAudioFile> {
    let config = state.config();
    let dir = &config.audio_output_dir;

    // If directory doesn't exist, return empty list
    if !dir.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();

    // Scan directory for .wav files
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Only process .wav files
            if path.extension().map_or(false, |ext| ext == "wav") {
                if let Some(file_info) = parse_wav_file_info(&path) {
                    files.push(file_info);
                }
            }
        }
    }

    // Sort by creation time (newest first)
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    files
}

/// Parse WAV file metadata
#[cfg(debug_assertions)]
fn parse_wav_file_info(path: &std::path::Path) -> Option<DebugAudioFile> {
    use crate::debug_state::AudioSource;

    let filename = path.file_name()?.to_str()?;

    // Parse source from filename (mic_*.wav or speaker_*.wav)
    let source = if filename.starts_with("mic_") {
        AudioSource::Mic
    } else if filename.starts_with("speaker_") {
        AudioSource::Speaker
    } else {
        return None; // Unknown file format
    };

    // Get file metadata
    let metadata = std::fs::metadata(path).ok()?;
    let size_bytes = metadata.len();

    // Get creation time
    let created_at = metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        .flatten()
        .unwrap_or_else(chrono::Utc::now);

    // Read WAV header for sample rate and duration
    let (sample_rate, duration_secs) = match hound::WavReader::open(path) {
        Ok(reader) => {
            let spec = reader.spec();
            let sample_rate = spec.sample_rate;
            let num_samples = reader.len() as f32;
            let duration = num_samples / sample_rate as f32;
            (sample_rate, duration)
        }
        Err(_) => (0, 0.0),
    };

    Some(DebugAudioFile {
        path: path.to_path_buf(),
        source,
        created_at,
        duration_secs,
        sample_rate,
        size_bytes,
    })
}

/// List debug files stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn list_debug_files() -> Result<(), String> {
    Err("Debug mode not available in release builds".to_string())
}

/// Get debug audio output directory
#[cfg(debug_assertions)]
#[tauri::command]
pub fn get_debug_audio_dir(state: State<DebugState>) -> String {
    state.config().audio_output_dir.display().to_string()
}

/// Get debug audio dir stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn get_debug_audio_dir() -> Result<(), String> {
    Err("Debug mode not available in release builds".to_string())
}

/// Reset debug counters
#[cfg(debug_assertions)]
#[tauri::command]
pub fn reset_debug_counters(state: State<DebugState>) {
    state.reset_counters();
    tracing::info!("Debug counters reset");
}

/// Reset debug counters stub for release builds
#[cfg(not(debug_assertions))]
#[tauri::command]
pub fn reset_debug_counters() -> Result<(), String> {
    Err("Debug mode not available in release builds".to_string())
}

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ============================================================================
// Constants
// ============================================================================

/** Maximum number of log entries to keep in memory */
export const MAX_LOG_ENTRIES = 100;

/** Metrics polling interval in milliseconds */
export const METRICS_POLL_INTERVAL_MS = 500;

/** Tauri command names */
export const COMMANDS = {
  IS_DEBUG_AVAILABLE: "is_debug_available",
  TOGGLE_DEBUG_MODE: "toggle_debug_mode",
  GET_DEBUG_CONFIG: "get_debug_config",
  GET_DEBUG_METRICS: "get_debug_metrics",
  LIST_DEBUG_FILES: "list_debug_files",
  RESET_DEBUG_COUNTERS: "reset_debug_counters",
  CHECK_SCREEN_RECORDING: "check_screen_recording_permission",
  REQUEST_SCREEN_RECORDING: "request_screen_recording_permission",
  OPEN_SCREEN_RECORDING_SETTINGS: "open_screen_recording_settings",
} as const;

/** Tauri event names */
export const EVENTS = {
  METRICS: "debug:metrics",
  LOG: "debug:log",
  FILE_SAVED: "debug:file-saved",
} as const;

// ============================================================================
// Types
// ============================================================================

export interface DebugConfig {
  enabled: boolean;
  save_audio_files: boolean;
  log_audio_buffers: boolean;
  log_performance: boolean;
  audio_output_dir: string;
}

export interface AudioMetrics {
  mic_sample_rate: number;
  speaker_sample_rate: number;
  mic_buffer_usage_percent: number;
  speaker_buffer_usage_percent: number;
  mic_samples_processed: number;
  speaker_samples_processed: number;
  mic_samples_dropped: number;
  speaker_samples_dropped: number;
  mic_latency_ms: number;
  speaker_latency_ms: number;
  mic_device_name: string | null;
  speaker_device_name: string | null;
  mic_capturing: boolean;
  speaker_capturing: boolean;
  last_update: string;
}

/** Extracted source metrics for reusable display */
export interface SourceMetrics {
  sampleRate: number;
  bufferUsagePercent: number;
  samplesProcessed: number;
  samplesDropped: number;
  latencyMs: number;
  deviceName: string | null;
  capturing: boolean;
}

export interface DebugAudioFile {
  path: string;
  source: "mic" | "speaker";
  created_at: string;
  duration_secs: number;
  sample_rate: number;
  size_bytes: number;
}

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  timestamp: string;
  level: LogLevel;
  message: string;
}

// ============================================================================
// Helper Functions
// ============================================================================

/** Extract mic metrics from flat audio metrics */
export function extractMicMetrics(metrics: AudioMetrics): SourceMetrics {
  return {
    sampleRate: metrics.mic_sample_rate,
    bufferUsagePercent: metrics.mic_buffer_usage_percent,
    samplesProcessed: metrics.mic_samples_processed,
    samplesDropped: metrics.mic_samples_dropped,
    latencyMs: metrics.mic_latency_ms,
    deviceName: metrics.mic_device_name,
    capturing: metrics.mic_capturing,
  };
}

/** Extract speaker metrics from flat audio metrics */
export function extractSpeakerMetrics(metrics: AudioMetrics): SourceMetrics {
  return {
    sampleRate: metrics.speaker_sample_rate,
    bufferUsagePercent: metrics.speaker_buffer_usage_percent,
    samplesProcessed: metrics.speaker_samples_processed,
    samplesDropped: metrics.speaker_samples_dropped,
    latencyMs: metrics.speaker_latency_ms,
    deviceName: metrics.speaker_device_name,
    capturing: metrics.speaker_capturing,
  };
}

// ============================================================================
// Store
// ============================================================================

interface DebugStore {
  // State
  isAvailable: boolean;
  isEnabled: boolean;
  config: DebugConfig | null;
  metrics: AudioMetrics | null;
  files: DebugAudioFile[];
  logs: LogEntry[];
  isLoading: boolean;
  error: string | null;
  hasScreenRecordingPermission: boolean | null;

  // Actions
  checkAvailability: () => Promise<void>;
  toggleDebug: (enabled: boolean) => Promise<void>;
  fetchMetrics: () => Promise<void>;
  fetchFiles: () => Promise<void>;
  fetchConfig: () => Promise<void>;
  resetCounters: () => Promise<void>;
  addLog: (entry: LogEntry) => void;
  clearLogs: () => void;
  clearError: () => void;
  setupEventListeners: () => Promise<UnlistenFn[]>;
  checkScreenRecordingPermission: () => Promise<boolean>;
  requestScreenRecordingPermission: () => Promise<boolean>;
  openScreenRecordingSettings: () => Promise<void>;
}

export const useDebugStore = create<DebugStore>((set, get) => ({
  // Initial state
  isAvailable: false,
  isEnabled: false,
  config: null,
  metrics: null,
  files: [],
  logs: [],
  isLoading: false,
  error: null,
  hasScreenRecordingPermission: null,

  checkAvailability: async () => {
    try {
      const available = await invoke<boolean>(COMMANDS.IS_DEBUG_AVAILABLE);
      set({ isAvailable: available });
    } catch {
      set({ isAvailable: false });
    }
  },

  toggleDebug: async (enabled: boolean) => {
    set({ isLoading: true, error: null });
    try {
      await invoke(COMMANDS.TOGGLE_DEBUG_MODE, { enabled });
      set({ isEnabled: enabled });
      if (enabled) {
        // Fetch initial data when enabling debug mode
        await Promise.all([
          get().fetchConfig(),
          get().fetchMetrics(),
          get().fetchFiles(),
        ]);
      }
    } catch (e) {
      set({ error: `Failed to toggle debug: ${e}` });
    } finally {
      set({ isLoading: false });
    }
  },

  fetchMetrics: async () => {
    try {
      const metrics = await invoke<AudioMetrics>(COMMANDS.GET_DEBUG_METRICS);
      set({ metrics });
    } catch (e) {
      console.error("Failed to fetch metrics:", e);
    }
  },

  fetchFiles: async () => {
    try {
      const files = await invoke<DebugAudioFile[]>(COMMANDS.LIST_DEBUG_FILES);
      set({ files });
    } catch (e) {
      console.error("Failed to fetch files:", e);
    }
  },

  fetchConfig: async () => {
    try {
      const config = await invoke<DebugConfig>(COMMANDS.GET_DEBUG_CONFIG);
      set({ config });
    } catch (e) {
      console.error("Failed to fetch config:", e);
    }
  },

  resetCounters: async () => {
    try {
      await invoke(COMMANDS.RESET_DEBUG_COUNTERS);
      await get().fetchMetrics();
    } catch (e) {
      set({ error: `Failed to reset counters: ${e}` });
    }
  },

  addLog: (entry: LogEntry) => {
    set((state) => ({
      logs: [...state.logs.slice(-(MAX_LOG_ENTRIES - 1)), entry],
    }));
  },

  clearLogs: () => set({ logs: [] }),

  clearError: () => set({ error: null }),

  setupEventListeners: async () => {
    const unlisteners: UnlistenFn[] = [];

    // Listen for metrics updates
    const unlistenMetrics = await listen<AudioMetrics>(
      EVENTS.METRICS,
      (event) => {
        set({ metrics: event.payload });
      }
    );
    unlisteners.push(unlistenMetrics);

    // Listen for log entries
    const unlistenLogs = await listen<LogEntry>(EVENTS.LOG, (event) => {
      get().addLog(event.payload);
    });
    unlisteners.push(unlistenLogs);

    // Listen for file saved notifications
    const unlistenFiles = await listen<DebugAudioFile>(
      EVENTS.FILE_SAVED,
      (event) => {
        set((state) => ({
          files: [...state.files, event.payload],
        }));
      }
    );
    unlisteners.push(unlistenFiles);

    return unlisteners;
  },

  checkScreenRecordingPermission: async () => {
    try {
      const hasPermission = await invoke<boolean>(COMMANDS.CHECK_SCREEN_RECORDING);
      set({ hasScreenRecordingPermission: hasPermission });
      return hasPermission;
    } catch (e) {
      console.error("Failed to check screen recording permission:", e);
      return false;
    }
  },

  requestScreenRecordingPermission: async () => {
    try {
      const granted = await invoke<boolean>(COMMANDS.REQUEST_SCREEN_RECORDING);
      set({ hasScreenRecordingPermission: granted });
      return granted;
    } catch (e) {
      console.error("Failed to request screen recording permission:", e);
      return false;
    }
  },

  openScreenRecordingSettings: async () => {
    try {
      await invoke(COMMANDS.OPEN_SCREEN_RECORDING_SETTINGS);
    } catch (e) {
      console.error("Failed to open screen recording settings:", e);
    }
  },
}));

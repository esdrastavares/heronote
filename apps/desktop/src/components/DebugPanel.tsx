import { useEffect, useState, useCallback } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import {
  useDebugStore,
  extractMicMetrics,
  extractSpeakerMetrics,
  METRICS_POLL_INTERVAL_MS,
  type SourceMetrics,
  type DebugAudioFile,
  type LogEntry,
  type LogLevel,
} from "../stores/debug-store";

// ============================================================================
// Constants
// ============================================================================

const BUFFER_WARNING_THRESHOLD = 80;

type TabType = "metrics" | "files" | "logs";

// ============================================================================
// Styles
// ============================================================================

const colors = {
  background: "#1a1a2e",
  backgroundLight: "#252540",
  backgroundActive: "#2a2a4e",
  border: "#4a4a6a",
  borderDark: "#333",
  text: "#eee",
  textMuted: "#888",
  textDimmed: "#666",
  success: "#4CAF50",
  info: "#2196F3",
  warning: "#ff9800",
  error: "#f44336",
  errorBg: "#3d1f1f",
  warnBg: "#3d3d1f",
  infoBg: "#1f1f3d",
} as const;

const baseStyles = {
  container: (isEnabled: boolean): React.CSSProperties => ({
    position: "fixed",
    bottom: "1rem",
    right: "1rem",
    width: isEnabled ? "420px" : "auto",
    background: colors.background,
    border: `1px solid ${colors.border}`,
    borderRadius: "8px",
    boxShadow: "0 4px 12px rgba(0, 0, 0, 0.3)",
    zIndex: 9999,
    fontFamily: "monospace",
    fontSize: "12px",
    color: colors.text,
  }),
  header: (isEnabled: boolean): React.CSSProperties => ({
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "0.75rem",
    borderBottom: isEnabled ? `1px solid ${colors.border}` : "none",
    background: colors.backgroundLight,
    borderRadius: isEnabled ? "8px 8px 0 0" : "8px",
  }),
  toggle: {
    display: "flex",
    alignItems: "center",
    gap: "0.5rem",
    cursor: "pointer",
  } as React.CSSProperties,
  tabs: {
    display: "flex",
    borderBottom: `1px solid ${colors.border}`,
  } as React.CSSProperties,
  content: {
    maxHeight: "350px",
    overflow: "auto",
    padding: "0.75rem",
  } as React.CSSProperties,
  button: {
    padding: "0.25rem 0.5rem",
    background: colors.border,
    border: "none",
    borderRadius: "4px",
    color: colors.text,
    cursor: "pointer",
    fontSize: "11px",
  } as React.CSSProperties,
  errorBanner: {
    padding: "0.5rem",
    background: colors.errorBg,
    color: colors.error,
  } as React.CSSProperties,
};

// ============================================================================
// Helper Functions
// ============================================================================

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const formatDuration = (secs: number): string => {
  const mins = Math.floor(secs / 60);
  const remainingSecs = (secs % 60).toFixed(1);
  return mins > 0 ? `${mins}m ${remainingSecs}s` : `${remainingSecs}s`;
};

const formatNumber = (num: number): string => num.toLocaleString();

const getLogLevelColor = (level: LogLevel): string => {
  switch (level) {
    case "error":
      return colors.error;
    case "warn":
      return colors.warning;
    default:
      return colors.success;
  }
};

const getLogLevelBg = (level: LogLevel): string => {
  switch (level) {
    case "error":
      return colors.errorBg;
    case "warn":
      return colors.warnBg;
    default:
      return colors.infoBg;
  }
};

// ============================================================================
// Sub-Components
// ============================================================================

interface TabButtonProps {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function TabButton({ active, onClick, children }: TabButtonProps) {
  return (
    <button
      style={{
        padding: "0.5rem 1rem",
        cursor: "pointer",
        background: active ? colors.backgroundActive : "transparent",
        border: "none",
        color: active ? colors.text : colors.textMuted,
        borderBottom: active
          ? `2px solid ${colors.success}`
          : "2px solid transparent",
        fontSize: "12px",
      }}
      onClick={onClick}
    >
      {children}
    </button>
  );
}

interface MetricRowProps {
  label: string;
  value: string | number;
  status?: "normal" | "warning" | "error";
}

function MetricRow({ label, value, status = "normal" }: MetricRowProps) {
  const valueColor =
    status === "error"
      ? colors.error
      : status === "warning"
      ? colors.warning
      : colors.success;

  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        padding: "0.25rem 0",
        borderBottom: `1px solid ${colors.borderDark}`,
      }}
    >
      <span style={{ color: colors.textMuted }}>{label}:</span>
      <span style={{ color: valueColor, fontWeight: "bold" }}>{value}</span>
    </div>
  );
}

interface SourceMetricsCardProps {
  title: string;
  metrics: SourceMetrics;
  color: string;
}

function SourceMetricsCard({ title, metrics, color }: SourceMetricsCardProps) {
  return (
    <>
      <h4
        style={{
          margin: "0 0 0.5rem",
          color: colors.textMuted,
          fontSize: "13px",
        }}
      >
        <span
          style={{
            display: "inline-block",
            width: "8px",
            height: "8px",
            borderRadius: "50%",
            marginRight: "6px",
            background: metrics.capturing ? color : colors.textDimmed,
          }}
        />
        {title}
      </h4>
      <MetricRow
        label="Status"
        value={metrics.capturing ? "Capturing" : "Idle"}
        status={metrics.capturing ? "normal" : undefined}
      />
      <MetricRow label="Device" value={metrics.deviceName || "N/A"} />
      <MetricRow label="Sample Rate" value={`${metrics.sampleRate} Hz`} />
      <MetricRow
        label="Buffer Usage"
        value={`${metrics.bufferUsagePercent.toFixed(1)}%`}
        status={
          metrics.bufferUsagePercent > BUFFER_WARNING_THRESHOLD
            ? "warning"
            : "normal"
        }
      />
      <MetricRow
        label="Samples Processed"
        value={formatNumber(metrics.samplesProcessed)}
      />
      <MetricRow
        label="Samples Dropped"
        value={formatNumber(metrics.samplesDropped)}
        status={metrics.samplesDropped > 0 ? "error" : "normal"}
      />
      <MetricRow
        label="Latency"
        value={`${metrics.latencyMs.toFixed(2)} ms`}
      />
    </>
  );
}

interface MetricsTabProps {
  micMetrics: SourceMetrics;
  speakerMetrics: SourceMetrics;
  onResetCounters: () => void;
}

function MetricsTab({
  micMetrics,
  speakerMetrics,
  onResetCounters,
}: MetricsTabProps) {
  return (
    <>
      <SourceMetricsCard
        title="Microphone"
        metrics={micMetrics}
        color={colors.success}
      />

      <div style={{ marginTop: "1rem" }}>
        <SourceMetricsCard
          title="Speaker (System Audio)"
          metrics={speakerMetrics}
          color={colors.info}
        />
      </div>

      <div style={{ marginTop: "1rem" }}>
        <button style={baseStyles.button} onClick={onResetCounters}>
          Reset Counters
        </button>
      </div>
    </>
  );
}

interface FilesTabProps {
  files: DebugAudioFile[];
  audioDir: string | null;
  onRefresh: () => void;
  onOpenFile: (path: string) => void;
  onOpenDir: () => void;
}

function FilesTab({ files, audioDir, onRefresh, onOpenFile, onOpenDir }: FilesTabProps) {
  return (
    <>
      <div style={{ marginBottom: "0.75rem", display: "flex", gap: "0.5rem" }}>
        <button style={baseStyles.button} onClick={onRefresh}>
          Refresh
        </button>
        <button style={baseStyles.button} onClick={onOpenDir}>
          Open Folder
        </button>
      </div>

      {files.length === 0 ? (
        <p style={{ color: colors.textDimmed, textAlign: "center" }}>
          No debug audio files yet.
          <br />
          <small>Enable debug mode and start capture to record.</small>
          {audioDir && (
            <>
              <br />
              <small style={{ fontSize: "10px" }}>Dir: {audioDir}</small>
            </>
          )}
        </p>
      ) : (
        files.map((file) => (
          <div
            key={file.path}
            style={{
              padding: "0.5rem",
              background: colors.backgroundLight,
              borderRadius: "4px",
              marginBottom: "0.5rem",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div style={{ fontWeight: "bold", color: colors.success }}>
                {file.path.split("/").pop()}
              </div>
              <button
                style={{
                  ...baseStyles.button,
                  background: colors.success,
                  padding: "0.2rem 0.5rem",
                }}
                onClick={() => onOpenFile(file.path)}
              >
                ▶ Play
              </button>
            </div>
            <div style={{ color: colors.textMuted, marginTop: "0.25rem" }}>
              Source: {file.source} | Duration: {formatDuration(file.duration_secs)}
            </div>
            <div style={{ color: colors.textDimmed }}>
              {file.sample_rate} Hz | {formatBytes(file.size_bytes)}
            </div>
          </div>
        ))
      )}
    </>
  );
}

interface LogsTabProps {
  logs: LogEntry[];
  onClearLogs: () => void;
}

function LogsTab({ logs, onClearLogs }: LogsTabProps) {
  return (
    <>
      <div style={{ marginBottom: "0.5rem" }}>
        <button style={baseStyles.button} onClick={onClearLogs}>
          Clear Logs
        </button>
      </div>
      {logs.length === 0 ? (
        <p style={{ color: colors.textDimmed, textAlign: "center" }}>
          No logs yet
        </p>
      ) : (
        logs.map((log) => (
          <div
            key={`${log.timestamp}-${log.message}`}
            style={{
              padding: "0.25rem 0.5rem",
              background: getLogLevelBg(log.level),
              borderRadius: "2px",
              marginBottom: "2px",
              fontSize: "11px",
            }}
          >
            <span style={{ color: colors.textDimmed }}>
              {new Date(log.timestamp).toLocaleTimeString()}
            </span>{" "}
            <span style={{ color: getLogLevelColor(log.level) }}>
              [{log.level.toUpperCase()}]
            </span>{" "}
            {log.message}
          </div>
        ))
      )}
    </>
  );
}

// ============================================================================
// Main Component
// ============================================================================

export function DebugPanel() {
  const {
    isAvailable,
    isEnabled,
    metrics,
    files,
    logs,
    config,
    isLoading,
    error,
    hasScreenRecordingPermission,
    checkAvailability,
    toggleDebug,
    fetchMetrics,
    fetchFiles,
    resetCounters,
    clearLogs,
    setupEventListeners,
    checkScreenRecordingPermission,
    openScreenRecordingSettings,
  } = useDebugStore();

  const [activeTab, setActiveTab] = useState<TabType>("metrics");

  const handleOpenFile = useCallback(async (path: string) => {
    try {
      await openPath(path);
    } catch (e) {
      console.error("Failed to open file:", e);
    }
  }, []);

  const handleOpenDir = useCallback(async () => {
    if (config?.audio_output_dir) {
      try {
        await openPath(config.audio_output_dir);
      } catch (e) {
        console.error("Failed to open directory:", e);
      }
    }
  }, [config]);

  // Check availability and permissions on mount
  useEffect(() => {
    checkAvailability();
    checkScreenRecordingPermission();
  }, [checkAvailability, checkScreenRecordingPermission]);

  // Setup listeners and polling when enabled
  useEffect(() => {
    if (!isEnabled) return;

    let unlisteners: (() => void)[] = [];
    let metricsInterval: number;

    const setup = async () => {
      unlisteners = await setupEventListeners();
      metricsInterval = window.setInterval(fetchMetrics, METRICS_POLL_INTERVAL_MS);
    };

    setup();

    return () => {
      unlisteners.forEach((ul) => ul());
      if (metricsInterval) clearInterval(metricsInterval);
    };
  }, [isEnabled, setupEventListeners, fetchMetrics]);

  // Refresh files when switching to files tab
  useEffect(() => {
    if (isEnabled && activeTab === "files") {
      fetchFiles();
    }
  }, [isEnabled, activeTab, fetchFiles]);

  const handleToggle = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      toggleDebug(e.target.checked);
    },
    [toggleDebug]
  );

  // Hide in production builds
  if (!isAvailable) {
    return null;
  }

  const micMetrics = metrics ? extractMicMetrics(metrics) : null;
  const speakerMetrics = metrics ? extractSpeakerMetrics(metrics) : null;

  return (
    <div style={baseStyles.container(isEnabled)}>
      {/* Header */}
      <div style={baseStyles.header(isEnabled)}>
        <label style={baseStyles.toggle}>
          <input
            type="checkbox"
            checked={isEnabled}
            onChange={handleToggle}
            disabled={isLoading}
          />
          <span style={{ color: isEnabled ? colors.success : colors.textMuted }}>
            Debug Mode
          </span>
        </label>
        {isEnabled && config && (
          <span style={{ color: colors.textDimmed, fontSize: "10px" }}>
            {config.audio_output_dir.split("/").pop()}
          </span>
        )}
      </div>

      {/* Error Banner */}
      {error && <div style={baseStyles.errorBanner}>{error}</div>}

      {/* Screen Recording Permission Banner */}
      {hasScreenRecordingPermission === false && (
        <div
          style={{
            padding: "0.5rem 0.75rem",
            background: colors.warnBg,
            borderBottom: `1px solid ${colors.warning}`,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "0.5rem",
          }}
        >
          <span style={{ color: colors.warning, fontSize: "11px" }}>
            Screen Recording permission required for speaker capture
          </span>
          <button
            style={{
              ...baseStyles.button,
              background: colors.warning,
              color: "#000",
              fontWeight: "bold",
            }}
            onClick={openScreenRecordingSettings}
          >
            Grant Access
          </button>
        </div>
      )}

      {/* Content */}
      {isEnabled && (
        <>
          {/* Tabs */}
          <div style={baseStyles.tabs}>
            <TabButton
              active={activeTab === "metrics"}
              onClick={() => setActiveTab("metrics")}
            >
              Metrics
            </TabButton>
            <TabButton
              active={activeTab === "files"}
              onClick={() => setActiveTab("files")}
            >
              Files ({files.length})
            </TabButton>
            <TabButton
              active={activeTab === "logs"}
              onClick={() => setActiveTab("logs")}
            >
              Logs ({logs.length})
            </TabButton>
          </div>

          {/* Tab Content */}
          <div style={baseStyles.content}>
            {activeTab === "metrics" && micMetrics && speakerMetrics && (
              <MetricsTab
                micMetrics={micMetrics}
                speakerMetrics={speakerMetrics}
                onResetCounters={resetCounters}
              />
            )}
            {activeTab === "files" && (
              <FilesTab
                files={files}
                audioDir={config?.audio_output_dir ?? null}
                onRefresh={fetchFiles}
                onOpenFile={handleOpenFile}
                onOpenDir={handleOpenDir}
              />
            )}
            {activeTab === "logs" && (
              <LogsTab logs={logs} onClearLogs={clearLogs} />
            )}
          </div>
        </>
      )}
    </div>
  );
}

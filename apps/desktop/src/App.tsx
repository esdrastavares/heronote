import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DebugPanel } from "./components/DebugPanel";

interface AudioDevice {
  name: string;
  device_type: "Input" | "Output";
  is_default: boolean;
}

function App() {
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [isMicCapturing, setIsMicCapturing] = useState(false);
  const [isSpeakerCapturing, setIsSpeakerCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadDevices();
  }, []);

  async function loadDevices() {
    try {
      const deviceList = await invoke<AudioDevice[]>("list_audio_devices");
      setDevices(deviceList);
      setError(null);
    } catch (e) {
      setError(`Failed to load devices: ${e}`);
    }
  }

  async function toggleMicCapture() {
    try {
      if (isMicCapturing) {
        await invoke("stop_mic_capture");
        setIsMicCapturing(false);
      } else {
        await invoke("start_mic_capture");
        setIsMicCapturing(true);
      }
      setError(null);
    } catch (e) {
      setError(`Mic capture error: ${e}`);
    }
  }

  async function toggleSpeakerCapture() {
    try {
      if (isSpeakerCapturing) {
        await invoke("stop_speaker_capture");
        setIsSpeakerCapturing(false);
      } else {
        await invoke("start_speaker_capture");
        setIsSpeakerCapturing(true);
      }
      setError(null);
    } catch (e) {
      setError(`Speaker capture error: ${e}`);
    }
  }

  const inputDevices = devices.filter((d) => d.device_type === "Input");
  const outputDevices = devices.filter((d) => d.device_type === "Output");

  return (
    <div style={{ padding: "2rem", maxWidth: "600px", margin: "0 auto" }}>
      <h1 style={{ marginBottom: "1.5rem", color: "#fff" }}>Heronote</h1>

      {error && (
        <div
          style={{
            padding: "1rem",
            background: "#ff4444",
            borderRadius: "8px",
            marginBottom: "1rem",
          }}
        >
          {error}
        </div>
      )}

      <section style={{ marginBottom: "2rem" }}>
        <h2 style={{ marginBottom: "1rem", fontSize: "1.2rem" }}>
          Audio Capture
        </h2>

        <div style={{ display: "flex", gap: "1rem", marginBottom: "1rem" }}>
          <button
            onClick={toggleMicCapture}
            style={{
              padding: "0.75rem 1.5rem",
              borderRadius: "8px",
              border: "none",
              background: isMicCapturing ? "#ff4444" : "#4CAF50",
              color: "white",
              cursor: "pointer",
              fontSize: "1rem",
            }}
          >
            {isMicCapturing ? "Stop Mic" : "Start Mic"}
          </button>

          <button
            onClick={toggleSpeakerCapture}
            style={{
              padding: "0.75rem 1.5rem",
              borderRadius: "8px",
              border: "none",
              background: isSpeakerCapturing ? "#ff4444" : "#2196F3",
              color: "white",
              cursor: "pointer",
              fontSize: "1rem",
            }}
          >
            {isSpeakerCapturing ? "Stop Speaker" : "Start Speaker"}
          </button>
        </div>

        {(isMicCapturing || isSpeakerCapturing) && (
          <div
            style={{
              padding: "1rem",
              background: "#2a2a4e",
              borderRadius: "8px",
            }}
          >
            <p>
              Recording:{" "}
              {[
                isMicCapturing && "Microphone",
                isSpeakerCapturing && "System Audio",
              ]
                .filter(Boolean)
                .join(", ")}
            </p>
          </div>
        )}
      </section>

      <section>
        <h2 style={{ marginBottom: "1rem", fontSize: "1.2rem" }}>
          Audio Devices
        </h2>

        <div style={{ marginBottom: "1rem" }}>
          <h3 style={{ fontSize: "1rem", marginBottom: "0.5rem", opacity: 0.7 }}>
            Input Devices
          </h3>
          {inputDevices.length === 0 ? (
            <p style={{ opacity: 0.5 }}>No input devices found</p>
          ) : (
            <ul style={{ listStyle: "none" }}>
              {inputDevices.map((device) => (
                <li
                  key={device.name}
                  style={{
                    padding: "0.5rem",
                    background: device.is_default ? "#3a3a5e" : "transparent",
                    borderRadius: "4px",
                    marginBottom: "0.25rem",
                  }}
                >
                  {device.name} {device.is_default && "(Default)"}
                </li>
              ))}
            </ul>
          )}
        </div>

        <div>
          <h3 style={{ fontSize: "1rem", marginBottom: "0.5rem", opacity: 0.7 }}>
            Output Devices
          </h3>
          {outputDevices.length === 0 ? (
            <p style={{ opacity: 0.5 }}>No output devices found</p>
          ) : (
            <ul style={{ listStyle: "none" }}>
              {outputDevices.map((device) => (
                <li
                  key={device.name}
                  style={{
                    padding: "0.5rem",
                    background: device.is_default ? "#3a3a5e" : "transparent",
                    borderRadius: "4px",
                    marginBottom: "0.25rem",
                  }}
                >
                  {device.name} {device.is_default && "(Default)"}
                </li>
              ))}
            </ul>
          )}
        </div>

        <button
          onClick={loadDevices}
          style={{
            marginTop: "1rem",
            padding: "0.5rem 1rem",
            borderRadius: "4px",
            border: "1px solid #555",
            background: "transparent",
            color: "#eee",
            cursor: "pointer",
          }}
        >
          Refresh Devices
        </button>
      </section>

      {/* Debug Panel - only visible in development builds */}
      <DebugPanel />
    </div>
  );
}

export default App;

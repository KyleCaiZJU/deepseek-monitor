import { useState, useEffect } from "react";
import { useAppStore } from "../store";
import { saveSettings as saveSettingsApi, setAutostart, isAutostartEnabled } from "../api";

export default function Settings({ onClose }: { onClose: () => void }) {
  const { settings, setSettings } = useAppStore();
  const [apiKey, setApiKey] = useState(settings.api_key);
  const [intervalMin, setIntervalMin] = useState(settings.interval_min);
  const [downloadsDir, setDownloadsDir] = useState(settings.downloads_dir);
  const [autostart, setAutostartState] = useState(false);

  useEffect(() => {
    isAutostartEnabled().then(setAutostartState).catch(() => {});
  }, []);

  async function handleSave() {
    const newSettings = {
      api_key: apiKey,
      interval_min: intervalMin,
      downloads_dir: downloadsDir,
    };
    await saveSettingsApi(newSettings);
    setSettings(newSettings);

    await setAutostart(autostart);
    onClose();
  }

  function handleAutostartToggle() {
    setAutostartState((prev) => !prev);
  }

  return (
    <div className="settings-overlay">
      <h2>Settings</h2>

      <div className="settings-field">
        <label>API Key</label>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-..."
        />
      </div>

      <div className="settings-field">
        <label>Refresh Interval (minutes)</label>
        <input
          type="number"
          value={intervalMin}
          onChange={(e) => setIntervalMin(Number(e.target.value))}
          min={1}
          max={60}
        />
      </div>

      <div className="settings-field">
        <label>Downloads Directory</label>
        <input
          type="text"
          value={downloadsDir}
          onChange={(e) => setDownloadsDir(e.target.value)}
        />
      </div>

      <div className="settings-toggle">
        <span>Launch at Startup</span>
        <button
          className={`toggle-switch ${autostart ? "on" : ""}`}
          onClick={handleAutostartToggle}
        />
      </div>

      <div className="settings-actions">
        <button className="btn" onClick={onClose}>
          Cancel
        </button>
        <button className="btn primary" onClick={handleSave}>
          Save
        </button>
      </div>
    </div>
  );
}

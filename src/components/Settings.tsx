import { useState, useEffect } from "react";
import { useAppStore } from "../store";
import { saveSettings as saveSettingsApi, setAutostart, isAutostartEnabled } from "../api";

export default function Settings({ onClose }: { onClose: () => void }) {
  const { settings, setSettings } = useAppStore();
  const [apiKey, setApiKey] = useState(settings.api_key);
  const [platformToken, setPlatformToken] = useState(settings.platform_token);
  const [intervalMin, setIntervalMin] = useState(settings.interval_min);
  const [autostart, setAutostartState] = useState(false);

  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isAutostartEnabled().then(setAutostartState).catch(() => {});
  }, []);

  async function handleSave() {
    setError(null);
    const newSettings = {
      api_key: apiKey,
      platform_token: platformToken,
      interval_min: intervalMin,
      downloads_dir: settings.downloads_dir,
    };
    try {
      await saveSettingsApi(newSettings);
      setSettings(newSettings);
      await setAutostart(autostart);
      onClose();
    } catch (e) {
      console.error("Failed to save settings:", e);
      setError(String(e));
    }
  }

  function handleAutostartToggle() {
    setAutostartState((prev) => !prev);
  }

  return (
    <div className="settings-overlay">
      <h2>{'设置'}</h2>

      <div className="settings-field">
        <label>
          {'API Key'}
          <span className="field-hint">{'用于查询账户总余额（非分 Key 追踪）'}</span>
        </label>
        <input
          type="password"
          value={apiKey}
          onChange={(e) => setApiKey(e.target.value)}
          placeholder="sk-..."
        />
      </div>

      <div className="settings-field">
        <label>{'平台 Token'}</label>
        <input
          type="password"
          value={platformToken}
          onChange={(e) => setPlatformToken(e.target.value)}
          placeholder="eyJ..."
        />
      </div>

      <div className="settings-field">
        <label>{'刷新间隔（分钟）'}</label>
        <input
          type="number"
          value={intervalMin}
          onChange={(e) => setIntervalMin(Number(e.target.value))}
          min={1}
          max={60}
        />
      </div>

      <div className="settings-toggle">
        <span>{'开机自启'}</span>
        <button
          className={`toggle-switch ${autostart ? "on" : ""}`}
          onClick={handleAutostartToggle}
        />
      </div>

      {error && <div className="settings-error">{error}</div>}

      <div className="settings-actions">
        <button className="btn" onClick={onClose}>
          {'取消'}
        </button>
        <button className="btn primary" onClick={handleSave}>
          {'保存'}
        </button>
      </div>
    </div>
  );
}

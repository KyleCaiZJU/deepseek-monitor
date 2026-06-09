import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "./store";
import type { Dashboard } from "./store";
import {
  getDashboard,
  getSettings,
  refresh as refreshCmd,
  setAutostart,
  isAutostartEnabled,
  fetchPlatformUsage,
} from "./api";
import BalanceCard from "./components/BalanceCard";
import CostCards from "./components/CostCards";
import TrendChart from "./components/TrendChart";
import ModelUsage from "./components/ModelUsage";
import CacheHitPanel from "./components/CacheHitPanel";
import Settings from "./components/Settings";
import "./styles.css";

export default function App() {
  const {
    dashboard,
    showSettings,
    setDashboard,
    setShowSettings,
    setLoading,
    setSettings,
  } = useAppStore();

  useEffect(() => {
    // Ensure window stays off the taskbar at runtime (belt + suspenders with config).
    getCurrentWindow().setSkipTaskbar(true);

    loadDashboard();

    // C12: Load settings from backend into zustand on startup
    // Issue 2: After loading settings, trigger platform data fetch + reload dashboard
    getSettings().then(async (s) => {
      setSettings(s);
      if (s.platform_token) {
        try {
          await fetchPlatformUsage();
          await loadDashboard();
        } catch (e) {
          console.error("Initial platform fetch failed:", e);
        }
      }
    }).catch(console.error);

    const unlisteners: (() => void)[] = [];

    listen<Dashboard>("dashboard-updated", (event) => {
      setDashboard(event.payload);
    }).then((fn) => unlisteners.push(fn));

    listen("menu-refresh", () => {
      handleRefresh();
    }).then((fn) => unlisteners.push(fn));

    listen("menu-settings", () => {
      setShowSettings(true);
    }).then((fn) => unlisteners.push(fn));

    listen("menu-autostart", async () => {
      try {
        const current = await isAutostartEnabled();
        await setAutostart(!current);
      } catch (e) {
        console.error("Failed to toggle autostart:", e);
      }
    }).then((fn) => unlisteners.push(fn));

    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        getCurrentWindow().hide();
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && useAppStore.getState().showSettings) {
        setShowSettings(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      unlisteners.forEach((fn) => fn());
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  async function loadDashboard() {
    setLoading(true);
    try {
      const d = await getDashboard();
      setDashboard(d);
    } catch (e) {
      console.error("Failed to load dashboard:", e);
    }
    setLoading(false);
  }

  async function handleRefresh() {
    setLoading(true);
    try {
      const d = await refreshCmd();
      setDashboard(d);
    } catch {
      try {
        const d = await getDashboard();
        setDashboard(d);
      } catch (_) {}
    }
    setLoading(false);
  }

  return (
    <div className="app">
      <div className="titlebar">
        <div className="titlebar-left">
          <span
            className={`titlebar-dot ${dashboard?.available ? "online" : "offline"}`}
          />
          <span className="titlebar-label">
            {dashboard?.available ? 'DeepSeek 监控' : '离线'}
          </span>
        </div>
        <div className="titlebar-actions">
          <button onClick={handleRefresh} title={'刷新'} className="btn-icon">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <polyline points="23 4 23 10 17 10"/>
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"/>
            </svg>
          </button>
          <button onClick={() => setShowSettings(true)} title={'设置'} className="btn-icon">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <circle cx="12" cy="12" r="3"/>
              <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            </svg>
          </button>
        </div>
      </div>

      <div className="content">
        {!dashboard ? (
          <div className="empty-state">{'加载中...'}</div>
        ) : (
          <>
            <BalanceCard
              balance={dashboard.balance}
              available={dashboard.available}
            />
            <CostCards
              todayCost={dashboard.today_cost}
              monthCost={dashboard.month_cost}
            />
            <ModelUsage models={dashboard.models} />
            <CacheHitPanel dashboard={dashboard} />
            <TrendChart trend={dashboard.trend} />
          </>
        )}
      </div>

      {showSettings && (
        <Settings onClose={() => setShowSettings(false)} />
      )}
    </div>
  );
}

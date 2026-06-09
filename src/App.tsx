import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "./store";
import type { Dashboard } from "./store";
import {
  getDashboard,
  refresh as refreshCmd,
  setAutostart,
  isAutostartEnabled,
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
  } = useAppStore();

  useEffect(() => {
    // Ensure window stays off the taskbar at runtime (belt + suspenders with config).
    getCurrentWindow().setSkipTaskbar(true);

    loadDashboard();

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
            {dashboard?.available ? "DSM" : "Offline"}
          </span>
        </div>
        <div className="titlebar-actions">
          <button onClick={handleRefresh} title="Refresh">
            &#x21bb;
          </button>
          <button onClick={() => setShowSettings(true)} title="Settings">
            &#x2699;
          </button>
        </div>
      </div>

      <div className="content">
        {!dashboard ? (
          <div className="empty-state">Loading...</div>
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

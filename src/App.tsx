import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "./store";
import type { Dashboard } from "./store";
import {
  getDashboard,
  refresh as refreshCmd,
  importCsv,
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
    loadDashboard();

    const unlisteners: (() => void)[] = [];

    listen<Dashboard>("dashboard-updated", (event) => {
      setDashboard(event.payload);
    }).then((fn) => unlisteners.push(fn));

    listen("menu-refresh", () => {
      handleRefresh();
    }).then((fn) => unlisteners.push(fn));

    listen("menu-import", () => {
      handleImport();
    }).then((fn) => unlisteners.push(fn));

    listen("menu-settings", () => {
      setShowSettings(true);
    }).then((fn) => unlisteners.push(fn));

    getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) {
        getCurrentWindow().hide();
      }
    });

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && showSettings) {
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

  async function handleImport() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (selected) {
        await importCsv(selected as string);
        const d = await getDashboard();
        setDashboard(d);
      }
    } catch (e) {
      console.error("Import failed:", e);
    }
  }

  return (
    <div className="app">
      <div className="titlebar">
        <h1>DeepSeek Monitor</h1>
        <div className="titlebar-actions">
          {dashboard?.last_import_ts && (
            <span className="freshness">
              CSV: {dashboard.last_import_ts.slice(0, 16).replace("T", " ")}
            </span>
          )}
          <button onClick={handleRefresh} title="Refresh">
            &#x21bb;
          </button>
          <button onClick={handleImport} title="Import CSV">
            &#x1F4C2;
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

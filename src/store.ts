import { create } from "zustand";

export interface DayPoint {
  date: string;
  output_tokens: number;
  cache_hit_tokens: number;
  cache_miss_tokens: number;
  request_count: number;
  cost: number;
}

export interface ModelUsage {
  model: string;
  output_tokens: number;
  cache_hit_tokens: number;
  cache_miss_tokens: number;
  request_count: number;
  cost: number;
  cache_hit_rate: number;
}

export interface CacheSource {
  api_key_name: string;
  request_count: number;
  hit_tokens: number;
  miss_tokens: number;
  hit_rate: number;
}

export interface Dashboard {
  balance: number;
  available: boolean;
  today_cost: number;
  month_cost: number;
  trend: DayPoint[];
  models: ModelUsage[];
  cache_overall_rate: number;
  cache_hit_tokens: number;
  cache_miss_tokens: number;
  cache_by_model: ModelUsage[];
  cache_by_source: CacheSource[];
}

export interface Settings {
  api_key: string;
  platform_token: string;
  interval_min: number;
  downloads_dir: string;
}

interface AppState {
  dashboard: Dashboard | null;
  settings: Settings;
  showSettings: boolean;
  loading: boolean;
  setDashboard: (d: Dashboard) => void;
  setSettings: (s: Settings) => void;
  setShowSettings: (v: boolean) => void;
  setLoading: (v: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  dashboard: null,
  settings: {
    api_key: "",
    platform_token: "",
    interval_min: 5,
    downloads_dir: "",
  },
  showSettings: false,
  loading: false,
  setDashboard: (dashboard) => set({ dashboard }),
  setSettings: (settings) => set({ settings }),
  setShowSettings: (showSettings) => set({ showSettings }),
  setLoading: (loading) => set({ loading }),
}));

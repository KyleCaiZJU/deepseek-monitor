import { invoke } from "@tauri-apps/api/core";
import type { Dashboard, Settings } from "./store";

export async function getDashboard(): Promise<Dashboard> {
  return invoke("get_dashboard");
}

export async function refresh(): Promise<Dashboard> {
  return invoke("refresh");
}

export async function importCsv(path: string): Promise<{
  amount_rows: number;
  cost_rows: number;
  skipped_amount: number;
  skipped_cost: number;
}> {
  return invoke("import_csv", { path });
}

export async function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function setAutostart(on: boolean): Promise<void> {
  return invoke("set_autostart", { on });
}

export async function isAutostartEnabled(): Promise<boolean> {
  return invoke("is_autostart_enabled");
}

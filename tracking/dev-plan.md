# DeepSeek Monitor — Dev Plan

> Generated 2026-06-09 after full codebase analysis.
> Rust compiles clean. TypeScript compiles clean. No compile errors.
> All issues below are **runtime** bugs or **missing** features.

---

## Priority Legend

- **P0**: App unusable or loses data across restarts
- **P1**: Core feature broken/missing
- **P2**: Quality/safety improvement
- **P3**: Nice to have

---

## Task List

### CYCLE 01 — Core Runtime Fixes (P0/P1)

| ID | Title | Priority | Files + Lines | Problem | Fix Approach | Dependencies | RUNTIME Verify? |
|----|-------|----------|---------------|---------|--------------|--------------|-----------------|
| **C01** | Settings not persisted across restarts | **P0** | `lib.rs:42`, `commands.rs:142-151` | `save_settings` only updates in-memory `SettingsState`. Nothing ever calls `tauri-plugin-store` to read/write. API key, interval, downloads_dir are **lost on every restart**. | Use `app_handle.store("settings.json")` from `tauri_plugin_store::StoreExt`. Read on startup (`setup`), write in `save_settings`. Inject `AppHandle` into `save_settings` command. | None | **YES** — restart app, verify API key survives |
| **C02** | Timer loop captures stale default settings | **P0** | `lib.rs:118-120` | `let settings = Settings::default();` captured once at app start. Timer **never** re-reads user's saved settings. Even after C01 fix, the timer will use the empty API key and default 5min interval forever. | Read settings from `SettingsState` (or store) each tick: `let settings = app.state::<SettingsState>().get();` | C01 | **YES** — change interval to 1min, verify timer respects it |
| **C03** | Watcher always monitors default Downloads dir | **P1** | `lib.rs:68` | `Settings::default().downloads_dir` — ignores any user-configured directory. | Read `downloads_dir` from settings state (not default). Or store watcher handle and restart when settings change. | C01 | **NO** |
| **C04** | Escape key broken in Settings overlay | **P1** | `App.tsx:56-61` | `handleKeyDown` closure captures `showSettings` from initial render (`false`). When Settings is visible and user presses Escape, the stale `false` skips `setShowSettings(false)`. | Replace destructured `showSettings` with `useAppStore.getState().showSettings` inside the handler. | None | **YES** — open Settings, press Escape |
| **C05** | Tray tooltip never updates with balance | **P1** | `tray.rs:26`, `lib.rs:140-141` | Tooltip is static `"DeepSeek Monitor"`. Spec says it must dynamically show `余额 ¥xxx`. Tray handle is not stored, so no way to call `set_tooltip()`. | Store `TrayIcon` in managed state (`Arc<Mutex<Option<TrayIcon>>>`). Call `tray.set_tooltip(Some(&format!("余额 ¥{:.2}", bal)))` in timer loop. | None | **YES** — set API key, hover tray icon, see balance |
| **C06** | Autostart menu item is plain text, not checkable | **P1** | `tray.rs:11,40-42` | `MenuItemBuilder` creates static text. Can't show current autostart state. Click just emits event, doesn't toggle. | Use `tray::menu::CheckMenuItemBuilder` instead. Read current autostart state on menu build. Toggle on click. | C01 (needs stored autostart pref) | **YES** — toggle autostart, reopen menu, verify checkmark |
| **C07** | `iconAsTemplate: true` is macOS-only | **P2** | `tauri.conf.json:32` | `"iconAsTemplate": true` is a macOS template icon feature. Harmless on Windows but semantically wrong (might cause warnings or unexpected behavior with icon rendering). | Set to `false` or remove the property entirely. | None | **NO** |
| **C08** | Hardcoded backslash in path construction | **P2** | `lib.rs:172`, `commands.rs:126` | `format!("{}\\deepseek-monitor", p)` and `format!("{}\\Downloads", p)` hardcode `\\`, fragile if path already has trailing slash or on non-Windows. | Use `Path::new(&p).join("deepseek-monitor")` and `Path::new(&p).join("Downloads")`. | None | **NO** |
| **C09** | `listen("menu-autostart", ...)` not registered | **P2** | `App.tsx` (missing) | `tray.rs:42` emits `"menu-autostart"`. But `App.tsx` never listens for this event. Clicking the menu item does nothing visible. | Add `listen("menu-autostart", () => { ... })` in App.tsx, toggling autostart and showing feedback. | C06 | **YES** — click autostart in tray menu |
| **C10** | `Settings` uses `serde::Serialize` but `trayIcon` in config has unknown fields | **P3** | `tauri.conf.json:30-34` | `trayIcon` block in `tauri.conf.json` has `iconAsTemplate` and `tooltip`. In Tauri 2, `trayIcon` config is under `app.trayIcon`. The `tooltip` here duplicates the one set in code. Redundant but not harmful. | Remove redundant `"tooltip"` from config (code sets it). Or remove entire `trayIcon` block from config and set everything in code. | None | **NO** |
| **C11** | `tauri.conf.json` `"plugins": {}` — user already fixed | **P0 (was)** | `tauri.conf.json:50` | Was `"plugins": {"autostart": ...}` with invalid structure causing crash. User already fixed to `{}`. | Already fixed. | None | **YES** — app should start |

---

### CYCLE 02 — Auto-Export from DeepSeek Platform (v2 Feature)

> **Research Finding**: The DeepSeek platform (platform.deepseek.com) does **not** have a documented public REST API for usage data export. The CSV export is only available through the web console UI (a JavaScript SPA). This means programmatic export requires either:
> 1. **Reverse-engineering the internal API** (calling the same endpoints the SPA uses), or
> 2. **WebView automation** (controlling a browser to click through the UI).

| ID | Title | Priority | Description | Approach | Dependencies |
|----|-------|----------|-------------|----------|--------------|
| **A01** | Research platform.deepseek.com internal API | P0 | Use browser DevTools to capture network requests when manually exporting CSV from platform.deepseek.com. Identify: (a) auth/session mechanism, (b) usage data list endpoint, (c) CSV download endpoint, (d) required headers/tokens. | Manual research task. Output: documented API endpoints in `tracking/research/platform-api.md`. | None |
| **A02** | Implement login/cookie management | P0 | Create a module that manages platform.deepseek.com session cookies. First-time: open a WebView2 window for user to log in. Capture cookies and persist to disk (encrypted). Subsequent: load cookies, verify they're still valid, refresh if needed. | `src-tauri/src/platform_auth.rs` — uses `reqwest::cookie::Jar` + `tauri::WebviewWindow` for one-time login. Store cookies in platform store or encrypted file. | A01 |
| **A03** | Implement usage data fetching + CSV download | P0 | Based on A01 findings, call the internal API endpoints to: list available months/data, trigger CSV export, download the CSV file to the monitored Downloads directory. | `src-tauri/src/platform_export.rs` — uses `reqwest` with stored cookies. Fallback: if direct API fails, use hidden WebView2 to navigate and inject JS to trigger export. | A01, A02 |
| **A04** | Schedule automatic exports | P1 | Run auto-export on a schedule (configurable, default: daily at 02:00 UTC or when app starts if missed). | Add to the existing timer loop in `lib.rs`. Track last export time in store. Use `tokio::time::interval`. | A03 |
| **A05** | Auto-export settings UI | P1 | Add settings toggles: enable/disable auto-export, schedule time, last export status. | Add fields to `Settings` struct + Settings component UI. | A04 |
| **A06** | Manual trigger + status indicator | P2 | Add "Export Now" button in app. Show last export time, success/failure status in the freshness indicator. | Add command + frontend button. | A03 |

#### Recommended Architecture for Auto-Export

Given the constraints (no public API, SPA-only web UI), use a **hybrid approach**:

```
┌─────────────────────────────────────────────────┐
│                  Rust Backend                     │
│                                                   │
│  ┌──────────────┐    ┌─────────────────────────┐ │
│  │ platform_auth │───▶│ WebView2 (hidden)       │ │
│  │ (cookie mgmt) │    │ - one-time login         │ │
│  └──────┬───────┘    │ - JS injection            │ │
│         │            └──────────┬──────────────┘ │
│         ▼                       │                │
│  ┌──────────────┐              │                │
│  │platform_export│◀─────────────┘                │
│  │ (reqwest +    │                               │
│  │  cookies)     │──▶ Downloads dir              │
│  └──────┬───────┘         │                      │
│         │                 ▼                      │
│         │         ┌──────────────┐               │
│         └────────▶│   watcher    │               │
│                   │ (picks up    │               │
│                   │  new CSVs)   │               │
│                   └──────────────┘               │
└─────────────────────────────────────────────────┘
```

**Fallback strategy if direct API reverse-engineering fails**:
- Use WebView2 (`tauri::WebviewWindowBuilder`) to create hidden window
- Navigate to `https://platform.deepseek.com/usage`
- Inject JS to click export buttons
- WebView2 natively downloads to Downloads folder
- Existing file watcher picks up the files automatically
- This is simpler but more fragile (depends on page structure)

---

### CYCLE 03 — Quality & Polish (P2/P3)

| ID | Title | Priority | Description |
|----|-------|----------|-------------|
| **Q01** | Add `tauri-plugin-shell` or notification for import success/failure | P2 | Show toast/notification when CSV auto-import succeeds. |
| **Q02** | Balance timer should pause when no API key set | P2 | Currently loops every interval even with empty key (just `continue`s). Could skip timer entirely. |
| **Q03** | CSV import should accept both `amount-2026-6.csv` and `amount-2026-06.csv` | P3 | File matcher only tests for `amount-` prefix. The month-without-zero format is handled. The zero-padded format needs `amount-\d{4}-\d{2}\.csv`. Currently both work since only prefix is checked. |
| **Q04** | Timezone localization for "Today"/"This Month" | P3 | Spec says "先按 UTC 显示并标注". Current UI already labels "Today (UTC)". Sufficient for now. |
| **Q05** | Low balance threshold warning | P3 | Spec mentions as future feature. Add threshold setting + notification. |
| **Q06** | Window drag support (titlebar drag to move) | P3 | Window is `decorations: false` so user can't drag it. Add `data-tauri-drag-region` to titlebar element. |
| **Q07** | CSV import dedup: `was_imported` uses `unwrap_or(true)` | P3 | If DB query fails, defaults to "already imported" and skips. This is conservative but could silently skip valid files during DB lock contention. Add retry logic. |

---

## Dependency Graph

```
C01 (persist settings)
├── C02 (timer reads settings)
├── C03 (watcher reads settings)
└── C06 (autostart menu checkbox depends on stored setting)

C04 (escape key) — independent
C05 (tray tooltip) — independent
C07 (iconAsTemplate) — independent
C08 (backslash paths) — independent
C09 (menu-autostart listener) — depends on C06

A01 (research) — independent
A02 (login/cookies) — depends on A01
A03 (CSV download) — depends on A01, A02
A04 (schedule) — depends on A03
A05 (UI) — depends on A04
A06 (manual trigger) — depends on A03
```

## Parallel Execution Groups

**Group 1** (no dependencies, can run in parallel):
C01, C04, C05, C07, C08, A01

**Group 2** (after Group 1):
C02 (after C01), C03 (after C01), C06 (after C01), C09 (after C06), A02 (after A01)

**Group 3** (after Group 2):
A03 (after A02), Q01-Q07 (anytime)

**Group 4** (after Group 3):
A04 (after A03), A05 (after A04), A06 (after A03)

---

## Files That Need Modification (Summary)

| File | Tasks |
|------|-------|
| `src-tauri/src/lib.rs` | C01, C02, C03, C05, C08, A04 |
| `src-tauri/src/commands.rs` | C01 (save_settings), C08 (dirs_download), A06 |
| `src-tauri/src/tray.rs` | C05 (tray handle store), C06 (checkmenu), C09 (emit handling) |
| `src-tauri/tauri.conf.json` | C07 (iconAsTemplate), C11 (already fixed) |
| `src-tauri/capabilities/default.json` | (verify, likely no changes needed) |
| `src/App.tsx` | C04 (escape key fix), C09 (menu-autostart listener) |
| `src/components/Settings.tsx` | A05 (auto-export UI) |
| `src/store.ts` | A05 (auto-export state) |
| `src/api.ts` | A06 (export-now command) |
| **NEW** `src-tauri/src/platform_auth.rs` | A02 |
| **NEW** `src-tauri/src/platform_export.rs` | A03 |

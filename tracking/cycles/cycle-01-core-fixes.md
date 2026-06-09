# Cycle 01 — Core Runtime Fixes

> **Goal**: Make the app actually work end-to-end. Fix all P0/P1 bugs found during codebase analysis.
> **Starting state**: Rust and TypeScript compile clean. App starts (after C11 fix). But many runtime bugs exist.
> **Success criteria**: User can enter API key, see it surviving restart. Balance fetches with user's key. Watcher uses correct directory. Tray tooltip shows live balance. Settings UI works properly.

---

## Bug Inventory

### Bug 1 (P0): Settings Lost on Every Restart

**Severity**: Critical — App unusable without this.

**Location**:
- `src-tauri/src/lib.rs:42` — `let cached_settings = Settings::default();` creates fresh defaults
- `src-tauri/src/commands.rs:142-151` — `save_settings()` only updates in-memory `SettingsState`
- No code anywhere calls `tauri-plugin-store` to persist/load settings

**Root Cause**: The `tauri-plugin-store` plugin is registered in `lib.rs:52` but never used. `SettingsState` is a simple in-memory `Mutex<Settings>` wrapper with no disk persistence.

**What Happens**:
1. User opens app, enters API key, saves settings
2. `save_settings()` calls `state.update(settings)` — updates RAM only
3. User restarts app
4. `Settings::default()` creates fresh empty settings
5. API key is empty, balance fetch never works

**Fix**:
1. In `lib.rs` `setup` closure: after creating `SettingsState`, call `app.store("settings.json")` to read persisted settings (if any), merge into `SettingsState`
2. In `commands.rs` `save_settings`: also write to persistent store using `app.store("settings.json")`
3. The `save_settings` command needs `AppHandle` parameter (already has it)

**Affected Files**:
- `src-tauri/src/lib.rs` — add store read in setup
- `src-tauri/src/commands.rs` — add store write in save_settings

**Verification**: Set API key, close app, reopen, verify API key is still filled.

---

### Bug 2 (P0): Timer Loop Uses Stale Default Settings Forever

**Severity**: Critical — Balance never fetches, even after user sets API key.

**Location**: `src-tauri/src/lib.rs:118-120`
```rust
let store_clone = store.clone();
let app_handle = app.handle().clone();
tauri::async_runtime::spawn(async move {
    let settings = Settings::default();  // <-- CAPTURED ONCE, NEVER UPDATED
    let interval = settings.interval_min.max(1);
    loop {
        // ...
        let api_key = settings.api_key.clone();  // <-- ALWAYS EMPTY STRING
```

**Root Cause**: `Settings::default()` is called once at spawn time, captured by the async block, and the `settings` variable never changes.

**Fix**: On each tick of the loop, read current settings from the managed state:
```rust
let settings = app_handle.state::<SettingsState>().get();
```
This requires `app_handle` to already be in scope (it is, via clone).

**Affected Files**: `src-tauri/src/lib.rs` (lines 118-120)

**Verification**: Set API key, wait for timer tick, verify balance appears.

---

### Bug 3 (P1): Watcher Monitors Default Downloads Dir, Ignores User Setting

**Severity**: High — User changes download dir in settings, watcher still watches old default.

**Location**: `src-tauri/src/lib.rs:68`
```rust
let downloads = std::path::PathBuf::from(&Settings::default().downloads_dir);
```

**Fix**: Read `downloads_dir` from the same `SettingsState` managed object. Or: restart watcher when settings change. Simpler: read from state once during setup (after C01 fix loads persisted settings).

**Affected Files**: `src-tauri/src/lib.rs` (line 68)

---

### Bug 4 (P1): Escape Key Doesn't Close Settings Panel

**Severity**: Medium — Annoying UX bug, user stuck in settings.

**Location**: `src/App.tsx:56-61`
```tsx
const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" && showSettings) {  // <-- showSettings is STALE
        setShowSettings(false);
    }
};
window.addEventListener("keydown", handleKeyDown);
```

**Root Cause**: `useEffect([], ...)` runs once on mount. `showSettings` from the destructured Zustand selector is captured at value `false`. When user opens settings and presses Escape, the captured `showSettings` is still `false`, so the condition never matches.

**Fix**: Use `useAppStore.getState().showSettings` instead of the destructured value:
```tsx
if (e.key === "Escape" && useAppStore.getState().showSettings) {
```

**Affected Files**: `src/App.tsx` (lines 56-58)

**Verification**: Open settings, press Escape, verify settings close.

---

### Bug 5 (P1): Tray Tooltip Never Shows Balance

**Severity**: Medium — Spec requirement not met.

**Location**:
- `src-tauri/src/tray.rs:26` — `.tooltip("DeepSeek Monitor")` is static
- No code stores the `TrayIcon` handle for later updating
- `lib.rs:141` emits `balance-updated` event but never calls `set_tooltip`

**Fix**:
1. Store the `TrayIcon` in managed state: wrap in `Arc<Mutex<Option<TrayIcon>>>`
2. In `lib.rs` timer loop, after fetching balance, call `tray.set_tooltip(Some(&format!("余额 ¥{:.2}", bal)))`
3. Import `ManagerExt` or use the tray handle directly

**Affected Files**:
- `src-tauri/src/tray.rs` — return tray handle from `create_tray()`
- `src-tauri/src/lib.rs` — store handle, update tooltip in timer

**Verification**: Set valid API key, wait for balance fetch, hover tray icon.

---

### Bug 6 (P1): Autostart Menu Item Is Plain Text, Not Checkable

**Severity**: Medium — Can't show current autostart state, confusing UX.

**Location**: `src-tauri/src/tray.rs:11`
```rust
let autostart_item = MenuItemBuilder::with_id("autostart", "Auto Start").build(app)?;
```

**Fix**: Use `CheckMenuItemBuilder` instead, which shows a checkmark. Read current autostart state on menu creation (via `tauri_plugin_autostart::ManagerExt`). On click, toggle state and rebuild menu (or update check state).

**Note**: Tauri 2 tray menu items cannot be updated after creation. To toggle the checkmark, you must rebuild the entire menu. This is a known Tauri 2 limitation. Alternative: just emit event to frontend (already done), and frontend shows toast/notification of the toggle.

**Affected Files**: `src-tauri/src/tray.rs` (lines 11, 40-42)

**Verification**: Toggle autostart, reopen tray menu, verify checkmark reflects current state.

---

### Bug 7 (P2): `iconAsTemplate: true` Is macOS-Only

**Severity**: Low — Harmless on Windows but semantically incorrect.

**Location**: `src-tauri/tauri.conf.json:32`

**Fix**: Change to `false` or remove the property.

**Affected Files**: `src-tauri/tauri.conf.json`

---

### Bug 8 (P2): Hardcoded Backslash in Path Construction

**Severity**: Low — Works fine on Windows but fragile.

**Location**:
- `src-tauri/src/lib.rs:172` — `format!("{}\\deepseek-monitor", p)`
- `src-tauri/src/commands.rs:126` — `format!("{}\\Downloads", p)`

**Fix**: Use `std::path::Path::new(&p).join("deepseek-monitor")` and `.to_string_lossy().to_string()`.

**Affected Files**:
- `src-tauri/src/lib.rs` (170-174)
- `src-tauri/src/commands.rs` (124-128)

---

### Bug 9 (P2): `"menu-autostart"` Event Not Listened in Frontend

**Severity**: Low — Clicking "Auto Start" in tray menu does nothing visible.

**Location**: Missing listener in `src/App.tsx`. Event is emitted in `tray.rs:42` but never consumed.

**Fix**: Add `listen("menu-autostart", handleAutostartToggle)` in App.tsx `useEffect`.

**Affected Files**: `src/App.tsx`

---

### Bug 10 (P3): Redundant `trayIcon.tooltip` in Config

**Severity**: Very Low — Code sets tooltip, config also has one. Redundant but harmless.

**Location**: `src-tauri/tauri.conf.json:33`

**Recommendation**: Remove `"tooltip"` from `trayIcon` block (keep the code-based tooltip that will be dynamic after Bug 5 fix).

---

### Bug 11 (P0 was, FIXED): Plugin Config Deserialization Crash

**Severity**: Was Critical, now Fixed.

**Location**: `src-tauri/tauri.conf.json:50`

**Fix**: User changed to `"plugins": {}`. Confirmed app now starts without crash.

---

## Implementation Order (Recommended)

```
Step 1: C07 (iconAsTemplate) — trivial, no dependencies
Step 2: C08 (backslash paths) — trivial, no dependencies
Step 3: C01 (persist settings) — foundation for C02, C03, C06
Step 4: C02 (timer reads settings) — depends on C01
Step 5: C03 (watcher reads settings) — depends on C01
Step 6: C04 (escape key) — independent
Step 7: C05 (tray tooltip) — independent
Step 8: C06 (autostart checkable) — depends on C01 (for stored autostart pref)
Step 9: C09 (menu-autostart listener) — depends on C06
Step 10: C10 (remove redundant tooltip) — trivial
```

**Parallelizable**: Steps 1, 2, 4, 6, 7 can all be done in parallel (they touch different files or non-overlapping sections).

---

## Files Touched in Cycle 01

| File | Bugs |
|------|------|
| `src-tauri/src/lib.rs` | C01, C02, C03, C05, C08 |
| `src-tauri/src/commands.rs` | C01, C08 |
| `src-tauri/src/tray.rs` | C05, C06, C09 |
| `src-tauri/tauri.conf.json` | C07, C10 |
| `src/App.tsx` | C04, C09 |
| **(no new files needed)** | |

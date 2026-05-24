# clash-tray

A Windows system-tray monitor and controller for an OpenClash / Mihomo router — the kind of setup you get with a GL.iNet Flint, Beryl, etc. Single user, Windows 10/11.

## What it does

- **Tray icon** with a tooltip reflecting connection state and the current proxy.
- **Popup** (click the tray or press `Ctrl+Alt+P`) — a hero card with the active country, leaf node, latency, and selection mode (Manual / Auto / Failover / Load balance), plus a per-group list for inspection and switching. One-click latency refresh on the active group.
- **Quick switch submenu** in the right-click tray menu — pick a favourite country or auto-select group with a checkmark on the current selection.
- **Toasts** when a URLTest / Fallback group re-elects a node, when the connection drops, and when it comes back.
- **Live stats** — download / upload speeds, session totals, active connection count.
- **Subscription info** parsed from your provider's pseudo-nodes (plan tier, remaining quota, expiry, reset countdown).
- **Settings UI** — URL, secret, poll interval, primary group, favourites, country overrides, autostart, notifications.

## Install

Grab the latest MSI from the [Releases page](https://github.com/Arghs/clash-tray/releases) and run it. Requires Windows 10 1809 or newer with WebView2 (already present on Windows 11; included on most Windows 10 builds).

## First run

1. Launch from the Start menu.
2. Right-click the tray icon → **Settings…**
3. Fill in your Clash external-controller URL (e.g. `http://192.168.8.1:9090`) and the dashboard secret.
4. **Test connection** — should report `OK — v<n> (meta) · <n> ms`.
5. **Save**. The popup will start populating within a poll cycle.

## Hotkey

- `Ctrl+Alt+P` — toggle the popup from anywhere.

## Where things live

- Settings: `%APPDATA%\com.clashtray.app\settings.json`
- Logs: `%APPDATA%\com.clashtray.app\logs\clash-tray.log.<YYYY-MM-DD>` (rolled daily)

## Build from source

```powershell
yarn install
yarn tauri build   # MSI lands in src-tauri/target/release/bundle/msi/
```

Requires Rust stable with the MSVC toolchain, Node 18+, and the MSVC "Desktop development with C++" workload.

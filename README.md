# clash-tray

A Windows system-tray monitor and controller for an OpenClash / Mihomo instance — designed for the typical GL.iNet router setup where the Clash external-controller lives at `http://192.168.8.1:9090`.

Personal tool, single user, Windows 10/11 only.

## Features

- Tray icon with a tooltip reflecting connection state and the current proxy.
- Click-to-toggle popup that lists every proxy group and its members, grouped by country, sorted by ping (dead nodes at the bottom).
- Click any node to switch to it. Click "Test latency" on a group to refresh ping numbers.
- Right-click tray menu with a **Quick switch** submenu (Auto select / Failover / Load balance entries from the primary group, plus your favourite countries) and a checkmark on the currently selected item.
- Auto-switch detection — if a URLTest / Fallback group re-elects a node, a Windows toast tells you which one.
- Connection-lost / connection-restored toasts.
- Real-time traffic stats: live ↓ / ↑ speeds + session totals + active connection count.
- Subscription info parsed from the provider's pseudo-nodes (plan tier, remaining quota, expiry, reset countdown).
- Settings UI: URL, secret, poll interval, primary group, favourites, country overrides, autostart, notifications.
- Global hotkey `Ctrl+Alt+P` toggles the popup.
- Rolling daily log at `%APPDATA%\dev.alex.clashtray\logs\clash-tray.log`.

## Stack

- **Backend:** Rust + Tauri 2 (`tauri-plugin-store`, `notification`, `opener`, `positioner`, `autostart`, `global-shortcut`, `single-instance`), `reqwest` (rustls), `tokio`, `tracing`.
- **Frontend:** SvelteKit (adapter-static, SPA mode) + Svelte 5 (runes) + TypeScript + Vite.
- **Polyfill:** `country-flag-emoji-polyfill` — Windows' default fonts can't render flag emoji.

## Requirements

- Windows 10 1809 or newer (target `x86_64-pc-windows-msvc`).
- Rust stable (1.75+) with the MSVC toolchain.
- Node 18+ and Yarn 1.x (`yarn` is preferred — `npm install` will also work).
- The MSVC Build Tools / "Desktop development with C++" workload (for the Tauri Rust build).
- WebView2 runtime — already present on Windows 11; pre-installed on most Windows 10 builds, otherwise install from Microsoft.

## Setup

```powershell
# from the repo root
yarn install
```

The first `yarn tauri dev` will compile a large dependency graph; budget ~5 minutes for the first build.

## Common commands

All commands run from the repo root.

| Task | Command |
| --- | --- |
| Run the app in dev mode (Vite + Tauri webview) | `yarn tauri dev` |
| Type-check the frontend | `yarn run check` |
| Build an MSI installer | `yarn tauri build` |
| Run Rust tests | `cd src-tauri; cargo test --lib` |
| Run a single Rust test | `cd src-tauri; cargo test --lib <name_substring>` |
| Probe the live Clash API | `cd src-tauri; cargo run --example probe -- http://192.168.8.1:9090 <secret>` |

`yarn tauri build` produces:
```
src-tauri/target/release/bundle/msi/clash-tray_<version>_x64_en-US.msi
```

Adding dependencies — use the package manager, do **not** edit the manifest files by hand:
- Rust: `cd src-tauri; cargo add <crate>`
- JS: `yarn add <pkg>`

If port `1420` (Vite dev) is stuck or you can't get a clean rebuild:
```powershell
Get-Process -Name node,clash-tray,cargo -ErrorAction SilentlyContinue | Stop-Process -Force
```

## First run

1. Launch via `yarn tauri dev` or by installing the MSI.
2. Right-click the tray icon → **Settings…**
3. Fill in:
   - **Clash URL** — e.g. `http://192.168.8.1:9090`.
   - **Secret** — the dashboard secret from OpenClash. Stored in `%APPDATA%\dev.alex.clashtray\settings.json`, never in the source.
4. Click **Test connection** — should report `OK — v<n> (meta) · <n> ms`.
5. **Save**. The tray popup will start populating within one poll interval.

## Configuration & data locations

- **Settings:** `%APPDATA%\dev.alex.clashtray\settings.json` (via `tauri-plugin-store`).
- **Logs:** `%APPDATA%\dev.alex.clashtray\logs\clash-tray.log.<YYYY-MM-DD>` (rolled daily by `tracing-appender`).
- **Autostart registry entry** (when enabled): `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\dev.alex.clashtray`.

## Project layout

```
clash-tray/
├─ src/                          # SvelteKit frontend
│  ├─ routes/+page.svelte        # window-label router (popup vs settings)
│  └─ lib/
│     ├─ views/{Popup,Settings}.svelte
│     ├─ components/             # ProxyGroup, NodeRow, ConnectionDot, DelayBadge, CountryHeader
│     ├─ api.ts                  # typed wrapper over invoke()
│     ├─ types.ts                # mirrors of Rust Serialize types
│     ├─ country.ts              # iso2 → flag/name helpers
│     └─ format.ts               # byte/speed formatting
└─ src-tauri/
   ├─ src/
   │  ├─ lib.rs                  # Builder + setup + plugin registration + logging + global hotkey
   │  ├─ clash/                  # ClashClient, types, errors — HTTP layer
   │  ├─ commands.rs             # #[tauri::command] handlers
   │  ├─ poll.rs                 # poll loop, snapshot construction, traffic/subscription parsing
   │  ├─ state.rs                # AppState, snapshot types
   │  ├─ tray.rs                 # tray menu (Quick switch, info header), submenu rebuilds
   │  ├─ country.rs              # 7-step country parser + 53 unit tests
   │  ├─ notify.rs               # Windows toast helpers
   │  ├─ settings.rs             # Settings struct + store I/O
   │  └─ events.rs               # serializable event payloads
   ├─ capabilities/default.json  # Tauri permissions
   ├─ tauri.conf.json
   └─ examples/probe.rs          # CLI probe against a live Clash router
```

## Known limitations

- Tray menu labels can't show flag emoji — Windows native menus render regional-indicator pairs as the two ASCII letters. The popup uses a Twemoji web-font polyfill so flags render correctly there.
- The notification "from" name shows as **Windows PowerShell** during `tauri dev`. After `tauri build` + install, the MSI registers the AUMID `dev.alex.clashtray` and toasts show the proper app name.
- Live speeds are averaged over the poll interval (default 2 s). Per-second resolution would need adding `tokio-tungstenite` and consuming Mihomo's `/traffic` WebSocket.
- Mode toggle (rule / global / direct), DNS query, rules list, and a 1-hour traffic graph are not implemented — `/configs` and `/connections` snapshots are the only Mihomo endpoints actually consumed beyond `/proxies`.
- No icon-glyph variants for `Connected` / `Lost` — state is signalled via the tooltip only.

## Architecture notes

- Single-source-of-truth `AppState` (three `tokio::sync::RwLock` fields).
- Poll loop ticks every `poll_interval_ms`, emits `state-updated` after each successful fetch.
- Two windows, both rendered from the same `+page.svelte` — the label of the active webview window decides which view to mount.
- Tray menu rebuilds are signature-diffed to avoid churning the OS menu every poll tick.

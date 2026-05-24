use std::sync::{Arc, Mutex};

use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Listener, Manager, PhysicalPosition, Runtime, WebviewWindow,
};

use crate::commands;
use crate::country::iso2_to_display;
use crate::state::{AppState, ConnectionState, GroupKind, StateSnapshot};

const TRAY_ID: &str = "main-tray";

/// Bundle of everything the menu builder needs. Computed under locks then released —
/// `build_menu_with` is pure and never touches state.
#[derive(Default, Clone)]
struct MenuInputs {
    favorites: Vec<String>,
    auto_members: Vec<(GroupKind, String)>,
    current_now: Option<String>,
    current_country: Option<String>,
    info_label: String,
    tooltip: String,
}

pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    // Initial menu uses what we can read synchronously from setup. Auto items and the
    // info line need the first poll's snapshot — the state-updated listener below
    // performs the first real rebuild.
    let initial_inputs = MenuInputs {
        favorites: match app.try_state::<Arc<AppState>>() {
            Some(s) => s.settings.blocking_read().favorites.clone(),
            None => Vec::new(),
        },
        info_label: "Connecting…".to_string(),
        tooltip: "Clash Tray — connecting…".to_string(),
        ..MenuInputs::default()
    };
    let menu = build_menu_with(app, &initial_inputs)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default window icon is embedded by tauri.conf.json bundle.icon");

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Clash Tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_popup(tray.app_handle());
            }
        })
        .build(app)?;

    // Rebuild on settings change (favorites or primary group can change).
    let app_for_settings = app.clone();
    app.listen("settings-changed", move |_| {
        let app = app_for_settings.clone();
        tauri::async_runtime::spawn(async move {
            rebuild_menu_async(&app).await;
        });
    });

    // Rebuild whenever the menu's relevant data changes — that's a cheap string
    // diff captured here. State-updated fires every poll tick; without the diff we
    // would churn the OS menu on every tick.
    let app_for_state = app.clone();
    let last_signature: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    app.listen("state-updated", move |_| {
        let app = app_for_state.clone();
        let last = last_signature.clone();
        tauri::async_runtime::spawn(async move {
            let inputs = collect_menu_inputs(&app).await;
            let sig = signature(&inputs);
            let changed = {
                let mut guard = last.lock().unwrap();
                if *guard != sig {
                    *guard = sig;
                    true
                } else {
                    false
                }
            };
            if changed {
                let _ = rebuild_with(&app, &inputs);
            }
        });
    });

    Ok(())
}

fn signature(inputs: &MenuInputs) -> String {
    // We don't include delay in the info; this signature changes only when names
    // change or the favorites/auto list changes — bounded rebuild frequency.
    format!(
        "info={}|tt={}|now={:?}|country={:?}|favs={}|autos={}",
        inputs.info_label,
        inputs.tooltip,
        inputs.current_now,
        inputs.current_country,
        inputs.favorites.join(","),
        inputs
            .auto_members
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>()
            .join(","),
    )
}

async fn rebuild_menu_async<R: Runtime>(app: &AppHandle<R>) {
    let inputs = collect_menu_inputs(app).await;
    let _ = rebuild_with(app, &inputs);
}

fn rebuild_with<R: Runtime>(app: &AppHandle<R>, inputs: &MenuInputs) -> tauri::Result<()> {
    let menu = build_menu_with(app, inputs)?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_menu(Some(menu));
        let _ = tray.set_tooltip(Some(&inputs.tooltip));
    }
    Ok(())
}

async fn collect_menu_inputs<R: Runtime>(app: &AppHandle<R>) -> MenuInputs {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return MenuInputs::default();
    };
    let (favorites, primary_name) = {
        let st = state.settings.read().await;
        (st.favorites.clone(), st.primary_group.clone())
    };

    let snap = state.snapshot.read().await;
    let primary = match primary_name.as_deref() {
        Some(name) => snap.groups.iter().find(|g| g.name == name),
        None => snap
            .groups
            .iter()
            .find(|g| g.kind == GroupKind::Selector && g.name != "GLOBAL"),
    };

    let auto_members = primary
        .map(|p| {
            p.members
                .iter()
                .filter_map(|m| match m.kind.as_str() {
                    "URLTest" => Some((GroupKind::UrlTest, m.name.clone())),
                    "Fallback" => Some((GroupKind::Fallback, m.name.clone())),
                    "LoadBalance" => Some((GroupKind::LoadBalance, m.name.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let current_now = primary.and_then(|p| p.now.clone());
    let current_country = primary.and_then(|p| p.now_country.clone());
    let info_label = info_for(&snap, primary);
    let tooltip = tooltip_for(&snap, primary);

    MenuInputs {
        favorites,
        auto_members,
        current_now,
        current_country,
        info_label,
        tooltip,
    }
}

fn tooltip_for(snap: &StateSnapshot, primary: Option<&crate::state::GroupView>) -> String {
    match snap.connection {
        ConnectionState::Lost => "Clash Tray — disconnected".to_string(),
        ConnectionState::Degraded => "Clash Tray — degraded".to_string(),
        ConnectionState::Connected => match primary {
            Some(p) => match p.now.as_deref() {
                Some(now) => format!("Clash Tray — {now}"),
                None => "Clash Tray".to_string(),
            },
            None => "Clash Tray".to_string(),
        },
    }
}

/// Render a one-line description of what's currently selected. Resolves through
/// auto-groups one level: if the primary's `now` points to a URLTest/Fallback group,
/// show "<group> → <leaf>" so the user can see which proxy is actually carrying traffic.
fn info_for(snap: &StateSnapshot, primary: Option<&crate::state::GroupView>) -> String {
    let Some(primary) = primary else {
        if snap.groups.is_empty() {
            return "Connecting…".to_string();
        }
        return "No primary group".to_string();
    };
    let Some(now) = primary.now.as_deref() else {
        return format!("{}: —", primary.name);
    };
    let child = snap.groups.iter().find(|g| {
        g.name == now
            && matches!(
                g.kind,
                GroupKind::UrlTest | GroupKind::Fallback | GroupKind::LoadBalance
            )
    });
    match child {
        Some(c) => {
            let leaf = c.now.as_deref().unwrap_or("—");
            format!("{now} → {leaf}")
        }
        None => now.to_string(),
    }
}

fn build_menu_with<R: Runtime>(
    app: &AppHandle<R>,
    inputs: &MenuInputs,
) -> tauri::Result<Menu<R>> {
    // Info header — disabled item showing what's currently selected.
    let info_label = format!("Current: {}", inputs.info_label);
    let info_item = MenuItem::with_id(app, "info", &info_label, false, None::<&str>)?;
    let hotkey_hint = MenuItem::with_id(
        app,
        "hotkey_hint",
        "Toggle popup: Ctrl+Alt+P",
        false,
        None::<&str>,
    )?;
    let info_sep = PredefinedMenuItem::separator(app)?;

    let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;

    // --- Quick switch submenu (checkable items) ---
    let auto_items: Vec<CheckMenuItem<R>> = inputs
        .auto_members
        .iter()
        .map(|(kind, name)| {
            let prefix = match kind {
                GroupKind::UrlTest => "Auto select",
                GroupKind::Fallback => "Failover",
                GroupKind::LoadBalance => "Load balance",
                _ => "Auto",
            };
            let label = format!("{prefix} — {name}");
            let checked = inputs.current_now.as_deref() == Some(name.as_str());
            CheckMenuItem::with_id(
                app,
                format!("auto:{name}"),
                &label,
                true,
                checked,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    // Native tray menus render regional-indicator emoji pairs as letter pairs, so the
    // popup polyfill doesn't help here. ISO2 + display name only.
    let country_items: Vec<CheckMenuItem<R>> = inputs
        .favorites
        .iter()
        .map(|c| {
            let label = format!("{} — {}", c, iso2_to_display(c));
            let checked = inputs.current_country.as_deref() == Some(c.as_str())
                && !inputs
                    .auto_members
                    .iter()
                    .any(|(_, n)| Some(n.as_str()) == inputs.current_now.as_deref());
            CheckMenuItem::with_id(
                app,
                format!("quick:{c}"),
                &label,
                true,
                checked,
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;

    let sep_in_quick;
    let placeholder;
    let mut quick_refs: Vec<&dyn IsMenuItem<R>> = Vec::new();
    for it in &auto_items {
        quick_refs.push(it);
    }
    if !auto_items.is_empty() && !country_items.is_empty() {
        sep_in_quick = PredefinedMenuItem::separator(app)?;
        quick_refs.push(&sep_in_quick);
    }
    for it in &country_items {
        quick_refs.push(it);
    }
    if quick_refs.is_empty() {
        placeholder = MenuItem::with_id(
            app,
            "quick_placeholder",
            "(no favorites — add some in Settings)",
            false,
            None::<&str>,
        )?;
        quick_refs.push(&placeholder);
    }
    let quick_switch = Submenu::with_id_and_items(
        app,
        "quick_switch",
        "Quick switch",
        true,
        &quick_refs,
    )?;

    let sep = PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &info_item,
            &hotkey_hint,
            &info_sep,
            &refresh,
            &quick_switch,
            &sep,
            &settings_item,
            &quit_item,
        ],
    )
}

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "refresh" => {
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                state.poll_notify.notify_one();
            }
        }
        "settings" => {
            let _ = commands::open_settings_impl(app);
        }
        "quit" => app.exit(0),
        s if s.starts_with("quick:") => {
            let country = s.strip_prefix("quick:").unwrap_or("").to_string();
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app2.try_state::<Arc<AppState>>() else {
                    return;
                };
                if let Err(e) =
                    commands::quick_switch_impl(&app2, state.inner(), &country).await
                {
                    tracing::warn!("quick_switch({country}) failed: {e}");
                }
            });
        }
        s if s.starts_with("auto:") => {
            let member = s.strip_prefix("auto:").unwrap_or("").to_string();
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app2.try_state::<Arc<AppState>>() else {
                    return;
                };
                if let Err(e) =
                    commands::switch_primary_to_impl(&app2, state.inner(), &member).await
                {
                    tracing::warn!("auto-select switch to '{member}' failed: {e}");
                }
            });
        }
        _ => {}
    }
}

pub fn toggle_popup<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window("popup") else {
        return;
    };
    let visible = win.is_visible().unwrap_or(false);
    if visible {
        // The on-focus-lost handler in setup() handles the fade and hide path,
        // so left-clicking the tray while the popup is visible just defocuses it.
        let _ = win.hide();
    } else {
        position_popup_near_tray(app, &win);
        // Tell the page to start invisible — without this, on re-show the page is
        // already opacity:1 from the previous show and the fade-in is invisible.
        let _ = tauri::Emitter::emit(app, "popup-showing", ());
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// `tauri-plugin-positioner`'s `Position::TrayCenter` panics with "Tray position not set"
/// if the user fires the global hotkey before the tray icon has emitted any
/// Click/Enter/Leave/Move event — on Windows that's a hard process abort via __fastfail
/// (release builds use panic=abort). Compute the position ourselves from `TrayIcon::rect()`,
/// which the OS reports as soon as the icon is created.
fn position_popup_near_tray<R: Runtime>(app: &AppHandle<R>, win: &WebviewWindow<R>) {
    let win_size = win
        .outer_size()
        .unwrap_or_else(|_| tauri::PhysicalSize::new(380, 520));
    let scale = win.scale_factor().unwrap_or(1.0);

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(Some(rect)) = tray.rect() {
            let tray_pos = rect.position.to_physical::<i32>(scale);
            let tray_size = rect.size.to_physical::<u32>(scale);
            let x = tray_pos.x + (tray_size.width as i32 / 2) - (win_size.width as i32 / 2);
            let y_above = tray_pos.y - win_size.height as i32;
            // If the tray is at the top of the screen (rare on Windows), drop below it.
            let y = if y_above < 0 {
                tray_pos.y + tray_size.height as i32
            } else {
                y_above
            };
            let _ = win.set_position(PhysicalPosition { x, y });
            return;
        }
    }

    // Fallback: bottom-right of the primary monitor (default Windows tray location).
    if let Ok(Some(monitor)) = win.primary_monitor() {
        let mon_size = monitor.size();
        let mon_pos = monitor.position();
        let x = mon_pos.x + mon_size.width as i32 - win_size.width as i32 - 16;
        let y = mon_pos.y + mon_size.height as i32 - win_size.height as i32 - 48;
        let _ = win.set_position(PhysicalPosition { x, y });
    }
}

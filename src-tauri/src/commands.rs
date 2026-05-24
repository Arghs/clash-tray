use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder};

use crate::clash::{ClashClient, VersionInfo};
use crate::settings::{self, Settings};
use crate::state::{AppState, GroupKind, GroupView, StateSnapshot};

pub fn open_settings_impl<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show()?;
        w.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("".into()))
        .inner_size(480.0, 600.0)
        .title("Clash Tray — Settings")
        .decorations(true)
        .resizable(false)
        .build()?;
    Ok(())
}

#[tauri::command]
pub async fn open_settings<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    open_settings_impl(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn hide_popup<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("popup") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_state(state: State<'_, Arc<AppState>>) -> Result<StateSnapshot, String> {
    Ok(state.snapshot.read().await.clone())
}

#[tauri::command]
pub async fn refresh_now(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.poll_notify.notify_one();
    Ok(())
}

#[tauri::command]
pub async fn switch_proxy<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
    group: String,
    node: String,
) -> Result<(), String> {
    state
        .last_manual_switch
        .lock()
        .unwrap()
        .insert(group.clone(), Instant::now());

    {
        let client = state.client.read().await;
        client.select(&group, &node).await.map_err(|e| e.to_string())?;
    }

    {
        let mut snap = state.snapshot.write().await;
        if let Some(g) = snap.groups.iter_mut().find(|g| g.name == group) {
            g.now = Some(node.clone());
        }
    }
    let snap = state.snapshot.read().await.clone();
    app.emit("state-updated", snap).map_err(|e| e.to_string())?;
    state.poll_notify.notify_one();
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, Arc<AppState>>) -> Result<Settings, String> {
    Ok(state.settings.read().await.clone())
}

#[tauri::command]
pub async fn save_settings<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
    new_settings: Settings,
) -> Result<(), String> {
    settings::save(&app, &new_settings)?;

    let url_or_secret_changed = {
        let cur = state.settings.read().await;
        cur.clash_url != new_settings.clash_url || cur.secret != new_settings.secret
    };
    if url_or_secret_changed {
        let new_client = ClashClient::new(&new_settings.clash_url, new_settings.secret.clone());
        *state.client.write().await = new_client;
    }
    *state.settings.write().await = new_settings.clone();

    app.emit("settings-changed", new_settings)
        .map_err(|e| e.to_string())?;
    state.poll_notify.notify_one();
    Ok(())
}

#[tauri::command]
pub async fn test_connection(url: String, secret: Option<String>) -> Result<VersionInfo, String> {
    let c = ClashClient::new(&url, secret);
    c.version().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_group_delay(
    state: State<'_, Arc<AppState>>,
    group: String,
) -> Result<HashMap<String, u32>, String> {
    let client = state.client.read().await;
    client.group_delay(&group).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn quick_switch<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, Arc<AppState>>,
    country: String,
) -> Result<String, String> {
    quick_switch_impl(&app, state.inner(), &country).await
}

/// Pick the fastest node in `country` within the primary group (or first selector
/// that has a match), switch to it, and return its name. Shared by the tauri::command
/// wrapper and the tray menu handler.
pub async fn quick_switch_impl<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    country: &str,
) -> Result<String, String> {
    let country = country.to_uppercase();
    let primary_group = state.settings.read().await.primary_group.clone();
    let snap = state.snapshot.read().await.clone();

    let group = choose_group(&snap.groups, primary_group.as_deref(), &country)
        .ok_or_else(|| format!("No selector group has a node in {country}"))?;

    let best = group
        .members
        .iter()
        .filter(|m| m.country == country && !m.is_group && !m.name.contains('\u{ff1a}'))
        .min_by(|a, b| {
            let da = a.latest_delay.filter(|d| *d != 0).unwrap_or(u32::MAX);
            let db = b.latest_delay.filter(|d| *d != 0).unwrap_or(u32::MAX);
            da.cmp(&db).then_with(|| a.name.cmp(&b.name))
        })
        .ok_or_else(|| format!("No {country} node found in group {}", group.name))?;

    let group_name = group.name.clone();
    let node_name = best.name.clone();

    state
        .last_manual_switch
        .lock()
        .unwrap()
        .insert(group_name.clone(), Instant::now());

    {
        let client = state.client.read().await;
        client
            .select(&group_name, &node_name)
            .await
            .map_err(|e| e.to_string())?;
    }
    {
        let mut snap = state.snapshot.write().await;
        if let Some(g) = snap.groups.iter_mut().find(|g| g.name == group_name) {
            g.now = Some(node_name.clone());
        }
    }
    let snap = state.snapshot.read().await.clone();
    app.emit("state-updated", snap).map_err(|e| e.to_string())?;
    state.poll_notify.notify_one();
    Ok(node_name)
}

fn choose_group<'a>(
    groups: &'a [GroupView],
    primary: Option<&str>,
    country: &str,
) -> Option<&'a GroupView> {
    if let Some(name) = primary {
        if let Some(g) = groups.iter().find(|g| g.name == name) {
            if g.members.iter().any(|m| m.country == country) {
                return Some(g);
            }
        }
    }
    groups.iter().find(|g| {
        g.kind == GroupKind::Selector
            && g.name != "GLOBAL"
            && g.members.iter().any(|m| m.country == country)
    })
}

/// Switch the primary selector group to a specific named member. Used for tray menu
/// "Auto select" / "Failover" items where the member is itself a URLTest / Fallback
/// group rather than a leaf node.
pub async fn switch_primary_to_impl<R: Runtime>(
    app: &AppHandle<R>,
    state: &Arc<AppState>,
    member: &str,
) -> Result<(), String> {
    let primary_name = state.settings.read().await.primary_group.clone();
    let snap = state.snapshot.read().await.clone();

    let primary = if let Some(name) = primary_name.as_deref() {
        snap.groups.iter().find(|g| g.name == name)
    } else {
        snap.groups
            .iter()
            .find(|g| g.kind == GroupKind::Selector && g.name != "GLOBAL")
    }
    .ok_or_else(|| "No primary selector group available".to_string())?;

    let group_name = primary.name.clone();
    state
        .last_manual_switch
        .lock()
        .unwrap()
        .insert(group_name.clone(), Instant::now());
    {
        let client = state.client.read().await;
        client
            .select(&group_name, member)
            .await
            .map_err(|e| e.to_string())?;
    }
    {
        let mut snap = state.snapshot.write().await;
        if let Some(g) = snap.groups.iter_mut().find(|g| g.name == group_name) {
            g.now = Some(member.to_string());
        }
    }
    let snap = state.snapshot.read().await.clone();
    app.emit("state-updated", snap).map_err(|e| e.to_string())?;
    state.poll_notify.notify_one();
    Ok(())
}

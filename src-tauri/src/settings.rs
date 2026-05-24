use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub clash_url: String,
    #[serde(default)]
    pub secret: Option<String>,
    pub poll_interval_ms: u64,
    #[serde(default)]
    pub primary_group: Option<String>,
    pub favorites: Vec<String>,
    pub country_overrides: HashMap<String, String>,
    pub notify_auto_switch: bool,
    pub notify_connection: bool,
    pub start_on_login: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            clash_url: "http://192.168.8.1:9090".to_string(),
            secret: None,
            poll_interval_ms: 2000,
            primary_group: None,
            favorites: ["HK", "JP", "SG", "TW", "US"]
                .into_iter()
                .map(String::from)
                .collect(),
            country_overrides: HashMap::new(),
            notify_auto_switch: true,
            notify_connection: true,
            start_on_login: false,
        }
    }
}

pub const STORE_FILE: &str = "settings.json";
pub const SETTINGS_KEY: &str = "settings";

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    let Ok(store) = app.store(STORE_FILE) else {
        return Settings::default();
    };
    match store.get(SETTINGS_KEY) {
        Some(v) => serde_json::from_value(v).unwrap_or_default(),
        None => Settings::default(),
    }
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let v = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set(SETTINGS_KEY, v);
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

use tauri::{AppHandle, Runtime};
use tauri_plugin_notification::NotificationExt;
use tracing::warn;

use crate::country::iso2_to_flag;
use crate::events::AutoSwitch;

pub fn auto_switch<R: Runtime>(app: &AppHandle<R>, sw: &AutoSwitch) {
    let title = format!("{} switched", sw.group);
    let body = format!(
        "{} {} → {} {}",
        iso2_to_flag(&sw.from_country),
        sw.from,
        iso2_to_flag(&sw.to_country),
        sw.to,
    );
    if let Err(e) = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show()
    {
        warn!("notification (auto-switch) failed: {e}");
    }
}

pub fn connection_lost<R: Runtime>(app: &AppHandle<R>, url: &str) {
    if let Err(e) = app
        .notification()
        .builder()
        .title("Clash unreachable")
        .body(format!("{url} — check OpenClash on the router"))
        .show()
    {
        warn!("notification (connection-lost) failed: {e}");
    }
}

pub fn connection_restored<R: Runtime>(app: &AppHandle<R>, url: &str) {
    if let Err(e) = app
        .notification()
        .builder()
        .title("Clash reconnected")
        .body(format!("Connection to {url} restored"))
        .show()
    {
        warn!("notification (connection-restored) failed: {e}");
    }
}

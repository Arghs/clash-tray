pub mod clash;
pub mod commands;
pub mod country;
pub mod events;
pub mod notify;
pub mod poll;
pub mod settings;
pub mod state;
pub mod tray;

use std::fs;
use std::sync::Arc;

use state::AppState;
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("popup") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            init_logging(app);

            let s = settings::load(app.handle());
            let state = Arc::new(AppState::new(s));
            app.manage(state.clone());

            let popup = WebviewWindowBuilder::new(app, "popup", WebviewUrl::App("".into()))
                .inner_size(420.0, 640.0)
                .decorations(false)
                .transparent(true)
                .resizable(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .shadow(true)
                .build()?;

            let popup_for_event = popup.clone();
            popup.on_window_event(move |event| {
                if let WindowEvent::Focused(false) = event {
                    // Let the page run a fade-out, then actually hide the window.
                    let win = popup_for_event.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = win.emit("popup-hiding", ());
                        tokio::time::sleep(std::time::Duration::from_millis(130)).await;
                        let _ = win.hide();
                    });
                }
            });

            tray::build_tray(app.handle())?;
            poll::spawn(app.handle().clone(), state);

            // Global hotkey: Ctrl+Alt+P toggles the popup.
            let app_for_hotkey = app.handle().clone();
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
            if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_, _, event| {
                if event.state() == ShortcutState::Pressed {
                    tray::toggle_popup(&app_for_hotkey);
                }
            }) {
                tracing::warn!("failed to register global hotkey Ctrl+Alt+P: {e}");
            }

            tracing::info!("clash-tray started");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_settings,
            commands::hide_popup,
            commands::get_state,
            commands::refresh_now,
            commands::switch_proxy,
            commands::get_settings,
            commands::save_settings,
            commands::test_connection,
            commands::test_group_delay,
            commands::quick_switch,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_logging(app: &tauri::App) {
    let log_dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("clash-tray-logs"));
    let _ = fs::create_dir_all(&log_dir);

    let appender = tracing_appender::rolling::daily(&log_dir, "clash-tray.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);
    // The guard must outlive the program; leak it intentionally — only happens once.
    Box::leak(Box::new(guard));

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("clash_tray=debug,info"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true);
    let stderr_layer = fmt::layer().with_writer(std::io::stderr).with_ansi(true);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer)
        .try_init();

    tracing::info!(?log_dir, "logging initialized");
}

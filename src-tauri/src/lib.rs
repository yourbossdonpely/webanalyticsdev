//! Web Analytics Dev by Don PELY — Tauri application entry.
//!
//! Owns the window, the file store, the local collector, the uptime engine
//! and desktop notifications. The React UI talks to all of it via commands.

mod analytics;
mod collector;
mod demo;
mod monitor;
mod store;

use collector::Collector;
use serde::Serialize;
use std::sync::{Arc, Mutex};
use store::{CheckRow, EventRow, Incident, Monitor, MonitorPatch, Settings, SettingsPatch, Site, Store};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_notification::NotificationExt;

struct AppState {
    store: Arc<Mutex<Store>>,
    collector: Mutex<Collector>,
}

fn with_store<T>(state: &State<'_, AppState>, f: impl for<'s> FnOnce(&'s Store) -> Result<T, String>) -> Result<T, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    f(&store)
}

// ---------- sites & analytics ----------

#[tauri::command]
fn sites_list(state: State<'_, AppState>) -> Result<Vec<Site>, String> {
    with_store(&state, |s| Ok(s.list_sites()))
}

#[tauri::command]
fn sites_add(state: State<'_, AppState>, domain: String, name: String, timezone: String) -> Result<Site, String> {
    with_store(&state, |s| s.add_site(&domain, &name, &timezone))
}

#[tauri::command]
fn sites_remove(state: State<'_, AppState>, id: String) -> Result<Vec<Site>, String> {
    with_store(&state, |s| s.remove_site(&id))
}

#[tauri::command]
fn sites_snippet(state: State<'_, AppState>, domain: String) -> Result<String, String> {
    with_store(&state, |s| {
        let port = s.get_settings().collector_port;
        Ok(format!(r#"<script defer src="http://127.0.0.1:{port}/t.js" data-site="{domain}"></script>"#))
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyticsFilter {
    field: String,
    value: String,
}

fn event_field<'a>(e: &'a EventRow, field: &str) -> &'a str {
    match field {
        "site" => &e.site,
        "kind" => &e.kind,
        "name" => &e.name,
        "path" => &e.path,
        "source" => &e.source,
        "medium" => &e.medium,
        "campaign" => &e.campaign,
        "device" => &e.device,
        "browser" => &e.browser,
        "country" => &e.country,
        _ => "",
    }
}

#[tauri::command]
fn analytics_query(
    state: State<'_, AppState>,
    site: Option<String>,
    days: Option<i64>,
    filter: Option<AnalyticsFilter>,
) -> Result<analytics::DashboardData, String> {
    with_store(&state, |s| {
        let days = days.unwrap_or(30);
        let mut events = s.load_events(site.as_deref(), days * 2 + 1);
        if let Some(f) = filter {
            events.retain(|e| event_field(e, &f.field) == f.value);
        }
        Ok(analytics::query_dashboard(&events, days))
    })
}

#[tauri::command]
fn analytics_realtime(state: State<'_, AppState>) -> Result<analytics::RealtimeData, String> {
    with_store(&state, |s| Ok(analytics::query_realtime(&s.load_events(None, 1), 5)))
}

// ---------- monitors ----------

#[tauri::command]
fn monitors_list(state: State<'_, AppState>) -> Result<Vec<Monitor>, String> {
    with_store(&state, |s| Ok(s.list_monitors()))
}

#[tauri::command]
fn monitors_add(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    url: String,
    interval_sec: u64,
    timeout_ms: u64,
) -> Result<Monitor, String> {
    let monitor = with_store(&state, |s| s.add_monitor(&name, &url, interval_sec, timeout_ms, 200))?;
    // First check runs immediately on a background thread so the UI stays snappy.
    let app_clone = app.clone();
    let store_clone = state.store.clone();
    let id = monitor.id.clone();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder().build().expect("http client");
        if let Ok(s) = store_clone.lock() {
            let owned = s.clone();
            drop(s);
            monitor::run_check(&app_clone, &owned, &client, &id);
        }
    });
    Ok(monitor)
}

#[tauri::command]
fn monitors_update(state: State<'_, AppState>, id: String, patch: MonitorPatch) -> Result<Monitor, String> {
    with_store(&state, |s| s.patch_monitor(&id, &patch))
}

#[tauri::command]
fn monitors_remove(state: State<'_, AppState>, id: String) -> Result<Vec<Monitor>, String> {
    with_store(&state, |s| s.remove_monitor(&id))
}

#[tauri::command]
fn monitors_check(app: AppHandle, state: State<'_, AppState>, id: Option<String>) -> Result<bool, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?.clone();
    let client = reqwest::blocking::Client::builder().build().map_err(|e| e.to_string())?;
    match id {
        Some(one) => {
            monitor::run_check(&app, &store, &client, &one);
        }
        None => {
            for m in store.list_monitors() {
                if m.enabled {
                    monitor::run_check(&app, &store, &client, &m.id);
                }
            }
        }
    }
    Ok(true)
}

#[tauri::command]
fn monitors_stats(state: State<'_, AppState>, hours: Option<i64>) -> Result<Vec<monitor::MonitorStats>, String> {
    with_store(&state, |s| Ok(monitor::stats_for(s, hours.unwrap_or(24))))
}

#[tauri::command]
fn monitors_checks(
    state: State<'_, AppState>,
    monitor_id: Option<String>,
    hours: Option<i64>,
) -> Result<Vec<CheckRow>, String> {
    with_store(&state, |s| Ok(s.load_checks(monitor_id.as_deref(), hours.unwrap_or(72))))
}

#[tauri::command]
fn incidents_list(state: State<'_, AppState>) -> Result<Vec<Incident>, String> {
    with_store(&state, |s| Ok(s.list_incidents()))
}

// ---------- settings & maintenance ----------

#[tauri::command]
fn settings_get(state: State<'_, AppState>) -> Result<Settings, String> {
    with_store(&state, |s| Ok(s.get_settings()))
}

#[tauri::command]
fn settings_save(state: State<'_, AppState>, patch: SettingsPatch) -> Result<Settings, String> {
    let next = with_store(&state, |s| s.save_settings(&patch))?;
    // Restart the collector so port/enabled changes apply immediately.
    let store = state.store.lock().map_err(|e| e.to_string())?.clone();
    let mut collector = state.collector.lock().map_err(|e| e.to_string())?;
    collector.stop();
    if next.collector_enabled {
        *collector = Collector::start(store, next.collector_port);
    }
    Ok(next)
}

#[tauri::command]
fn notify_test(app: AppHandle) -> Result<bool, String> {
    app.notification()
        .builder()
        .title("Web Analytics Dev")
        .body("Desktop notifications are working. We will alert you here if a site goes down.")
        .show()
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectorStatus {
    running: bool,
    port: Option<u16>,
}

#[tauri::command]
fn collector_status(state: State<'_, AppState>) -> Result<CollectorStatus, String> {
    let collector = state.collector.lock().map_err(|e| e.to_string())?;
    Ok(CollectorStatus { running: collector.port().is_some(), port: collector.port() })
}

#[tauri::command]
fn demo_seed(state: State<'_, AppState>, app: AppHandle) -> Result<bool, String> {
    with_store(&state, |s| {
        demo::seed_demo(s);
        Ok(true)
    })?;
    let store = state.store.lock().map_err(|e| e.to_string())?.clone();
    monitor::emit_monitors(&app, &store);
    Ok(true)
}

#[tauri::command]
fn data_reset(state: State<'_, AppState>, app: AppHandle) -> Result<bool, String> {
    with_store(&state, |s| {
        s.reset_all();
        Ok(true)
    })?;
    let store = state.store.lock().map_err(|e| e.to_string())?.clone();
    monitor::emit_monitors(&app, &store);
    Ok(true)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: String,
    version: String,
    author: String,
    platform: String,
}

#[tauri::command]
fn app_info(app: AppHandle) -> Result<AppInfo, String> {
    Ok(AppInfo {
        name: "Web Analytics Dev".to_string(),
        version: app.package_info().version.to_string(),
        author: "Don PELY".to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

// ---------- app ----------

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let open = MenuItem::with_id(app, "open", "Open Web Analytics Dev", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().expect("window icon").clone())
        .menu(&menu)
        .tooltip("Web Analytics Dev")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map(|d| d.join("data")).map_err(|e| e.to_string())?;
            let store = Store::new(data_dir);
            let fresh = store.list_sites().is_empty() && store.list_monitors().is_empty();
            if fresh {
                demo::seed_demo(&store);
            }
            let settings = store.get_settings();
            let collector = if settings.collector_enabled {
                Collector::start(store.clone(), settings.collector_port)
            } else {
                Collector::stopped()
            };
            let shared = Arc::new(Mutex::new(store.clone()));
            app.manage(AppState { store: shared.clone(), collector: Mutex::new(collector) });
            monitor::spawn_scheduler(app.handle().clone(), shared);
            build_tray(app.handle())?;
            // Confirm the backend is alive (also exercises the event channel).
            let _ = app.emit("backend-ready", true);
            Ok(())
        })
        .on_window_event(|win, event| {
            // Monitoring + collecting continue in the tray when the window closes.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            sites_list,
            sites_add,
            sites_remove,
            sites_snippet,
            analytics_query,
            analytics_realtime,
            monitors_list,
            monitors_add,
            monitors_update,
            monitors_remove,
            monitors_check,
            monitors_stats,
            monitors_checks,
            incidents_list,
            settings_get,
            settings_save,
            notify_test,
            collector_status,
            demo_seed,
            data_reset,
            app_info
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Web Analytics Dev");
}

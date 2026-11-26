//! Web Analytics Dev by Don PELY — website uptime monitor engine.
//!
//! Each enabled monitor gets an HTTP(S) check on its own interval plus a
//! best-effort ICMP ping sample. A site is DOWN after 2 consecutive failures
//! and UP again on the first success. Transitions raise desktop notifications
//! and are recorded as incidents.

use crate::store::{now_ms, CheckRow, Monitor, Store};
use serde::Serialize;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

const USER_AGENT: &str = "WebAnalyticsDev-Uptime/0.1 (+https://github.com/yourbossdonpely/webanalyticsdev)";

pub struct HttpOutcome {
    pub ok: bool,
    pub status: Option<u16>,
    pub error: Option<String>,
    pub latency_ms: u64,
}

pub fn http_check(client: &reqwest::blocking::Client, url: &str, timeout_ms: u64, expect_status: u16) -> HttpOutcome {
    let started = Instant::now();
    let result = client.get(url).timeout(Duration::from_millis(timeout_ms)).send();
    let latency_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(res) => {
            let status = res.status().as_u16();
            let ok = if expect_status == 0 { status < 400 } else { status == expect_status };
            HttpOutcome {
                ok,
                status: Some(status),
                error: if ok { None } else { Some(format!("HTTP {status}")) },
                latency_ms,
            }
        }
        Err(e) => HttpOutcome { ok: false, status: None, error: Some(trim_error(&e.to_string())), latency_ms },
    }
}

fn trim_error(s: &str) -> String {
    s.chars().take(200).collect()
}

/// Best-effort ICMP ping. Never panics; returns latency on success.
pub fn ping_check(url: &str) -> Option<u64> {
    let host = url
        .split("://").nth(1).unwrap_or(url)
        .split('/').next().unwrap_or("")
        .split('@').next_back().unwrap_or("")
        .split(':').next().unwrap_or("");
    if host.is_empty() {
        return None;
    }
    let args: &[&str] = if cfg!(windows) {
        &["-n", "1", "-w", "3000", host]
    } else {
        &["-c", "1", "-W", "3", host]
    };
    let started = Instant::now();
    let output = Command::new("ping").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_lowercase();
    // matches "time=12ms", "time=12 ms", "time<1ms"
    let mut best: Option<u64> = None;
    for token in text.split_whitespace() {
        let t = token.trim_matches(|c: char| c == ',' || c == ';');
        if let Some(rest) = t.strip_prefix("time=") {
            let num: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(v) = num.parse::<f64>() {
                best = Some(v.round() as u64);
            }
        } else if t == "time<" || t.starts_with("time<") {
            best = Some(1);
        }
    }
    best.or(Some(started.elapsed().as_millis() as u64))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorStats {
    pub id: String,
    pub checks: usize,
    pub up: usize,
    pub uptime_pct: Option<f64>,
    pub p50_ms: Option<u64>,
}

pub fn stats_for(store: &Store, hours: i64) -> Vec<MonitorStats> {
    let monitors = store.list_monitors();
    let checks = store.load_checks(None, hours);
    monitors
        .into_iter()
        .map(|m| {
            let rows: Vec<&CheckRow> = checks.iter().filter(|c| c.monitor_id == m.id).collect();
            let up = rows.iter().filter(|c| c.ok).count();
            let mut lat: Vec<u64> = rows.iter().filter(|c| c.ok).filter_map(|c| c.latency_ms).collect();
            lat.sort_unstable();
            let p50 = if lat.is_empty() { None } else { Some(lat[lat.len() / 2]) };
            MonitorStats {
                id: m.id,
                checks: rows.len(),
                up,
                uptime_pct: if rows.is_empty() { None } else { Some(up as f64 / rows.len() as f64 * 100.0) },
                p50_ms: p50,
            }
        })
        .collect()
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

/// Run one check for a monitor. Returns the new status, or None if skipped.
pub fn run_check(app: &AppHandle, store: &Store, client: &reqwest::blocking::Client, id: &str) -> Option<String> {
    let monitor: Monitor = store.list_monitors().into_iter().find(|m| m.id == id)?;
    if !monitor.enabled {
        return None;
    }

    let http = http_check(client, &monitor.url, monitor.timeout_ms, monitor.expect_status);
    let ping_ms = ping_check(&monitor.url);

    let fails = if http.ok { 0 } else { monitor.consecutive_fails + 1 };
    let was_down = monitor.status == "down";
    let is_down = !http.ok && fails >= 2;
    let next_status = if http.ok { "up" } else if is_down || was_down { "down" } else { "unknown" };

    let _ = store.update_monitor_status(
        id,
        next_status,
        http.ok.then_some(http.latency_ms),
        ping_ms,
        http.error.clone(),
        fails,
    );
    let _ = store.record_check(&CheckRow {
        t: now_ms(),
        monitor_id: id.to_string(),
        ok: http.ok,
        status: http.status,
        latency_ms: http.ok.then_some(http.latency_ms),
        ping_ms,
        error: http.error.clone(),
    });

    let settings = store.get_settings();
    if next_status == "down" && !was_down {
        if let Ok(incident) = store.open_incident(id, &monitor.name, &monitor.url, http.error.as_deref().unwrap_or("site unreachable")) {
            if settings.notifications_enabled {
                notify(app, &format!("🔴 {} is DOWN", monitor.name), &format!("{}\n{}", monitor.url, incident.error));
                let _ = store.mark_notified(&incident.id, "down");
            }
        }
    } else if next_status == "up" && was_down {
        if let Ok(Some(incident)) = store.resolve_incident(id) {
            if settings.notifications_enabled && settings.notify_on_recovery {
                let mins = ((now_ms() - incident.started_at) / 60000).max(1);
                notify(app, &format!("🟢 {} is back UP", monitor.name), &format!("Downtime: {mins} min. {}", monitor.url));
                let _ = store.mark_notified(&incident.id, "up");
            }
        }
    }

    emit_monitors(app, store);
    Some(next_status.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorsPayload {
    monitors: Vec<Monitor>,
    incidents: Vec<crate::store::Incident>,
}

pub fn emit_monitors(app: &AppHandle, store: &Store) {
    let payload = MonitorsPayload { monitors: store.list_monitors(), incidents: store.list_incidents() };
    let _ = app.emit("monitors-updated", payload);
}

/// Background scheduler: ticks every second, checks monitors past their due time.
pub fn spawn_scheduler(app: AppHandle, store: Arc<Mutex<Store>>) {
    std::thread::Builder::new()
        .name("wadev-monitor".into())
        .spawn(move || {
            let client = reqwest::blocking::Client::builder()
                .user_agent(USER_AGENT)
                .build()
                .expect("http client");
            let mut due: HashMap<String, Instant> = HashMap::new();
            loop {
                let monitors = store.lock().map(|s| s.list_monitors()).unwrap_or_default();
                for m in monitors {
                    if !m.enabled {
                        due.remove(&m.id);
                        continue;
                    }
                    let entry = due.entry(m.id.clone()).or_insert_with(Instant::now);
                    if Instant::now() >= *entry {
                        *entry = Instant::now() + Duration::from_secs(m.interval_sec);
                        let id = m.id.clone();
                        // Clone the store handle; the (slow) check runs without holding the lock.
                        if let Ok(s) = store.lock() {
                            let owned = s.clone();
                            drop(s);
                            run_check(&app, &owned, &client, &id);
                        }
                    }
                }
                // Prune entries for deleted monitors.
                let ids: std::collections::HashSet<String> =
                    store.lock().map(|s| s.list_monitors().into_iter().map(|m| m.id).collect()).unwrap_or_default();
                due.retain(|id, _| ids.contains(id));
                std::thread::sleep(Duration::from_secs(1));
            }
        })
        .expect("spawn monitor thread");
}

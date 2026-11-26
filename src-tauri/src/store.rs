//! Web Analytics Dev by Don PELY — file-backed data store.
//!
//! Zero-db JSON storage under the app data directory:
//!   sites.json / monitors.json / incidents.json / settings.json (documents)
//!   events.jsonl / checks.jsonl (append-only logs, trimmed past a cap)

use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn uid(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{:x}_{n}", now_ms())
}

fn clamp_str(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

// ---------- models ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub domain: String,
    pub name: String,
    #[serde(default)]
    pub timezone: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    pub id: String,
    pub name: String,
    pub url: String,
    pub interval_sec: u64,
    pub timeout_ms: u64,
    pub expect_status: u16,
    pub enabled: bool,
    /// "up" | "down" | "unknown"
    pub status: String,
    pub last_check_at: Option<i64>,
    pub last_latency_ms: Option<u64>,
    pub last_ping_ms: Option<u64>,
    pub last_error: Option<String>,
    pub consecutive_fails: u32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorPatch {
    pub name: Option<String>,
    pub url: Option<String>,
    pub interval_sec: Option<u64>,
    pub timeout_ms: Option<u64>,
    pub expect_status: Option<u16>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Incident {
    pub id: String,
    pub monitor_id: String,
    pub name: String,
    pub url: String,
    pub error: String,
    pub started_at: i64,
    pub resolved_at: Option<i64>,
    #[serde(default)]
    pub notified_down: bool,
    #[serde(default)]
    pub notified_up: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventRow {
    #[serde(default)]
    pub t: i64,
    #[serde(default)]
    pub site: String,
    /// "pageview" | "event"
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub referrer: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub medium: String,
    #[serde(default)]
    pub campaign: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub browser: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub visitor: String,
    #[serde(default)]
    pub session: String,
    #[serde(default)]
    pub duration_ms: i64,
}

fn default_kind() -> String {
    "pageview".to_string()
}

impl EventRow {
    /// Sanitize + clamp an inbound payload before storage.
    pub fn sanitize(mut self) -> Self {
        if self.t <= 0 {
            self.t = now_ms();
        }
        self.site = self.site.trim().to_lowercase();
        if self.kind != "event" {
            self.kind = "pageview".to_string();
        }
        if self.name.is_empty() {
            self.name = self.path.clone();
        }
        self.name = clamp_str(&self.name, 300);
        self.path = clamp_str(&self.path, 300);
        self.referrer = clamp_str(&self.referrer, 500);
        self.source = clamp_str(&self.source, 120);
        self.medium = clamp_str(&self.medium, 60);
        self.campaign = clamp_str(&self.campaign, 120);
        self.device = clamp_str(&self.device, 20);
        self.browser = clamp_str(&self.browser, 40);
        self.os = clamp_str(&self.os, 40);
        self.country = clamp_str(&self.country, 60);
        self.lang = clamp_str(&self.lang, 12);
        self.visitor = clamp_str(&self.visitor, 64);
        self.session = clamp_str(&self.session, 64);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRow {
    pub t: i64,
    pub monitor_id: String,
    pub ok: bool,
    pub status: Option<u16>,
    pub latency_ms: Option<u64>,
    pub ping_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub collector_port: u16,
    pub collector_enabled: bool,
    pub notifications_enabled: bool,
    pub notify_on_recovery: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "system".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            collector_port: 3415,
            collector_enabled: true,
            notifications_enabled: true,
            notify_on_recovery: true,
            theme: "system".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub collector_port: Option<u16>,
    pub collector_enabled: Option<bool>,
    pub notifications_enabled: Option<bool>,
    pub notify_on_recovery: Option<bool>,
    pub theme: Option<String>,
}

// ---------- store ----------

const MAX_EVENT_LINES: usize = 200_000;
const MAX_CHECK_LINES: usize = 100_000;
const TRIM_THRESHOLD_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        Self { dir }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn read_doc<T: for<'de> Deserialize<'de> + Default>(&self, name: &str) -> T {
        let path = self.file(name);
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn write_doc<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let path = self.file(name);
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn append_line<T: Serialize>(&self, name: &str, value: &T) -> Result<(), String> {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.file(name))
            .map_err(|e| e.to_string())?;
        let mut line = serde_json::to_string(value).map_err(|e| e.to_string())?;
        line.push('\n');
        f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn tail_lines<T: for<'de> Deserialize<'de>>(&self, name: &str, max: usize) -> Vec<T> {
        let text = match fs::read_to_string(self.file(name)) {
            Ok(t) => t,
            Err(_) => return vec![],
        };
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(max);
        lines[start..]
            .iter()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    fn trim_log(&self, name: &str, max: usize) {
        let path = self.file(name);
        let big = fs::metadata(&path).map(|m| m.len() > TRIM_THRESHOLD_BYTES).unwrap_or(false);
        if !big {
            return;
        }
        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return,
        };
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let start = lines.len().saturating_sub(max);
        let kept = lines[start..].join("\n");
        let _ = fs::write(&path, if kept.is_empty() { String::new() } else { kept + "\n" });
    }

    // ---- sites ----
    pub fn list_sites(&self) -> Vec<Site> {
        self.read_doc("sites.json")
    }

    pub fn add_site(&self, domain: &str, name: &str, timezone: &str) -> Result<Site, String> {
        let mut clean = domain.trim().to_lowercase();
        for prefix in ["https://", "http://"] {
            if let Some(rest) = clean.strip_prefix(prefix) {
                clean = rest.to_string();
            }
        }
        if let Some(idx) = clean.find('/') {
            clean.truncate(idx);
        }
        if clean.is_empty() {
            return Err("Domain is required.".to_string());
        }
        let mut sites = self.list_sites();
        if sites.iter().any(|s| s.domain == clean) {
            return Err("That site is already tracked.".to_string());
        }
        let site = Site {
            id: uid("site"),
            name: if name.trim().is_empty() { clean.clone() } else { name.trim().to_string() },
            domain: clean,
            timezone: if timezone.is_empty() { "UTC".to_string() } else { timezone.to_string() },
            created_at: now_ms(),
        };
        sites.push(site.clone());
        self.write_doc("sites.json", &sites)?;
        Ok(site)
    }

    pub fn remove_site(&self, id: &str) -> Result<Vec<Site>, String> {
        let sites: Vec<Site> = self.list_sites().into_iter().filter(|s| s.id != id).collect();
        self.write_doc("sites.json", &sites)?;
        Ok(sites)
    }

    // ---- events ----
    pub fn record_event(&self, evt: EventRow) -> Result<(), String> {
        let row = evt.sanitize();
        if row.site.is_empty() {
            return Err("missing site".to_string());
        }
        self.append_line("events.jsonl", &row)?;
        self.trim_log("events.jsonl", MAX_EVENT_LINES);
        Ok(())
    }

    pub fn load_events(&self, site: Option<&str>, days: i64) -> Vec<EventRow> {
        let since = now_ms() - days * 24 * 3600 * 1000;
        self.tail_lines::<EventRow>("events.jsonl", MAX_EVENT_LINES)
            .into_iter()
            .filter(|e| e.t >= since && site.map(|s| e.site == s).unwrap_or(true))
            .collect()
    }

    // ---- monitors ----
    pub fn list_monitors(&self) -> Vec<Monitor> {
        self.read_doc("monitors.json")
    }

    pub fn add_monitor(
        &self,
        name: &str,
        url: &str,
        interval_sec: u64,
        timeout_ms: u64,
        expect_status: u16,
    ) -> Result<Monitor, String> {
        let target = url.trim().to_string();
        let lower = target.to_lowercase();
        if !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err("URL must start with http:// or https://".to_string());
        }
        let mut monitors = self.list_monitors();
        let monitor = Monitor {
            id: uid("mon"),
            name: if name.trim().is_empty() { target.clone() } else { name.trim().to_string() },
            url: target,
            interval_sec: interval_sec.clamp(15, 3600),
            timeout_ms: timeout_ms.clamp(2000, 60000),
            expect_status,
            enabled: true,
            status: "unknown".to_string(),
            last_check_at: None,
            last_latency_ms: None,
            last_ping_ms: None,
            last_error: None,
            consecutive_fails: 0,
            created_at: now_ms(),
        };
        monitors.push(monitor.clone());
        self.write_doc("monitors.json", &monitors)?;
        Ok(monitor)
    }

    pub fn update_monitor_status(
        &self,
        id: &str,
        status: &str,
        latency_ms: Option<u64>,
        ping_ms: Option<u64>,
        error: Option<String>,
        fails: u32,
    ) -> Result<Option<Monitor>, String> {
        let mut monitors = self.list_monitors();
        let found = monitors.iter_mut().find(|m| m.id == id);
        if let Some(m) = found {
            m.status = status.to_string();
            m.last_check_at = Some(now_ms());
            m.last_latency_ms = latency_ms;
            m.last_ping_ms = ping_ms;
            m.last_error = error;
            m.consecutive_fails = fails;
            let out = m.clone();
            self.write_doc("monitors.json", &monitors)?;
            Ok(Some(out))
        } else {
            Ok(None)
        }
    }

    pub fn patch_monitor(&self, id: &str, patch: &MonitorPatch) -> Result<Monitor, String> {
        let mut monitors = self.list_monitors();
        let m = monitors.iter_mut().find(|m| m.id == id).ok_or("Monitor not found.")?;
        if let Some(v) = &patch.name {
            if !v.trim().is_empty() {
                m.name = v.trim().to_string();
            }
        }
        if let Some(v) = &patch.url {
            let lower = v.trim().to_lowercase();
            if !(lower.starts_with("http://") || lower.starts_with("https://")) {
                return Err("URL must start with http:// or https://".to_string());
            }
            m.url = v.trim().to_string();
        }
        if let Some(v) = patch.interval_sec {
            m.interval_sec = v.clamp(15, 3600);
        }
        if let Some(v) = patch.timeout_ms {
            m.timeout_ms = v.clamp(2000, 60000);
        }
        if let Some(v) = patch.expect_status {
            m.expect_status = v;
        }
        if let Some(v) = patch.enabled {
            m.enabled = v;
        }
        let out = m.clone();
        self.write_doc("monitors.json", &monitors)?;
        Ok(out)
    }

    pub fn remove_monitor(&self, id: &str) -> Result<Vec<Monitor>, String> {
        let monitors: Vec<Monitor> = self.list_monitors().into_iter().filter(|m| m.id != id).collect();
        self.write_doc("monitors.json", &monitors)?;
        let incidents: Vec<Incident> = self.list_incidents().into_iter().filter(|i| i.monitor_id != id).collect();
        self.write_doc("incidents.json", &incidents)?;
        Ok(monitors)
    }

    pub fn record_check(&self, check: &CheckRow) -> Result<(), String> {
        self.append_line("checks.jsonl", check)?;
        self.trim_log("checks.jsonl", MAX_CHECK_LINES);
        Ok(())
    }

    pub fn load_checks(&self, monitor_id: Option<&str>, hours: i64) -> Vec<CheckRow> {
        let since = now_ms() - hours * 3600 * 1000;
        self.tail_lines::<CheckRow>("checks.jsonl", MAX_CHECK_LINES)
            .into_iter()
            .filter(|c| c.t >= since && monitor_id.map(|m| c.monitor_id == m).unwrap_or(true))
            .collect()
    }

    // ---- incidents ----
    pub fn list_incidents(&self) -> Vec<Incident> {
        self.read_doc("incidents.json")
    }

    pub fn open_incident(&self, monitor_id: &str, name: &str, url: &str, error: &str) -> Result<Incident, String> {
        let mut incidents = self.list_incidents();
        if let Some(open) = incidents.iter().find(|i| i.monitor_id == monitor_id && i.resolved_at.is_none()) {
            return Ok(open.clone());
        }
        let incident = Incident {
            id: uid("inc"),
            monitor_id: monitor_id.to_string(),
            name: name.to_string(),
            url: url.to_string(),
            error: clamp_str(error, 300),
            started_at: now_ms(),
            resolved_at: None,
            notified_down: false,
            notified_up: false,
        };
        incidents.insert(0, incident.clone());
        incidents.truncate(500);
        self.write_doc("incidents.json", &incidents)?;
        Ok(incident)
    }

    pub fn resolve_incident(&self, monitor_id: &str) -> Result<Option<Incident>, String> {
        let mut incidents = self.list_incidents();
        let open = incidents.iter_mut().find(|i| i.monitor_id == monitor_id && i.resolved_at.is_none());
        match open {
            Some(i) => {
                i.resolved_at = Some(now_ms());
                let out = i.clone();
                self.write_doc("incidents.json", &incidents)?;
                Ok(Some(out))
            }
            None => Ok(None),
        }
    }

    pub fn mark_notified(&self, incident_id: &str, field: &str) -> Result<(), String> {
        let mut incidents = self.list_incidents();
        if let Some(i) = incidents.iter_mut().find(|i| i.id == incident_id) {
            match field {
                "down" => i.notified_down = true,
                "up" => i.notified_up = true,
                _ => {}
            }
            self.write_doc("incidents.json", &incidents)?;
        }
        Ok(())
    }

    // ---- settings ----
    pub fn get_settings(&self) -> Settings {
        let stored: Option<Settings> = fs::read_to_string(self.file("settings.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());
        stored.unwrap_or_default()
    }

    pub fn save_settings(&self, patch: &SettingsPatch) -> Result<Settings, String> {
        let mut next = self.get_settings();
        if let Some(v) = patch.collector_port {
            next.collector_port = v;
        }
        if let Some(v) = patch.collector_enabled {
            next.collector_enabled = v;
        }
        if let Some(v) = patch.notifications_enabled {
            next.notifications_enabled = v;
        }
        if let Some(v) = patch.notify_on_recovery {
            next.notify_on_recovery = v;
        }
        if let Some(v) = &patch.theme {
            next.theme = v.clone();
        }
        self.write_doc("settings.json", &next)?;
        Ok(next)
    }

    // ---- maintenance ----
    pub fn reset_all(&self) {
        for name in ["sites.json", "monitors.json", "incidents.json", "settings.json", "events.jsonl", "checks.jsonl"] {
            let _ = fs::remove_file(self.file(name));
        }
    }

    #[cfg(test)]
    pub fn dir(&self) -> &std::path::Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store(tag: &str) -> Store {
        let dir = std::env::temp_dir().join(format!("wadev-test-{tag}-{}", now_ms()));
        let _ = fs::remove_dir_all(&dir);
        Store::new(dir)
    }

    #[test]
    fn sites_roundtrip_and_duplicates_rejected() {
        let store = test_store("sites");
        let site = store.add_site("Example.COM/blog", "My Site", "").unwrap();
        assert_eq!(site.domain, "example.com");
        assert!(store.add_site("example.com", "", "").is_err());
        assert_eq!(store.list_sites().len(), 1);
        store.remove_site(&site.id).unwrap();
        assert!(store.list_sites().is_empty());
        let _ = fs::remove_dir_all(store.dir());
    }

    #[test]
    fn events_filter_by_site_and_window() {
        let store = test_store("events");
        store.record_event(EventRow { site: "a.com".into(), path: "/".into(), ..Default::default() }).unwrap();
        assert!(store.record_event(EventRow::default()).is_err());
        assert_eq!(store.load_events(Some("a.com"), 30).len(), 1);
        assert!(store.load_events(Some("b.com"), 30).is_empty());
        let _ = fs::remove_dir_all(store.dir());
    }

    #[test]
    fn monitors_and_incidents_lifecycle() {
        let store = test_store("monitors");
        assert!(store.add_monitor("x", "notaurl", 60, 10000, 200).is_err());
        let m = store.add_monitor("", "https://example.com", 5, 100, 200).unwrap();
        assert_eq!(m.interval_sec, 15); // clamped
        assert_eq!(m.name, "https://example.com");
        let inc = store.open_incident(&m.id, &m.name, &m.url, "boom").unwrap();
        assert!(inc.resolved_at.is_none());
        let same = store.open_incident(&m.id, &m.name, &m.url, "boom").unwrap();
        assert_eq!(same.id, inc.id); // idempotent while open
        let resolved = store.resolve_incident(&m.id).unwrap().unwrap();
        assert!(resolved.resolved_at.is_some());
        store.mark_notified(&inc.id, "down").unwrap();
        assert!(store.list_incidents()[0].notified_down);
        let _ = fs::remove_dir_all(store.dir());
    }

    #[test]
    fn settings_merge_with_defaults() {
        let store = test_store("settings");
        let s = store.get_settings();
        assert_eq!(s.collector_port, 3415);
        let next = store.save_settings(&SettingsPatch { collector_port: Some(8080), ..Default::default() }).unwrap();
        assert_eq!(next.collector_port, 8080);
        assert!(next.notifications_enabled);
        let _ = fs::remove_dir_all(store.dir());
    }
}

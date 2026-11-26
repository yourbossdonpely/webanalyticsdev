//! Web Analytics Dev by Don PELY — analytics aggregation.
//!
//! Pure functions over stored events: totals, timeseries, top-N breakdowns,
//! realtime visitors. All bucketing is UTC in v1.

use crate::store::{now_ms, EventRow};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

const DAY_MS: i64 = 24 * 3600 * 1000;

/// Days since Unix epoch (UTC) for a millisecond timestamp.
fn day_index(ms: i64) -> i64 {
    ms.div_euclid(DAY_MS)
}

/// Howard Hinnant's days-to-civil-date algorithm.
fn days_to_ymd(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y } as i32, m, d)
}

#[allow(dead_code)]
fn day_key(ms: i64) -> String {
    let (y, m, d) = days_to_ymd(day_index(ms));
    format!("{y:04}-{m:02}-{d:02}")
}

/// JS `getDay()` convention: 0 = Sunday. 1970-01-01 was a Thursday (4).
fn weekday(ms: i64) -> usize {
    (day_index(ms) + 4).rem_euclid(7) as usize
}

fn hour(ms: i64) -> usize {
    (ms.div_euclid(3_600_000).rem_euclid(24)) as usize
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CountRow {
    pub name: String,
    pub count: usize,
}

fn count_by(rows: &[&EventRow], key: fn(&EventRow) -> &str, fallback: &str, limit: usize) -> Vec<CountRow> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for r in rows {
        let v = key(r).trim();
        let name = if v.is_empty() { fallback } else { v }.to_string();
        *map.entry(name).or_insert(0) += 1;
    }
    let mut out: Vec<CountRow> = map.into_iter().map(|(name, count)| CountRow { name, count }).collect();
    out.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    out.truncate(limit);
    out
}

fn unique_count(rows: &[&EventRow], key: fn(&EventRow) -> &str) -> usize {
    let mut set: HashSet<&str> = HashSet::new();
    for r in rows {
        let v = key(r);
        if !v.is_empty() {
            set.insert(v);
        }
    }
    set.len()
}

fn label<'a>(v: &'a str, fallback: &'a str) -> &'a str {
    if v.trim().is_empty() {
        fallback
    } else {
        v
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub visitors: usize,
    pub sessions: usize,
    pub pageviews: usize,
    pub events: usize,
    pub visitors_change: f64,
    pub sessions_change: f64,
    pub pageviews_change: f64,
    pub events_change: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayPoint {
    pub date: String,
    pub visitors: usize,
    pub pageviews: usize,
    pub events: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub totals: Totals,
    pub timeseries: Vec<DayPoint>,
    /// 7 (Sun..Sat) × 24 hourly buckets.
    pub heatmap: [[u32; 24]; 7],
    pub pages: Vec<CountRow>,
    pub sources: Vec<CountRow>,
    pub mediums: Vec<CountRow>,
    pub campaigns: Vec<CountRow>,
    pub custom_events: Vec<CountRow>,
    pub devices: Vec<CountRow>,
    pub browsers: Vec<CountRow>,
    pub countries: Vec<CountRow>,
}

fn change(cur: usize, prev: usize) -> f64 {
    if prev == 0 {
        return if cur > 0 { 100.0 } else { 0.0 };
    }
    ((cur as f64 - prev as f64) / prev as f64 * 1000.0).round() / 10.0
}

fn summarize(rows: &[&EventRow]) -> (usize, usize, usize, usize) {
    let pv = rows.iter().filter(|r| r.kind != "event").count();
    (
        unique_count(rows, |r| &r.visitor),
        unique_count(rows, |r| &r.session),
        pv,
        rows.len() - pv,
    )
}

pub fn query_dashboard(events: &[EventRow], days: i64) -> DashboardData {
    let days = days.clamp(1, 365);
    let now = now_ms();
    let start = now - days * DAY_MS;
    let prev_start = start - days * DAY_MS;

    let cur: Vec<&EventRow> = events.iter().filter(|e| e.t >= start).collect();
    let prev: Vec<&EventRow> = events.iter().filter(|e| e.t >= prev_start && e.t < start).collect();
    let (v, s, p, e) = summarize(&cur);
    let (pv_, ss, pp, ee) = summarize(&prev);

    // Timeseries buckets for the last `days` days.
    let today_idx = day_index(now);
    let mut buckets: Vec<(String, HashSet<&str>, usize, usize)> = Vec::with_capacity(days as usize);
    for i in (0..days).rev() {
        let idx = today_idx - i;
        let (y, m, d) = days_to_ymd(idx);
        buckets.push((format!("{y:04}-{m:02}-{d:02}"), HashSet::new(), 0, 0));
    }
    for r in &cur {
        let pos = day_index(r.t) - (today_idx - days + 1);
        if pos >= 0 && (pos as usize) < buckets.len() {
            let b = &mut buckets[pos as usize];
            if !r.visitor.is_empty() {
                b.1.insert(r.visitor.as_str());
            }
            if r.kind == "event" {
                b.3 += 1;
            } else {
                b.2 += 1;
            }
        }
    }
    let timeseries: Vec<DayPoint> = buckets
        .into_iter()
        .map(|(date, visitors, pageviews, evts)| DayPoint { date, visitors: visitors.len(), pageviews, events: evts })
        .collect();

    let mut heatmap = [[0u32; 24]; 7];
    for r in &cur {
        heatmap[weekday(r.t)][hour(r.t)] += 1;
    }

    let pageviews: Vec<&EventRow> = cur.iter().copied().filter(|r| r.kind != "event").collect();
    let customs: Vec<&EventRow> = cur.iter().copied().filter(|r| r.kind == "event").collect();
    let campaigned: Vec<&EventRow> = cur.iter().copied().filter(|r| !r.campaign.trim().is_empty()).collect();

    DashboardData {
        totals: Totals {
            visitors: v,
            sessions: s,
            pageviews: p,
            events: e,
            visitors_change: change(v, pv_),
            sessions_change: change(s, ss),
            pageviews_change: change(p, pp),
            events_change: change(e, ee),
        },
        timeseries,
        heatmap,
        pages: count_by(&pageviews, |r| label(&r.path, "/"), "/", 12),
        sources: count_by(&cur, |r| label(&r.source, "direct"), "direct", 12),
        mediums: count_by(&cur, |r| label(&r.medium, "none"), "none", 8),
        campaigns: count_by(&campaigned, |r| r.campaign.as_str(), "(none)", 8),
        custom_events: count_by(&customs, |r| label(&r.name, "(event)"), "(event)", 12),
        devices: count_by(&cur, |r| label(&r.device, "unknown"), "unknown", 6),
        browsers: count_by(&cur, |r| label(&r.browser, "unknown"), "unknown", 8),
        countries: count_by(&cur, |r| label(&r.country, "unknown"), "unknown", 12),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentHit {
    pub t: i64,
    pub site: String,
    pub path: String,
    pub kind: String,
    pub name: String,
    pub country: String,
    pub device: String,
    pub browser: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RealtimeData {
    pub total: usize,
    pub by_site: Vec<CountRow>,
    pub recent: Vec<RecentHit>,
}

pub fn query_realtime(events: &[EventRow], minutes: i64) -> RealtimeData {
    let since = now_ms() - minutes * 60 * 1000;
    let live: Vec<&EventRow> = events.iter().filter(|e| e.t >= since).collect();
    let mut recent: Vec<RecentHit> = live
        .iter()
        .rev()
        .take(20)
        .map(|r| RecentHit {
            t: r.t,
            site: r.site.clone(),
            path: r.path.clone(),
            kind: r.kind.clone(),
            name: r.name.clone(),
            country: r.country.clone(),
            device: r.device.clone(),
            browser: r.browser.clone(),
        })
        .collect();
    // newest first already (rev); keep stable
    recent.sort_by(|a, b| b.t.cmp(&a.t));
    RealtimeData {
        total: unique_count(&live, |r| &r.visitor),
        by_site: count_by(&live, |r| r.site.as_str(), "(none)", 12),
        recent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::EventRow;

    #[test]
    fn civil_date_roundtrip_spot_checks() {
        assert_eq!(day_key(0), "1970-01-01");
        assert_eq!(day_key(1_700_000_000_000), "2023-11-14");
        assert_eq!(weekday(0), 4); // Thursday
        assert_eq!(weekday(1_700_000_000_000), 2); // Tuesday
        assert_eq!(hour(3_600_000), 1);
    }

    fn evt(site: &str, visitor: &str, session: &str, days_ago: i64, kind: &str, path: &str) -> EventRow {
        EventRow {
            t: now_ms() - days_ago * DAY_MS,
            site: site.into(),
            kind: kind.into(),
            name: path.into(),
            path: path.into(),
            visitor: visitor.into(),
            session: session.into(),
            source: "google".into(),
            medium: "organic".into(),
            device: "desktop".into(),
            browser: "Chrome".into(),
            country: "Germany".into(),
            ..Default::default()
        }
    }

    #[test]
    fn dashboard_totals_and_change() {
        let events = vec![
            evt("a.com", "v1", "s1", 1, "pageview", "/"),
            evt("a.com", "v1", "s1", 1, "pageview", "/pricing"),
            evt("a.com", "v2", "s2", 1, "event", "Signup"),
            evt("a.com", "v9", "s9", 40, "pageview", "/"), // previous window
        ];
        let d = query_dashboard(&events, 30);
        assert_eq!(d.totals.visitors, 2);
        assert_eq!(d.totals.pageviews, 2);
        assert_eq!(d.totals.events, 1);
        assert_eq!(d.timeseries.len(), 30);
        assert!(d.totals.visitors_change > 0.0);
        assert_eq!(d.pages[0].count, 1);
        assert_eq!(d.custom_events[0].name, "Signup");
        let heat_total: u32 = d.heatmap.iter().flat_map(|r| r.iter()).sum();
        assert_eq!(heat_total, 3);
    }

    #[test]
    fn realtime_counts_last_minutes() {
        let mut e1 = evt("a.com", "v1", "s1", 0, "pageview", "/");
        e1.t = now_ms() - 60_000;
        let e2 = evt("a.com", "v2", "s2", 2, "pageview", "/");
        let rt = query_realtime(&[e1, e2], 5);
        assert_eq!(rt.total, 1);
        assert_eq!(rt.recent.len(), 1);
    }
}

//! Web Analytics Dev by Don PELY — synthetic demo data generator.
//!
//! Seeds fictional sites, 30 days of plausible traffic and sample monitors
//! so the dashboard is alive on first launch. Deterministic output.

use crate::store::{now_ms, EventRow, Store};

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f64 {
        // mulberry32
        self.0 = self.0.wrapping_add(0x6D2B79F5);
        let mut t = self.0 ^ (self.0 >> 15);
        t = t.wrapping_mul(1 | self.0);
        t = t.wrapping_add(t.wrapping_mul(t ^ (t >> 7) ^ 61 | t) ^ t);
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }

    fn pick<'a, T>(&mut self, arr: &'a [T]) -> &'a T {
        &arr[(self.next() * arr.len() as f64).floor() as usize]
    }

    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.next() * (hi - lo)
    }
}

const PATHS: [&str; 10] = ["/", "/pricing", "/blog", "/blog/hello-world", "/docs", "/docs/quickstart", "/about", "/contact", "/features", "/download"];
const SOURCES: [(&str, &str); 9] = [
    ("google", "organic"), ("google", "organic"), ("bing", "organic"),
    ("github.com", "referral"), ("x.com", "social"), ("linkedin.com", "social"),
    ("newsletter", "email"), ("", "direct"), ("", "direct"),
];
const DEVICES: [(&str, &str, &str); 6] = [
    ("desktop", "Chrome", "Windows"), ("desktop", "Chrome", "macOS"), ("desktop", "Firefox", "Linux"),
    ("mobile", "Safari", "iOS"), ("mobile", "Chrome", "Android"), ("tablet", "Safari", "iPadOS"),
];
const COUNTRIES: [&str; 10] = ["United States", "United Kingdom", "Germany", "France", "Nigeria", "Canada", "India", "Netherlands", "Brazil", "Australia"];
const CAMPAIGNS: [&str; 5] = ["", "", "", "launch", "spring-sale"];
const EVENTS: [&str; 4] = ["Signup", "Purchase", "Download", "Contact"];

pub fn seed_demo(store: &Store) {
    let mut rng = Rng(2026_1126);
    let now = now_ms();
    const DAY_MS: i64 = 24 * 3600 * 1000;

    let defs = [
        ("demo-shop.example.com", "Demo Shop", 1.0),
        ("demo-blog.example.com", "Demo Blog", 0.55),
        ("demo-docs.example.com", "Demo Docs", 0.3),
    ];
    let mut sites: Vec<String> = Vec::new();
    for (domain, name, _scale) in defs {
        let label = format!("🟣 {name} (demo)");
        match store.add_site(domain, &label, "UTC") {
            Ok(s) => sites.push(s.domain),
            Err(_) => sites.push(domain.to_string()),
        }
    }

    let days = 30;
    let visits_per_day = 120.0;
    for d in (1..=days).rev() {
        let day_start = now - d as i64 * DAY_MS;
        let weekday = (day_start.div_euclid(DAY_MS) + 4).rem_euclid(7);
        let weekend_boost = if weekday == 0 || weekday == 6 { 1.25 } else { 1.0 };
        for (site, _name, scale) in &defs {
            let count = (visits_per_day * scale * weekend_boost * rng.range(0.7, 1.3)).round() as usize;
            for _ in 0..count {
                let hour = (24.0 * rng.next().powf(0.7)) as i64; // daytime skew
                let t = day_start + hour * 3_600_000 + (rng.next() * 3_600_000.0) as i64;
                let (source, medium) = rng.pick(&SOURCES);
                let (device, browser, os) = rng.pick(&DEVICES);
                let visitor = format!("{site}:demo-{}", (rng.next() * (count.max(20) as f64 * 0.6)) as usize);
                let session = format!("{visitor}:{}", t / 1_800_000);
                let entry = EventRow {
                    t,
                    site: site.to_string(),
                    kind: "pageview".to_string(),
                    path: rng.pick(&PATHS).to_string(),
                    referrer: if source.contains('.') { format!("https://{source}/") } else { String::new() },
                    source: source.to_string(),
                    medium: medium.to_string(),
                    campaign: rng.pick(&CAMPAIGNS).to_string(),
                    device: device.to_string(),
                    browser: browser.to_string(),
                    os: os.to_string(),
                    country: rng.pick(&COUNTRIES).to_string(),
                    lang: "en".to_string(),
                    visitor: visitor.clone(),
                    session: session.clone(),
                    ..Default::default()
                };
                let _ = store.record_event(entry.clone());
                let r = rng.next();
                if r > 0.55 {
                    let mut second = entry.clone();
                    second.t = t + 45_000;
                    second.path = rng.pick(&PATHS).to_string();
                    second.duration_ms = 45_000;
                    let _ = store.record_event(second);
                }
                if r > 0.86 {
                    let mut custom = entry.clone();
                    custom.t = t + 90_000;
                    custom.kind = "event".to_string();
                    custom.name = rng.pick(&EVENTS).to_string();
                    let _ = store.record_event(custom);
                }
            }
        }
    }

    // Live visitors right now.
    for i in 0..8 {
        let (site, _, _) = rng.pick(&defs);
        let (source, medium) = rng.pick(&SOURCES);
        let (device, browser, os) = rng.pick(&DEVICES);
        let _ = store.record_event(EventRow {
            t: now - (rng.next() * 4.0 * 60_000.0) as i64,
            site: site.to_string(),
            kind: "pageview".to_string(),
            path: rng.pick(&PATHS).to_string(),
            source: source.to_string(),
            medium: medium.to_string(),
            device: device.to_string(),
            browser: browser.to_string(),
            os: os.to_string(),
            country: rng.pick(&COUNTRIES).to_string(),
            lang: "en".to_string(),
            visitor: format!("{site}:live-{i}"),
            session: format!("{site}:live-{i}"),
            ..Default::default()
        });
    }

    // Sample monitors.
    for (name, url) in [("Demo Shop (demo)", "https://example.com/"), ("Demo Blog (demo)", "https://example.org/")] {
        if !store.list_monitors().iter().any(|m| m.url == url) {
            let _ = store.add_monitor(name, url, 60, 10000, 200);
        }
    }
}

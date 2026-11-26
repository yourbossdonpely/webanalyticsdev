<div align="center">

<img src="assets/icon-1024.png" width="128" height="128" alt="Web Analytics Dev logo" />

# Web Analytics Dev

**by Don PELY**

Lightweight, cookieless web analytics plus website uptime monitoring —
one fast standalone desktop app for Windows, macOS and Linux.

[![CI](https://github.com/yourbossdonpely/webanalyticsdev/actions/workflows/ci.yml/badge.svg)](https://github.com/yourbossdonpely/webanalyticsdev/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/yourbossdonpely/webanalyticsdev)](https://github.com/yourbossdonpely/webanalyticsdev/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-teal.svg)](LICENSE)

[Download](#-download) · [Features](#-features) · [Quick start](#-quick-start) · [How it works](#-how-it-works) · [Develop](#-develop)

</div>

---

## 📥 Download

Get the latest standalone installer from the
**[Releases page](https://github.com/yourbossdonpely/webanalyticsdev/releases)**:

| OS | Files |
|---|---|
| Windows | `Web-Analytics-Dev_*_x64-setup.exe` (installer), `*.msi` |
| macOS | `Web-Analytics-Dev_*.dmg` |
| Linux | `*.deb` (Debian/Ubuntu), `*.rpm` (Fedora/RHEL), `*.AppImage` |
| Source | `webanalyticsdev-<version>.zip` + `Source code (zip)` |

No admin rights, no cloud account, no Docker — just install and run.

## ✨ Features

**📊 Cookieless web analytics**

- Tiny (~4 KB) tracker snippet, no cookies, no IP storage, no cross-day identifiers
- Overview dashboard: visitors, pageviews, sessions and custom events with change vs the previous period
- Day-by-day chart, top pages, sources, channels, UTM campaigns, custom events, devices, browsers, countries
- Weekday × hour activity heatmap
- Click any row to filter the whole dashboard; custom events via `wadev("Signup")` or `class="wadev-event-Signup"`
- SPA-aware: page changes in React/Vue/etc. apps are tracked automatically

**⚡ Realtime**

- Live visitor count in the sidebar, live visitors by site, latest-hits feed

**💓 Website monitoring with desktop alerts**

- HTTP(S) checks plus ICMP ping on a schedule you set (15s – 1h per site)
- A site is DOWN after 2 consecutive failures, UP again on the first success
- Native desktop notification the moment a site goes down — and when it recovers
- 24h uptime %, latency, per-check history timeline, incident log
- Monitoring keeps running in the system tray when the window is closed

**🖥️ Local-first desktop app**

- Standalone native app (Tauri 2 Rust core + React UI) — small, fast, offline-friendly
- All data stays on your machine as plain JSON; system-tray resident; light/dark themes

## 🚀 Quick start

1. **Install** the app from the [Releases page](https://github.com/yourbossdonpely/webanalyticsdev/releases) and open it.
   It starts with fictional demo sites so every screen is alive immediately.
2. **Track a site:** go to **Sites → Add site**, then paste the snippet it shows you just before
   `</head>` on your website:
   ```html
   <script defer src="http://127.0.0.1:3415/t.js" data-site="example.com"></script>
   ```
   Custom events: `wadev("Signup")`, or add `class="wadev-event-Signup"` to any element.
   > The tracker posts to the collector built into the app while it runs. Point it at your
   > machine's LAN address (same port) to collect from other devices on your network.
3. **Monitor a site:** go to **Uptime → Add website**, enter the URL and interval.
   You'll get a desktop notification if it goes down — try the **Send test notification**
   button in **Settings** first.
4. **Clean up:** **Settings → Erase all data** wipes the demo content and starts fresh.

## 🧭 How it works

```
your site ── tracker (t.js) ──POST /e──▶ collector (127.0.0.1:3415, in-app)
                                                    │
dashboard (React) ──invoke──▶ Rust core ──▶ JSON store (app data dir)
                                    │
                                    ├──▶ uptime scheduler ──▶ HTTP + ping ──▶ incidents + desktop alerts
                                    └──▶ tray icon (monitoring continues when the window is closed)
```

- **Privacy:** the tracker sends no cookies and the collector stores no IPs or user agents —
  only the page, referrer/source, device class, browser family and a daily-rotating anonymous id.
- **Storage:** `sites.json`, `events.jsonl`, `monitors.json`, `checks.jsonl`, `incidents.json`,
  `settings.json` under the app data directory. Delete the app data dir (or use Erase all data)
  and everything is gone.
- **Time buckets** are UTC in v1.

## 🛠️ Develop

Requirements: [Node.js 20+](https://nodejs.org/), [Rust stable](https://rustup.rs/)
(Windows: MSVC Build Tools; Linux: webkit2gtk/gtk3 system packages, installed by CI).

```sh
git clone https://github.com/yourbossdonpely/webanalyticsdev.git
cd webanalyticsdev
npm install
npx tauri dev        # run with hot reload
```

| Command | What it does |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust unit tests (store, analytics) |
| `npm run build` | Type-check + production frontend build |
| `npx tauri build` | Full release bundles for your OS (exe/msi, dmg, deb/rpm) |
| `npm run icons` | Re-render the logo PNG and all platform icons |

### Layout

```
tracker/t.js            the cookieless tracker (embedded into the app binary)
src/                    React + TypeScript dashboard (Vite)
src-tauri/              Rust backend: store, analytics, collector, monitor, tray
  icons/                generated platform icons (ico, icns, png sets)
assets/                 logo.svg + rendered icon-1024.png
scripts/                icon renderer (pure Node, zero native deps)
.github/workflows/      ci.yml (test+build) and release.yml (tagged multi-OS bundles)
```

## 🗺️ Roadmap

- Public share links and scheduled email digests
- Per-site timezones and local-time heatmap
- Remote collector mode (collect from production without the desktop app running)
- Auto-updates with signed bundles

## 📄 License

MIT — see [LICENSE](LICENSE). © 2026 Don PELY.

Inspired by the self-hosted analytics idea (notably
[quick-web-analytics](https://github.com/Zesty0wl/quick-web-analytics)); all code here is an
original implementation.

# Changelog

All notable changes to Web Analytics Dev by Don PELY.

## [0.1.1] - 2026-11-26

### Changed

- Official "Analytics Pulse" app icon by Don PELY now used for all desktop
  installers (Windows, macOS, Linux), the README and the tray icon.

## [0.1.0] - 2026-11-26

First public release.

### Added

- Cookieless web analytics: ~4 KB tracker (`t.js`), local collector, overview dashboard
  (visitors, pageviews, sessions, custom events, day chart, breakdowns, heatmap, filters)
- Realtime view: live visitor count, live by site, latest-hits feed
- Website monitoring: HTTP(S) + ICMP ping checks per site, 2-fail down / 1-success up logic,
  native desktop down/recovery notifications, uptime %, check history, incident log
- Standalone desktop app (Tauri 2 + React) with system tray; monitoring continues when closed
- Light/dark themes, demo data on first run, erase/reset, JSON file storage
- Installers: Windows setup `.exe` + `.msi`, macOS `.dmg`, Linux `.deb` / `.rpm` / `.AppImage`

// Web Analytics Dev by Don PELY — shared frontend types (mirror the Rust serde models).

export interface Site {
  id: string;
  domain: string;
  name: string;
  timezone: string;
  createdAt: number;
}

export interface Monitor {
  id: string;
  name: string;
  url: string;
  intervalSec: number;
  timeoutMs: number;
  expectStatus: number;
  enabled: boolean;
  status: 'up' | 'down' | 'unknown';
  lastCheckAt: number | null;
  lastLatencyMs: number | null;
  lastPingMs: number | null;
  lastError: string | null;
  consecutiveFails: number;
  createdAt: number;
}

export interface MonitorPatch {
  name?: string;
  url?: string;
  intervalSec?: number;
  timeoutMs?: number;
  expectStatus?: number;
  enabled?: boolean;
}

export interface Incident {
  id: string;
  monitorId: string;
  name: string;
  url: string;
  error: string;
  startedAt: number;
  resolvedAt: number | null;
  notifiedDown: boolean;
  notifiedUp: boolean;
}

export interface CheckRow {
  t: number;
  monitorId: string;
  ok: boolean;
  status: number | null;
  latencyMs: number | null;
  pingMs: number | null;
  error: string | null;
}

export interface MonitorStats {
  id: string;
  checks: number;
  up: number;
  uptimePct: number | null;
  p50Ms: number | null;
}

export interface Settings {
  collectorPort: number;
  collectorEnabled: boolean;
  notificationsEnabled: boolean;
  notifyOnRecovery: boolean;
  theme: string;
}

export interface SettingsPatch {
  collectorPort?: number;
  collectorEnabled?: boolean;
  notificationsEnabled?: boolean;
  notifyOnRecovery?: boolean;
  theme?: string;
}

export interface CountRow {
  name: string;
  count: number;
}

export interface DayPoint {
  date: string;
  visitors: number;
  pageviews: number;
  events: number;
}

export interface Totals {
  visitors: number;
  sessions: number;
  pageviews: number;
  events: number;
  visitorsChange: number;
  sessionsChange: number;
  pageviewsChange: number;
  eventsChange: number;
}

export interface DashboardData {
  totals: Totals;
  timeseries: DayPoint[];
  heatmap: number[][];
  pages: CountRow[];
  sources: CountRow[];
  mediums: CountRow[];
  campaigns: CountRow[];
  customEvents: CountRow[];
  devices: CountRow[];
  browsers: CountRow[];
  countries: CountRow[];
}

export interface RecentHit {
  t: number;
  site: string;
  path: string;
  kind: string;
  name: string;
  country: string;
  device: string;
  browser: string;
}

export interface RealtimeData {
  total: number;
  bySite: CountRow[];
  recent: RecentHit[];
}

export interface CollectorStatus {
  running: boolean;
  port: number | null;
}

export interface AppInfo {
  name: string;
  version: string;
  author: string;
  platform: string;
}

export interface AnalyticsFilter {
  field: string;
  value: string;
}

export interface MonitorsUpdated {
  monitors: Monitor[];
  incidents: Incident[];
}

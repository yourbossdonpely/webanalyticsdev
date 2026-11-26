// Web Analytics Dev by Don PELY — typed Tauri command wrappers.
import { invoke } from '@tauri-apps/api/core';
import type {
  AnalyticsFilter, AppInfo, CheckRow, CollectorStatus, DashboardData, Incident,
  Monitor, MonitorPatch, MonitorStats, RealtimeData, Settings, SettingsPatch, Site,
} from './types';

export const api = {
  listSites: () => invoke<Site[]>('sites_list'),
  addSite: (domain: string, name: string, timezone = 'UTC') =>
    invoke<Site>('sites_add', { domain, name, timezone }),
  removeSite: (id: string) => invoke<Site[]>('sites_remove', { id }),
  snippet: (domain: string) => invoke<string>('sites_snippet', { domain }),

  query: (site: string | null, days: number, filter: AnalyticsFilter | null) =>
    invoke<DashboardData>('analytics_query', { site, days, filter }),
  realtime: () => invoke<RealtimeData>('analytics_realtime'),

  listMonitors: () => invoke<Monitor[]>('monitors_list'),
  addMonitor: (name: string, url: string, intervalSec: number, timeoutMs: number) =>
    invoke<Monitor>('monitors_add', { name, url, intervalSec, timeoutMs }),
  updateMonitor: (id: string, patch: MonitorPatch) =>
    invoke<Monitor>('monitors_update', { id, patch }),
  removeMonitor: (id: string) => invoke<Monitor[]>('monitors_remove', { id }),
  checkNow: (id: string | null) => invoke<boolean>('monitors_check', { id }),
  monitorStats: (hours: number) => invoke<MonitorStats[]>('monitors_stats', { hours }),
  monitorChecks: (monitorId: string | null, hours: number) =>
    invoke<CheckRow[]>('monitors_checks', { monitorId, hours }),
  listIncidents: () => invoke<Incident[]>('incidents_list'),

  getSettings: () => invoke<Settings>('settings_get'),
  saveSettings: (patch: SettingsPatch) => invoke<Settings>('settings_save', { patch }),
  testNotification: () => invoke<boolean>('notify_test'),
  collectorStatus: () => invoke<CollectorStatus>('collector_status'),
  seedDemo: () => invoke<boolean>('demo_seed'),
  resetAll: () => invoke<boolean>('data_reset'),
  appInfo: () => invoke<AppInfo>('app_info'),
};

export function errMsg(err: unknown): string {
  return typeof err === 'string' ? err : err instanceof Error ? err.message : 'Request failed.';
}

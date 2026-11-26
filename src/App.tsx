// Web Analytics Dev by Don PELY — app shell: nav, topbar, toasts, modal, live events.
import { useCallback, useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, errMsg } from './lib/api';
import type {
  AnalyticsFilter, AppInfo, CollectorStatus, Incident, Monitor,
  MonitorsUpdated, Settings, Site,
} from './lib/types';
import Overview from './views/Overview';
import Realtime from './views/Realtime';
import Uptime from './views/Uptime';
import Sites from './views/Sites';
import SettingsView from './views/Settings';
import './styles.css';

type View = 'overview' | 'realtime' | 'uptime' | 'sites' | 'settings';

interface Toast {
  id: number;
  msg: string;
  isError?: boolean;
}

let toastId = 0;

export default function App() {
  const [view, setView] = useState<View>('overview');
  const [sites, setSites] = useState<Site[]>([]);
  const [site, setSite] = useState('');
  const [days, setDays] = useState(30);
  const [filter, setFilter] = useState<AnalyticsFilter | null>(null);
  const [monitors, setMonitors] = useState<Monitor[]>([]);
  const [incidents, setIncidents] = useState<Incident[]>([]);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [collector, setCollector] = useState<CollectorStatus | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [live, setLive] = useState(0);
  const [light, setLight] = useState(false);
  const [modal, setModal] = useState<React.ReactNode | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);

  const toast = useCallback((msg: string, isError?: boolean) => {
    const id = ++toastId;
    setToasts((t) => [...t, { id, msg, isError }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 4500);
  }, []);

  const closeModal = useCallback(() => setModal(null), []);

  const refreshAll = useCallback(() => {
    api.listSites().then(setSites).catch((e) => toast(`Load sites: ${errMsg(e)}`, true));
    api.listMonitors().then(setMonitors).catch(() => {});
    api.listIncidents().then(setIncidents).catch(() => {});
    api.getSettings().then(setSettings).catch(() => {});
    api.collectorStatus().then(setCollector).catch(() => {});
  }, [toast]);

  useEffect(() => {
    api.appInfo().then(setInfo).catch(() => {});
    refreshAll();
    const unlisten = listen<MonitorsUpdated>('monitors-updated', (event) => {
      setMonitors(event.payload.monitors);
      setIncidents(event.payload.incidents);
    });
    const liveTimer = setInterval(() => {
      api.realtime().then((rt) => setLive(rt.total)).catch(() => {});
    }, 15000);
    api.realtime().then((rt) => setLive(rt.total)).catch(() => {});
    return () => {
      unlisten.then((off) => off()).catch(() => {});
      clearInterval(liveTimer);
    };
  }, [refreshAll]);

  useEffect(() => {
    document.body.classList.toggle('light', light);
  }, [light]);

  const down = monitors.filter((m) => m.enabled && m.status === 'down').length;
  const siteName = sites.find((s) => s.domain === site)?.name ?? site;

  return (
    <div id="app">
      <aside id="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <svg viewBox="0 0 32 32" width="26" height="26">
              <rect x="1" y="1" width="30" height="30" rx="8" fill="#1b2540" />
              <path
                d="M6 20 L12 20 L15 12 L19 24 L22 16 L26 16"
                stroke="#38e1c6"
                strokeWidth="2.6"
                fill="none"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <circle cx="26" cy="16" r="2.6" fill="#8b7bff" />
            </svg>
          </span>
          <span className="brand-text">
            <strong>Web Analytics Dev</strong>
            <small>by Don PELY</small>
          </span>
        </div>
        <nav id="nav">
          <button data-view="overview" className={view === 'overview' ? 'active' : ''} onClick={() => setView('overview')}>
            📊 Overview
          </button>
          <button data-view="realtime" className={view === 'realtime' ? 'active' : ''} onClick={() => setView('realtime')}>
            ⚡ Realtime
          </button>
          <button data-view="uptime" className={view === 'uptime' ? 'active' : ''} onClick={() => setView('uptime')}>
            💓 Uptime {down > 0 && <span className="badge">{down}</span>}
          </button>
          <button data-view="sites" className={view === 'sites' ? 'active' : ''} onClick={() => setView('sites')}>
            🌐 Sites
          </button>
          <button data-view="settings" className={view === 'settings' ? 'active' : ''} onClick={() => setView('settings')}>
            ⚙️ Settings
          </button>
        </nav>
        <div className="sidebar-foot">
          <div className="live-pill">
            <span className="dot" />
            <span>{live}</span> live
          </div>
          <small>
            {info ? `v${info.version} · ${info.platform}` : '…'}
          </small>
        </div>
      </aside>
      <main id="main">
        <header id="topbar">
          <div className="site-picker">
            <label htmlFor="site-select">Site</label>
            <select id="site-select" value={site} onChange={(e) => setSite(e.target.value)}>
              <option value="">All sites</option>
              {sites.map((s) => (
                <option key={s.id} value={s.domain}>
                  {s.name}
                </option>
              ))}
            </select>
            <select id="range-select" aria-label="Date range" value={days} onChange={(e) => setDays(Number(e.target.value))}>
              <option value={7}>Last 7 days</option>
              <option value={30}>Last 30 days</option>
              <option value={90}>Last 90 days</option>
            </select>
          </div>
          <div className="top-actions">
            <button
              className="btn ghost"
              onClick={() => {
                setView('sites');
              }}
            >
              ＋ Install tracker
            </button>
            <button className="btn ghost" title="Toggle theme" onClick={() => setLight((l) => !l)}>
              {light ? '🌞' : '🌙'}
            </button>
          </div>
        </header>
        {filter && (
          <div id="filter-bar">
            Filtered: <strong>{`${filter.field} = ${filter.value}`}</strong>{' '}
            <button className="btn xs" onClick={() => setFilter(null)}>
              ✕ clear
            </button>
          </div>
        )}
        <section id="view">
          {view === 'overview' && (
            <Overview site={site} siteName={siteName} days={days} filter={filter} onFilter={setFilter} toast={toast} />
          )}
          {view === 'realtime' && <Realtime site={site} toast={toast} />}
          {view === 'uptime' && (
            <Uptime
              monitors={monitors}
              incidents={incidents}
              onChanged={refreshAll}
              toast={toast}
              openModal={setModal}
              closeModal={closeModal}
            />
          )}
          {view === 'sites' && (
            <Sites
              sites={sites}
              onSites={(s) => {
                setSites(s);
                refreshAll();
              }}
              onOpenSite={(domain) => {
                setSite(domain);
                setView('overview');
              }}
              toast={toast}
              openModal={setModal}
              closeModal={closeModal}
            />
          )}
          {view === 'settings' && settings && collector && info && (
            <SettingsView settings={settings} status={collector} info={info} onChanged={refreshAll} toast={toast} />
          )}
        </section>
      </main>

      {modal && (
        <div
          id="modal-root"
          onClick={(e) => {
            if ((e.target as HTMLElement).id === 'modal-root') closeModal();
          }}
        >
          <div id="modal-card">{modal}</div>
        </div>
      )}
      <div id="toast-root">
        {toasts.map((t) => (
          <div key={t.id} className={`toast${t.isError ? ' error' : ''}`}>
            {t.msg}
          </div>
        ))}
      </div>
    </div>
  );
}

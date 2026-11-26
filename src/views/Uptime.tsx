// Web Analytics Dev by Don PELY — website uptime monitoring view.
import { useEffect, useState } from 'react';
import { api, errMsg } from '../lib/api';
import type { CheckRow, Incident, Monitor, MonitorStats } from '../lib/types';
import { fmtTime } from '../components/charts';

function uptimeClass(pct: number | null): string {
  if (pct == null) return '';
  if (pct >= 99) return '';
  if (pct >= 95) return 'warn';
  return 'bad';
}

export default function Uptime({
  monitors,
  incidents,
  onChanged,
  toast,
  openModal,
  closeModal,
}: {
  monitors: Monitor[];
  incidents: Incident[];
  onChanged: () => void;
  toast: (msg: string, isError?: boolean) => void;
  openModal: (content: React.ReactNode) => void;
  closeModal: () => void;
}) {
  const [stats, setStats] = useState<MonitorStats[]>([]);

  const loadStats = () => {
    api.monitorStats(24).then(setStats).catch(() => {});
  };

  useEffect(loadStats, [monitors]);

  const statById = Object.fromEntries(stats.map((s) => [s.id, s]));
  const open = incidents.filter((i) => !i.resolvedAt);
  const past = incidents.filter((i) => i.resolvedAt).slice(0, 10);

  const showAdd = () => {
    let name = '';
    let url = '';
    let intervalSec = 60;
    let timeoutMs = 10000;
    openModal(
      <>
        <h2>Monitor a website</h2>
        <p style={{ color: 'var(--muted)' }}>
          We ping it and load the page on a schedule. If it goes down, you get a desktop notification.
        </p>
        <div className="form-row">
          <label>
            Display name
            <input placeholder="My blog" onChange={(e) => (name = e.target.value)} />
          </label>
        </div>
        <div className="form-row">
          <label>
            URL
            <input placeholder="https://example.com" style={{ width: '100%' }} onChange={(e) => (url = e.target.value)} />
          </label>
        </div>
        <div className="form-row">
          <label>
            Check every (seconds)
            <input type="number" defaultValue={60} min={15} max={3600} onChange={(e) => (intervalSec = Number(e.target.value))} />
          </label>
          <label>
            Timeout (ms)
            <input type="number" defaultValue={10000} min={2000} max={60000} onChange={(e) => (timeoutMs = Number(e.target.value))} />
          </label>
        </div>
        <div className="modal-actions">
          <button className="btn ghost" onClick={closeModal}>
            Cancel
          </button>
          <button
            className="btn"
            onClick={() => {
              api
                .addMonitor(name, url, intervalSec, timeoutMs)
                .then((m) => {
                  closeModal();
                  toast(`Monitoring ${m.name} — first check running…`);
                  onChanged();
                })
                .catch((e) => toast(`Add monitor: ${errMsg(e)}`, true));
            }}
          >
            Start monitoring
          </button>
        </div>
      </>,
    );
  };

  const showHistory = (m: Monitor) => {
    api
      .monitorChecks(m.id, 72)
      .then((checks: CheckRow[]) => {
        const ticks = checks.slice(-120);
        openModal(
          <>
            <h2>{m.name}</h2>
            <p style={{ color: 'var(--muted)' }}>
              {m.url} · last {checks.length} checks (72h)
            </p>
            <div className="tick-row">
              {ticks.length === 0 ? (
                <p>No checks yet.</p>
              ) : (
                ticks.map((c, i) => (
                  <span
                    key={i}
                    className={`tick ${c.ok ? 'ok' : 'fail'}`}
                    title={`${new Date(c.t).toLocaleString()} — ${c.ok ? `${c.latencyMs}ms` : c.error}`}
                  />
                ))
              )}
            </div>
            <div className="modal-actions">
              <button className="btn ghost" onClick={closeModal}>
                Close
              </button>
            </div>
          </>,
        );
      })
      .catch((e) => toast(`Load history: ${errMsg(e)}`, true));
  };

  return (
    <>
      <div className="view-title">
        <h2>💓 Website monitoring</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <button
            className="btn ghost"
            onClick={() => {
              toast('Checking all websites…');
              api
                .checkNow(null)
                .then(onChanged)
                .catch((e) => toast(`Check all: ${errMsg(e)}`, true));
            }}
          >
            Check all now
          </button>
          <button className="btn" onClick={showAdd}>
            ＋ Add website
          </button>
        </div>
      </div>
      {open.length > 0 && (
        <div className="card" style={{ borderColor: 'var(--down)' }}>
          <h3>
            🔴 {open.length} site{open.length > 1 ? 's' : ''} down now
          </h3>
          {open.map((i) => (
            <div className="incident" key={i.id}>
              <strong>{i.name}</strong> — {i.url}
              <br />
              <small>
                Down since {new Date(i.startedAt).toLocaleString()} · {i.error}
              </small>
            </div>
          ))}
        </div>
      )}
      <div className={`card ${open.length > 0 ? 'mt' : ''}`}>
        <h3>Monitored websites</h3>
        {monitors.length === 0 && (
          <p style={{ color: 'var(--muted)' }}>
            No websites yet. Add one to get ping + HTTP checks with desktop alerts when it goes down.
          </p>
        )}
        {monitors.map((m) => {
          const s = statById[m.id];
          const pct = s?.uptimePct == null ? null : Math.round(s.uptimePct * 100) / 100;
          return (
            <div className="mon" key={m.id}>
              <span className={`status-dot ${m.enabled ? m.status : 'unknown'}`} title={m.status} />
              <div className="mon-info">
                <strong>
                  {m.name} {!m.enabled && <small>(paused)</small>}
                </strong>
                <small>
                  {m.url} · every {m.intervalSec}s · HTTP{m.lastPingMs != null ? ` · ping ${m.lastPingMs}ms` : ''}
                </small>
                {m.lastError && (
                  <>
                    <br />
                    <small style={{ color: 'var(--down)' }}>Last error: {m.lastError}</small>
                  </>
                )}
              </div>
              <div className="mon-meta">
                <div className={`uptime-pct ${uptimeClass(pct)}`}>{pct == null ? '—' : `${pct}%`}</div>
                <div>
                  24h · {m.lastLatencyMs != null ? `${m.lastLatencyMs}ms` : '—'} · {fmtTime(m.lastCheckAt)}
                </div>
                <div style={{ marginTop: 6, display: 'flex', gap: 6, justifyContent: 'flex-end' }}>
                  <button
                    className="btn xs ghost"
                    onClick={() => {
                      toast(`Checking ${m.name}…`);
                      api
                        .checkNow(m.id)
                        .then(onChanged)
                        .catch((e) => toast(`Check site: ${errMsg(e)}`, true));
                    }}
                  >
                    Check
                  </button>
                  <button
                    className="btn xs ghost"
                    onClick={() =>
                      api
                        .updateMonitor(m.id, { enabled: !m.enabled })
                        .then(onChanged)
                        .catch((e) => toast(`Update monitor: ${errMsg(e)}`, true))
                    }
                  >
                    {m.enabled ? 'Pause' : 'Resume'}
                  </button>
                  <button className="btn xs ghost" onClick={() => showHistory(m)}>
                    History
                  </button>
                  <button
                    className="btn xs danger"
                    onClick={() => {
                      if (confirm(`Stop monitoring ${m.name}?`)) {
                        api
                          .removeMonitor(m.id)
                          .then(onChanged)
                          .catch((e) => toast(`Remove monitor: ${errMsg(e)}`, true));
                      }
                    }}
                  >
                    ✕
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
      {past.length > 0 && (
        <div className="card mt">
          <h3>Recent incidents</h3>
          {past.map((i) => (
            <div className="incident resolved" key={i.id}>
              <strong>{i.name}</strong> — {i.url}
              <br />
              <small>
                {new Date(i.startedAt).toLocaleString()} →{' '}
                {i.resolvedAt ? new Date(i.resolvedAt).toLocaleString() : '…'} · {i.error}
              </small>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

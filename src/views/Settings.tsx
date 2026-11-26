// Web Analytics Dev by Don PELY — settings view.
import { useState } from 'react';
import { api, errMsg } from '../lib/api';
import type { AppInfo, CollectorStatus, Settings } from '../lib/types';

export default function SettingsView({
  settings,
  status,
  info,
  onChanged,
  toast,
}: {
  settings: Settings;
  status: CollectorStatus;
  info: AppInfo;
  onChanged: () => void;
  toast: (msg: string, isError?: boolean) => void;
}) {
  const [port, setPort] = useState(settings.collectorPort);
  const [collectorOn, setCollectorOn] = useState(settings.collectorEnabled);

  return (
    <>
      <div className="view-title">
        <h2>⚙️ Settings</h2>
      </div>
      <div className="grid two">
        <div className="card">
          <h3>Notifications</h3>
          <div className="switch-row">
            <span>Desktop alerts when a site goes down</span>
            <input
              type="checkbox"
              defaultChecked={settings.notificationsEnabled}
              onChange={(e) =>
                api
                  .saveSettings({ notificationsEnabled: e.target.checked })
                  .then(onChanged)
                  .catch((err) => toast(`Save settings: ${errMsg(err)}`, true))
              }
            />
          </div>
          <div className="switch-row">
            <span>Also notify on recovery</span>
            <input
              type="checkbox"
              defaultChecked={settings.notifyOnRecovery}
              onChange={(e) =>
                api
                  .saveSettings({ notifyOnRecovery: e.target.checked })
                  .then(onChanged)
                  .catch((err) => toast(`Save settings: ${errMsg(err)}`, true))
              }
            />
          </div>
          <div style={{ marginTop: 12 }}>
            <button
              className="btn ghost"
              onClick={() =>
                api.testNotification().catch((e) => toast(`Test notification: ${errMsg(e)}`, true))
              }
            >
              Send test notification
            </button>
          </div>
        </div>
        <div className="card">
          <h3>Local collector</h3>
          <div className="switch-row">
            <span>
              Collector server{' '}
              {status.running ? `🟢 running on port ${status.port}` : '🔴 stopped'}
            </span>
            <input type="checkbox" checked={collectorOn} onChange={(e) => setCollectorOn(e.target.checked)} />
          </div>
          <div className="form-row" style={{ marginTop: 12 }}>
            <label>
              Port
              <input
                type="number"
                value={port}
                min={1024}
                max={65535}
                onChange={(e) => setPort(Number(e.target.value))}
              />
            </label>
          </div>
          <p style={{ color: 'var(--muted)', fontSize: 12 }}>
            The tracker posts pageviews here. Keep the app running (tray icon) to collect data.
          </p>
          <button
            className="btn ghost"
            onClick={() =>
              api
                .saveSettings({ collectorEnabled: collectorOn, collectorPort: port })
                .then(() => {
                  toast('Collector settings saved.');
                  onChanged();
                })
                .catch((e) => toast(`Save collector: ${errMsg(e)}`, true))
            }
          >
            Save collector settings
          </button>
        </div>
      </div>
      <div className="grid two mt">
        <div className="card">
          <h3>Data</h3>
          <p style={{ color: 'var(--muted)', fontSize: 13 }}>
            Reload the fictional demo sites, or wipe everything and start clean.
          </p>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              className="btn ghost"
              onClick={() =>
                api
                  .seedDemo()
                  .then(() => {
                    toast('Demo data loaded.');
                    onChanged();
                  })
                  .catch((e) => toast(`Seed demo: ${errMsg(e)}`, true))
              }
            >
              Reload demo data
            </button>
            <button
              className="btn danger"
              onClick={() => {
                if (!confirm('Erase ALL sites, events, monitors and incidents? This cannot be undone.')) return;
                api
                  .resetAll()
                  .then(onChanged)
                  .catch((e) => toast(`Reset data: ${errMsg(e)}`, true));
              }}
            >
              Erase all data
            </button>
          </div>
        </div>
        <div className="card">
          <h3>About</h3>
          <p>
            <strong>Web Analytics Dev</strong> v{info.version}
            <br />
            <small style={{ color: 'var(--muted)' }}>
              by {info.author} · MIT License
              <br />
              Tauri desktop app · {info.platform}
            </small>
          </p>
        </div>
      </div>
    </>
  );
}

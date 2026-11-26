// Web Analytics Dev by Don PELY — realtime visitors view.
import { useEffect, useState } from 'react';
import { api, errMsg } from '../lib/api';
import type { DashboardData, RealtimeData } from '../lib/types';
import { RowsTable } from '../components/charts';

export default function Realtime({
  site,
  toast,
}: {
  site: string;
  toast: (msg: string, isError?: boolean) => void;
}) {
  const [rt, setRt] = useState<RealtimeData | null>(null);
  const [today, setToday] = useState<DashboardData | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = () => {
      api
        .realtime()
        .then((d) => {
          if (!cancelled) setRt(d);
        })
        .catch((e) => toast(`Load realtime: ${errMsg(e)}`, true));
      api
        .query(site || null, 1, null)
        .then((d) => {
          if (!cancelled) setToday(d);
        })
        .catch(() => {});
    };
    load();
    const timer = setInterval(load, 15000);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [site]);

  if (!rt) return <p style={{ color: 'var(--muted)' }}>Loading…</p>;
  return (
    <>
      <div className="view-title">
        <h2>⚡ Realtime</h2>
        <span className="live-pill">
          <span className="dot" />
          {rt.total} online now
        </span>
      </div>
      <div className="grid two">
        <div className="card">
          <h3>Live visitors by site</h3>
          <RowsTable rows={rt.bySite} total={rt.total} />
        </div>
        <div className="card">
          <h3>Today at a glance</h3>
          {today ? (
            <RowsTable
              rows={[
                { name: 'Visitors', count: today.totals.visitors },
                { name: 'Pageviews', count: today.totals.pageviews },
                { name: 'Custom events', count: today.totals.events },
              ]}
              total={today.totals.pageviews + today.totals.events}
            />
          ) : (
            <p style={{ color: 'var(--muted)' }}>Loading…</p>
          )}
        </div>
      </div>
      <div className="card mt">
        <h3>Latest hits</h3>
        {rt.recent.length === 0 ? (
          <p style={{ color: 'var(--muted)' }}>
            No visits in the last 5 minutes. Add the tracker to a site to see live traffic.
          </p>
        ) : (
          rt.recent.map((r, i) => (
            <div className="feed-item" key={`${r.t}-${i}`}>
              <time>{new Date(r.t).toLocaleTimeString()}</time>
              <code>{r.kind === 'event' ? `⚡ ${r.name}` : r.path}</code>
              <span style={{ color: 'var(--muted)' }}>
                {r.site} · {r.country} · {r.device} {r.browser}
              </span>
            </div>
          ))
        )}
      </div>
    </>
  );
}

// Web Analytics Dev by Don PELY — traffic overview view.
import { useEffect, useState } from 'react';
import { api, errMsg } from '../lib/api';
import type { AnalyticsFilter, DashboardData } from '../lib/types';
import { Delta, Heatmap, LineChart, RowsTable } from '../components/charts';

export default function Overview({
  site,
  siteName,
  days,
  filter,
  onFilter,
  toast,
}: {
  site: string;
  siteName: string;
  days: number;
  filter: AnalyticsFilter | null;
  onFilter: (f: AnalyticsFilter | null) => void;
  toast: (msg: string, isError?: boolean) => void;
}) {
  const [data, setData] = useState<DashboardData | null>(null);

  useEffect(() => {
    let cancelled = false;
    setData(null);
    api
      .query(site || null, days, filter)
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((e) => toast(`Load analytics: ${errMsg(e)}`, true));
    return () => {
      cancelled = true;
    };
  }, [site, days, filter]);

  if (!data) return <p style={{ color: 'var(--muted)' }}>Loading…</p>;
  const t = data.totals;
  const totalHits = t.pageviews + t.events;

  return (
    <>
      <div className="view-title">
        <h2>
          {site ? siteName : 'All sites'}{' '}
          <small style={{ color: 'var(--muted)', fontWeight: 400 }}>· last {days} days</small>
        </h2>
      </div>
      <div className="grid kpis">
        <div className="card kpi">
          <h3>Visitors</h3>
          <div className="value">{t.visitors.toLocaleString()}</div>
          <Delta pct={t.visitorsChange} />
        </div>
        <div className="card kpi">
          <h3>Pageviews</h3>
          <div className="value">{t.pageviews.toLocaleString()}</div>
          <Delta pct={t.pageviewsChange} />
        </div>
        <div className="card kpi">
          <h3>Sessions</h3>
          <div className="value">{t.sessions.toLocaleString()}</div>
          <Delta pct={t.sessionsChange} />
        </div>
        <div className="card kpi">
          <h3>Custom events</h3>
          <div className="value">{t.events.toLocaleString()}</div>
          <Delta pct={t.eventsChange} />
        </div>
      </div>
      <div className="card mt">
        <h3>Visitors per day</h3>
        <LineChart series={data.timeseries} dataKey="visitors" />
      </div>
      <div className="grid two mt">
        <div className="card">
          <h3>Top pages</h3>
          <RowsTable rows={data.pages} total={t.pageviews} onFilter={(v) => onFilter({ field: 'path', value: v })} />
        </div>
        <div className="card">
          <h3>Sources</h3>
          <RowsTable rows={data.sources} total={totalHits} onFilter={(v) => onFilter({ field: 'source', value: v })} />
        </div>
      </div>
      <div className="grid two mt">
        <div className="card">
          <h3>Channels</h3>
          <RowsTable rows={data.mediums} total={totalHits} onFilter={(v) => onFilter({ field: 'medium', value: v })} />
        </div>
        <div className="card">
          <h3>Custom events</h3>
          <RowsTable rows={data.customEvents} total={t.events} onFilter={(v) => onFilter({ field: 'name', value: v })} />
        </div>
      </div>
      <div className="grid three mt">
        <div className="card">
          <h3>Devices</h3>
          <RowsTable rows={data.devices} total={totalHits} onFilter={(v) => onFilter({ field: 'device', value: v })} />
        </div>
        <div className="card">
          <h3>Browsers</h3>
          <RowsTable rows={data.browsers} total={totalHits} onFilter={(v) => onFilter({ field: 'browser', value: v })} />
        </div>
        <div className="card">
          <h3>Countries</h3>
          <RowsTable rows={data.countries} total={totalHits} onFilter={(v) => onFilter({ field: 'country', value: v })} />
        </div>
      </div>
      <div className="card mt">
        <h3>Weekday × hour activity (UTC)</h3>
        <Heatmap heat={data.heatmap} />
      </div>
    </>
  );
}

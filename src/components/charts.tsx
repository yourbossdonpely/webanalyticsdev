// Web Analytics Dev by Don PELY — hand-rolled SVG charts and tables (no chart lib).
import { Fragment } from 'react';
import type { CountRow, DayPoint } from '../lib/types';

export function Delta({ pct }: { pct: number }) {
  const cls = pct > 0 ? 'up' : pct < 0 ? 'down' : 'flat';
  const arrow = pct > 0 ? '▲' : pct < 0 ? '▼' : '●';
  return (
    <div className={`delta ${cls}`}>
      {arrow} {pct}% vs prev
    </div>
  );
}

export function RowsTable({
  rows,
  total,
  onFilter,
}: {
  rows: CountRow[];
  total: number;
  onFilter?: (value: string) => void;
}) {
  if (rows.length === 0) return <p style={{ color: 'var(--muted)' }}>No data yet.</p>;
  const max = Math.max(...rows.map((r) => r.count), 1);
  const denom = Math.max(total, 1);
  return (
    <table className="rows">
      <tbody>
        {rows.map((r) => (
          <tr
            key={r.name}
            data-filter={onFilter ? '1' : undefined}
            onClick={onFilter ? () => onFilter(r.name) : undefined}
          >
            <td>{r.name}</td>
            <td className="bar-cell">
              <div className="bar" style={{ width: `${Math.round((r.count / max) * 100)}%` }} />
            </td>
            <td className="num">
              {r.count} · {Math.round((r.count / denom) * 100)}%
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function LineChart({ series, dataKey }: { series: DayPoint[]; dataKey: 'visitors' | 'pageviews' | 'events' }) {
  if (series.length === 0) return <p style={{ color: 'var(--muted)' }}>No data yet.</p>;
  const W = 720;
  const H = 180;
  const P = 8;
  const vals = series.map((d) => d[dataKey]);
  const max = Math.max(...vals, 1);
  const step = (W - P * 2) / Math.max(series.length - 1, 1);
  const pts = series.map(
    (d, i) => [P + i * step, H - P - (d[dataKey] / max) * (H - P * 2)] as const,
  );
  const line = pts.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ');
  const area = `${line} L${(W - P).toFixed(1)},${H - P} L${P},${H - P} Z`;
  const stride = Math.ceil(series.length / 8);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} style={{ width: '100%', height: 'auto' }} role="img" aria-label="traffic chart">
      <defs>
        <linearGradient id="wad-area" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#38e1c6" stopOpacity="0.45" />
          <stop offset="1" stopColor="#38e1c6" stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={area} fill="url(#wad-area)" />
      <path d={line} fill="none" stroke="#38e1c6" strokeWidth="2.5" strokeLinejoin="round" />
      {pts.map((p, i) =>
        i % stride === 0 ? (
          <circle key={i} cx={p[0]} cy={p[1]} r="3" fill="#8b7bff">
            <title>
              {series[i].date}: {series[i][dataKey]}
            </title>
          </circle>
        ) : null,
      )}
      <text x={W - P} y={H - 2} textAnchor="end" fontSize="10" fill="var(--muted)">
        {series[0].date} → {series[series.length - 1].date}
      </text>
    </svg>
  );
}

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

export function Heatmap({ heat }: { heat: number[][] }) {
  const max = Math.max(...heat.flat(), 1);
  return (
    <div className="heatmap">
      <span />
      {Array.from({ length: 24 }, (_, h) => (
        <span key={h} style={{ textAlign: 'center' }}>
          {h % 6 === 0 ? `${h}h` : ''}
        </span>
      ))}
      {heat.map((row, d) => (
        <Fragment key={`d${d}`}>
          <span>{DAYS[d]}</span>
          {row.map((v, h) => (
            <span
              key={`${d}-${h}`}
              className="hcell"
              style={v === 0 ? undefined : { background: `rgba(56,225,198,${(0.15 + (v / max) * 0.85).toFixed(2)})` }}
              title={`${DAYS[d]} ${h}:00 — ${v} views (UTC)`}
            />
          ))}
        </Fragment>
      ))}
    </div>
  );
}

export function fmtTime(ms: number | null): string {
  if (!ms) return 'never';
  const mins = Math.round((Date.now() - ms) / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const h = Math.round(mins / 60);
  if (h < 24) return `${h}h ago`;
  return new Date(ms).toLocaleString();
}

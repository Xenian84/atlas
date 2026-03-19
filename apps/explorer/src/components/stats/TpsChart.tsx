'use client';
import { useState, useEffect, useCallback } from 'react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer,
} from 'recharts';
import { clientFetch } from '@/lib/api';
import { getNetworkPulse } from '@/lib/atlasRpc';

interface Sample { time: string; ts: number; tps: number; }
interface TpsResponse { samples: Sample[]; }

const Tip = ({ active, payload }: { active?: boolean; payload?: { value: number; name: string }[] }) => {
  if (!active || !payload?.length) return null;
  return (
    <div style={{
      background: 'hsl(var(--card))', border: '1px solid hsl(var(--border-strong))',
      padding: '8px 12px', fontFamily: 'var(--font-mono)', fontSize: 11,
    }}>
      {payload.map(p => (
        <div key={p.name} style={{ color: 'hsl(var(--accent-blue))', marginBottom: 2 }}>
          TPS: <b>{p.value.toLocaleString()}</b>
        </div>
      ))}
    </div>
  );
};

export default function TpsChart() {
  const [data, setData]             = useState<Sample[]>([]);
  const [loading, setLoading]       = useState(true);
  const [liveTps, setLiveTps]       = useState<number>(0);

  const load = useCallback(async () => {
    try {
      // Atlas-native TPS history — sourced directly from indexed tx_store
      const [hist, pulse] = await Promise.all([
        clientFetch<TpsResponse>('/v1/network/tps'),
        getNetworkPulse().catch(() => null),
      ]);
      // Show last 30 completed minutes (current minute excluded server-side)
      const trimmed = hist.samples.slice(-30);
      setData(trimmed);
      if (pulse?.tps_1m) setLiveTps(Math.round(pulse.tps_1m));
      else if (hist.samples.length) setLiveTps(hist.samples[hist.samples.length - 1].tps);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 10_000); return () => clearInterval(iv); }, [load]);

  return (
    <div className="atlas-card" style={{ padding: '20px 20px 14px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 16 }}>
        <div>
          <span className="statlabel">NETWORK TPS — Atlas Indexed</span>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 26, fontWeight: 700, color: 'hsl(var(--accent-blue))', lineHeight: 1, marginTop: 8 }}>
            {loading ? '—' : liveTps.toLocaleString()}
            <span style={{ fontSize: 11, color: 'hsl(var(--foreground-tertiary))', fontWeight: 400, marginLeft: 6 }}>tx/s</span>
          </div>
        </div>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.08em', paddingTop: 4 }}>
          NON-VOTE · CONFIRMED
        </span>
      </div>

      {/* Chart */}
      <div style={{ height: 160 }}>
        {loading ? (
          <div className="skeleton" style={{ height: '100%', width: '100%' }} />
        ) : (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
              <defs>
                <linearGradient id="tpsGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="hsl(217 80% 62%)" stopOpacity={0.3} />
                  <stop offset="95%" stopColor="hsl(217 80% 62%)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(220 14% 12%)" vertical={false} />
              <XAxis
                dataKey="time"
                tick={{ fill: 'hsl(220 9% 38%)', fontSize: 9, fontFamily: 'var(--font-mono)' }}
                tickLine={false} axisLine={false} interval={9}
              />
              <YAxis
                tick={{ fill: 'hsl(220 9% 38%)', fontSize: 9, fontFamily: 'var(--font-mono)' }}
                tickLine={false} axisLine={false} width={36}
                tickFormatter={v => v >= 1000 ? `${(v / 1000).toFixed(0)}k` : String(v)}
              />
              <Tooltip content={<Tip />} />
              <Area
                type="monotone" dataKey="tps"
                stroke="hsl(217 80% 62%)" strokeWidth={1.5}
                fill="url(#tpsGrad)" dot={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Legend */}
      <div style={{ marginTop: 10, display: 'flex', gap: 16, alignItems: 'center' }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-tertiary))' }}>
          <span style={{ width: 7, height: 7, background: 'hsl(217 80% 62%)', display: 'inline-block' }} />
          True TPS (non-vote)
        </span>
        <span style={{ marginLeft: 'auto', fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
          60-min · per-minute bins · updates 10s
        </span>
      </div>
    </div>
  );
}

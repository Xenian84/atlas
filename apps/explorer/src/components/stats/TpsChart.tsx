'use client';
import { useState, useEffect, useCallback } from 'react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer,
} from 'recharts';
import { getPerformanceSamples, type PerformanceSample } from '@/lib/atlasRpc';

interface DataPoint { slot: number; tps: number; voteTps: number; time: string; }

function toDataPoint(s: PerformanceSample, idx: number, arr: PerformanceSample[]): DataPoint {
  const period  = Math.max(s.samplePeriodSecs, 1);
  const tps     = Math.round(s.numTransactions / period);
  const nonVote = s.numNonVoteTransactions ?? 0;
  const voteTps = Math.round((s.numTransactions - nonVote) / period);
  const secsAgo = (arr.length - idx) * period;
  const d       = new Date(Date.now() - secsAgo * 1000);
  return { slot: s.slot, tps, voteTps, time: d.toLocaleTimeString('en', { hour: '2-digit', minute: '2-digit' }) };
}

const Tip = ({ active, payload }: { active?: boolean; payload?: { value: number; name: string }[] }) => {
  if (!active || !payload?.length) return null;
  return (
    <div style={{
      background: 'hsl(var(--card))', border: '1px solid hsl(var(--border-strong))',
      padding: '8px 12px', fontFamily: 'var(--font-mono)', fontSize: 11,
    }}>
      {payload.map(p => (
        <div key={p.name} style={{ color: p.name === 'tps' ? 'hsl(var(--accent-blue))' : 'hsl(var(--accent-purple))', marginBottom: 2 }}>
          {p.name === 'tps' ? 'True TPS' : 'Vote TPS'}: <b>{p.value.toLocaleString()}</b>
        </div>
      ))}
    </div>
  );
};

export default function TpsChart() {
  const [data, setData]             = useState<DataPoint[]>([]);
  const [showVote, setShowVote]     = useState(false);
  const [loading, setLoading]       = useState(true);
  const [currentTps, setCurrentTps] = useState(0);

  const load = useCallback(async () => {
    try {
      const samples = await getPerformanceSamples(60);
      const pts = samples.slice().reverse().map(toDataPoint);
      setData(pts);
      if (pts.length) setCurrentTps(pts[pts.length - 1].tps);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 10_000); return () => clearInterval(iv); }, [load]);

  return (
    <div className="atlas-card" style={{ padding: '20px 20px 14px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 16 }}>
        <div>
          <span className="statlabel">NETWORK TPS</span>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 26, fontWeight: 700, color: 'hsl(var(--accent-blue))', lineHeight: 1, marginTop: 8 }}>
            {loading ? '—' : currentTps.toLocaleString()}
            <span style={{ fontSize: 11, color: 'hsl(var(--foreground-tertiary))', fontWeight: 400, marginLeft: 6 }}>tx/s</span>
          </div>
        </div>
        <button
          onClick={() => setShowVote(v => !v)}
          style={{
            background: showVote ? 'hsla(var(--accent-purple),.15)' : 'hsl(var(--background-secondary))',
            border: `1px solid ${showVote ? 'hsla(var(--accent-purple),.4)' : 'hsl(var(--border-strong))'}`,
            padding: '5px 10px',
            color: showVote ? 'hsl(var(--accent-purple))' : 'hsl(var(--foreground-tertiary))',
            fontFamily: 'var(--font-mono)', fontSize: 9, cursor: 'pointer',
            letterSpacing: '.1em', transition: 'all .15s',
          }}
        >
          {showVote ? '✓ ' : ''}INCLUDE VOTE
        </button>
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
                <linearGradient id="voteGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="hsl(270 60% 65%)" stopOpacity={0.25} />
                  <stop offset="95%" stopColor="hsl(270 60% 65%)" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="hsl(220 14% 12%)" vertical={false} />
              <XAxis dataKey="time" tick={{ fill: 'hsl(220 9% 38%)', fontSize: 9, fontFamily: 'var(--font-mono)' }} tickLine={false} axisLine={false} interval={9} />
              <YAxis tick={{ fill: 'hsl(220 9% 38%)', fontSize: 9, fontFamily: 'var(--font-mono)' }} tickLine={false} axisLine={false} width={38}
                tickFormatter={v => v >= 1000 ? `${(v / 1000).toFixed(0)}k` : String(v)} />
              <Tooltip content={<Tip />} />
              <Area type="monotone" dataKey="tps" stroke="hsl(217 80% 62%)" strokeWidth={1.5} fill="url(#tpsGrad)" dot={false} name="tps" />
              {showVote && <Area type="monotone" dataKey="voteTps" stroke="hsl(270 60% 65%)" strokeWidth={1.5} fill="url(#voteGrad)" dot={false} name="voteTps" />}
            </AreaChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Legend */}
      <div style={{ marginTop: 10, display: 'flex', gap: 16, alignItems: 'center' }}>
        <Dot color="hsl(217 80% 62%)" label="True TPS" />
        {showVote && <Dot color="hsl(270 60% 65%)" label="Vote TPS" />}
        <span style={{ marginLeft: 'auto', fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
          60-sample · updates every 10s
        </span>
      </div>
    </div>
  );
}

function Dot({ color, label }: { color: string; label: string }) {
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-tertiary))' }}>
      <span style={{ width: 7, height: 7, background: color, display: 'inline-block' }} />
      {label}
    </span>
  );
}

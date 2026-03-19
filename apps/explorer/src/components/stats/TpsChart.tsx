'use client';

import { useState, useEffect, useCallback } from 'react';
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer, ReferenceLine,
} from 'recharts';
import { getPerformanceSamples, type PerformanceSample } from '@/lib/atlasRpc';

interface DataPoint {
  slot: number;
  tps: number;
  voteTps: number;
  time: string;
}

function toDataPoint(s: PerformanceSample): DataPoint {
  const period = Math.max(s.samplePeriodSecs, 1);
  const tps    = Math.round(s.numTransactions / period);
  const nonVote = s.numNonVoteTransactions ?? 0;
  const voteTps = Math.round((s.numTransactions - nonVote) / period);
  return {
    slot: s.slot,
    tps,
    voteTps,
    time: new Date(Date.now() - (60 - 0) * 1000).toLocaleTimeString('en', { hour: '2-digit', minute: '2-digit' }),
  };
}

const CustomTooltip = ({ active, payload }: { active?: boolean; payload?: { value: number; name: string }[] }) => {
  if (!active || !payload?.length) return null;
  return (
    <div style={{
      background: 'rgba(10,15,25,0.97)', border: '1px solid rgba(255,255,255,0.1)',
      borderRadius: 6, padding: '8px 12px', fontFamily: 'monospace', fontSize: 11,
    }}>
      {payload.map(p => (
        <div key={p.name} style={{ color: p.name === 'tps' ? '#89b4fa' : '#cba6f7', marginBottom: 2 }}>
          {p.name === 'tps' ? 'True TPS' : 'Vote TPS'}: <b>{p.value.toLocaleString()}</b>
        </div>
      ))}
    </div>
  );
};

export default function TpsChart() {
  const [data, setData]         = useState<DataPoint[]>([]);
  const [showVote, setShowVote] = useState(false);
  const [loading, setLoading]   = useState(true);
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

  useEffect(() => {
    load();
    const iv = setInterval(load, 10_000);
    return () => clearInterval(iv);
  }, [load]);

  return (
    <div style={{
      background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)',
      borderRadius: 10, padding: '20px 22px',
    }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.12em', color: '#45475a', textTransform: 'uppercase', fontFamily: 'monospace' }}>
            NETWORK TPS
          </div>
          <div style={{ fontSize: 28, fontWeight: 700, color: '#89b4fa', fontFamily: 'monospace', lineHeight: 1.2 }}>
            {loading ? '…' : currentTps.toLocaleString()}
            <span style={{ fontSize: 12, color: '#45475a', fontWeight: 400, marginLeft: 6 }}>tx/s</span>
          </div>
        </div>

        <button
          onClick={() => setShowVote(v => !v)}
          style={{
            background: showVote ? 'rgba(203,166,247,0.15)' : 'rgba(255,255,255,0.04)',
            border: `1px solid ${showVote ? 'rgba(203,166,247,0.4)' : 'rgba(255,255,255,0.1)'}`,
            borderRadius: 6, padding: '5px 12px',
            color: showVote ? '#cba6f7' : '#6c7086',
            fontFamily: 'monospace', fontSize: 10, cursor: 'pointer',
            letterSpacing: '0.08em', transition: 'all 0.2s',
          }}
        >
          {showVote ? '✓ ' : ''}Include Vote Txns
        </button>
      </div>

      {/* Chart */}
      <div style={{ height: 180 }}>
        {loading ? (
          <div style={{ height: '100%', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#45475a', fontSize: 12, fontFamily: 'monospace' }}>
            Loading samples…
          </div>
        ) : (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={data} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
              <defs>
                <linearGradient id="tpsGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="#89b4fa" stopOpacity={0.25} />
                  <stop offset="95%" stopColor="#89b4fa" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="voteGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%"  stopColor="#cba6f7" stopOpacity={0.2} />
                  <stop offset="95%" stopColor="#cba6f7" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.04)" vertical={false} />
              <XAxis dataKey="time" tick={{ fill: '#45475a', fontSize: 9, fontFamily: 'monospace' }} tickLine={false} axisLine={false} interval={9} />
              <YAxis tick={{ fill: '#45475a', fontSize: 9, fontFamily: 'monospace' }} tickLine={false} axisLine={false} width={40}
                tickFormatter={v => v >= 1000 ? `${(v/1000).toFixed(0)}k` : String(v)} />
              <Tooltip content={<CustomTooltip />} />
              <Area type="monotone" dataKey="tps" stroke="#89b4fa" strokeWidth={1.5}
                fill="url(#tpsGrad)" dot={false} name="tps" />
              {showVote && (
                <Area type="monotone" dataKey="voteTps" stroke="#cba6f7" strokeWidth={1.5}
                  fill="url(#voteGrad)" dot={false} name="voteTps" />
              )}
            </AreaChart>
          </ResponsiveContainer>
        )}
      </div>

      <div style={{ marginTop: 10, display: 'flex', gap: 16, fontSize: 10, fontFamily: 'monospace', color: '#45475a' }}>
        <LegendDot color="#89b4fa" label="True TPS (excl. vote)" />
        {showVote && <LegendDot color="#cba6f7" label="Vote transactions" />}
        <span style={{ marginLeft: 'auto' }}>60-sample rolling window · updates every 10s</span>
      </div>
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
      <span style={{ width: 8, height: 8, borderRadius: '50%', background: color, display: 'inline-block' }} />
      {label}
    </span>
  );
}

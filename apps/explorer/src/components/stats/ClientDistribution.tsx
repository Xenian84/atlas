'use client';

import { useState, useEffect, useCallback } from 'react';
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid } from 'recharts';
import { getClusterNodes, type ClusterNode } from '@/lib/atlasRpc';

const CLIENT_COLORS: Record<string, string> = {
  'tachyon':   '#00e5ff',
  'agave':     '#89b4fa',
  'jito':      '#cba6f7',
  'firedancer':'#a6e3a1',
  'other':     '#45475a',
};

function detectClient(version: string | null): string {
  if (!version) return 'other';
  const v = version.toLowerCase();
  if (v.includes('tachyon')) return 'tachyon';
  if (v.includes('jito'))    return 'jito';
  if (v.includes('fire') || v.includes('fd')) return 'firedancer';
  if (v.includes('agave'))   return 'agave';
  return 'agave'; // default for X1
}

interface ClientShare { name: string; count: number; pct: number; color: string; }
interface VersionShare { version: string; count: number; }

function analyze(nodes: ClusterNode[]): { clients: ClientShare[]; versions: VersionShare[] } {
  const clientMap: Record<string, number> = {};
  const versionMap: Record<string, number> = {};

  for (const n of nodes) {
    const client = detectClient(n.version);
    clientMap[client] = (clientMap[client] ?? 0) + 1;

    const ver = n.version ?? 'unknown';
    versionMap[ver] = (versionMap[ver] ?? 0) + 1;
  }

  const total = nodes.length || 1;
  const clients = Object.entries(clientMap)
    .sort((a, b) => b[1] - a[1])
    .map(([name, count]) => ({
      name: name.charAt(0).toUpperCase() + name.slice(1),
      count,
      pct: Math.round((count / total) * 1000) / 10,
      color: CLIENT_COLORS[name] ?? '#45475a',
    }));

  const versions = Object.entries(versionMap)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 8)
    .map(([version, count]) => ({ version, count }));

  return { clients, versions };
}

const PieTooltip = ({ active, payload }: { active?: boolean; payload?: {name: string; value: number; payload: ClientShare}[] }) => {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload;
  return (
    <div style={{ background: 'rgba(10,15,25,0.97)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: 6, padding: '7px 12px', fontFamily: 'monospace', fontSize: 11 }}>
      <span style={{ color: d.color }}>{d.name}</span>
      <span style={{ color: '#6c7086', marginLeft: 8 }}>{d.count} nodes ({d.pct}%)</span>
    </div>
  );
};

export default function ClientDistribution() {
  const [clients, setClients]   = useState<ClientShare[]>([]);
  const [versions, setVersions] = useState<VersionShare[]>([]);
  const [total, setTotal]       = useState(0);
  const [loading, setLoading]   = useState(true);

  const load = useCallback(async () => {
    try {
      const nodes = await getClusterNodes();
      const { clients: c, versions: v } = analyze(nodes);
      setClients(c);
      setVersions(v);
      setTotal(nodes.length);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 60_000); return () => clearInterval(iv); }, [load]);

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
      {/* Client Distribution Donut */}
      <div style={{ background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 10, padding: '20px 22px' }}>
        <SectionLabel>CLIENT DISTRIBUTION</SectionLabel>
        {loading ? <LoadingText /> : (
          <>
            <div style={{ position: 'relative', height: 160 }}>
              <ResponsiveContainer width="100%" height="100%">
                <PieChart>
                  <Pie data={clients} dataKey="count" cx="50%" cy="50%"
                    innerRadius={45} outerRadius={72} paddingAngle={2} strokeWidth={0}>
                    {clients.map((c, i) => <Cell key={i} fill={c.color} />)}
                  </Pie>
                  <Tooltip content={<PieTooltip />} />
                </PieChart>
              </ResponsiveContainer>
              {/* Center label */}
              <div style={{ position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', flexDirection: 'column', pointerEvents: 'none' }}>
                <span style={{ fontSize: 20, fontWeight: 700, color: '#cdd6f4', fontFamily: 'monospace' }}>{total.toLocaleString()}</span>
                <span style={{ fontSize: 9, color: '#45475a', fontFamily: 'monospace', textTransform: 'uppercase', letterSpacing: '0.1em' }}>nodes</span>
              </div>
            </div>
            <div style={{ marginTop: 12, display: 'flex', flexDirection: 'column', gap: 6 }}>
              {clients.map(c => (
                <div key={c.name} style={{ display: 'flex', alignItems: 'center', gap: 8, fontFamily: 'monospace', fontSize: 11 }}>
                  <span style={{ width: 8, height: 8, borderRadius: 2, background: c.color, flexShrink: 0 }} />
                  <span style={{ color: '#a6adc8', flex: 1 }}>{c.name}</span>
                  <span style={{ color: c.color, fontWeight: 600 }}>{c.pct}%</span>
                  <span style={{ color: '#45475a' }}>{c.count}</span>
                </div>
              ))}
            </div>
          </>
        )}
      </div>

      {/* Node Versions Bar */}
      <div style={{ background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 10, padding: '20px 22px' }}>
        <SectionLabel>NODE VERSIONS</SectionLabel>
        {loading ? <LoadingText /> : (
          <div style={{ height: 220 }}>
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={versions} layout="vertical" margin={{ left: 0, right: 16, top: 4, bottom: 4 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.04)" horizontal={false} />
                <XAxis type="number" tick={{ fill: '#45475a', fontSize: 9, fontFamily: 'monospace' }} tickLine={false} axisLine={false} />
                <YAxis type="category" dataKey="version" width={80}
                  tick={{ fill: '#a6adc8', fontSize: 9, fontFamily: 'monospace' }}
                  tickLine={false} axisLine={false} />
                <Tooltip
                  cursor={{ fill: 'rgba(255,255,255,0.03)' }}
                  contentStyle={{ background: 'rgba(10,15,25,0.97)', border: '1px solid rgba(255,255,255,0.1)', borderRadius: 6, fontFamily: 'monospace', fontSize: 11 }}
                  labelStyle={{ color: '#cdd6f4' }}
                  itemStyle={{ color: '#89b4fa' }}
                />
                <Bar dataKey="count" fill="#89b4fa" radius={[0, 3, 3, 0]}
                  background={{ fill: 'rgba(255,255,255,0.02)', radius: 3 }} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}
      </div>
    </div>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <div style={{ fontSize: 10, letterSpacing: '0.12em', color: '#45475a', textTransform: 'uppercase', fontFamily: 'monospace', marginBottom: 14 }}>
      {children}
    </div>
  );
}

function LoadingText() {
  return <div style={{ color: '#45475a', fontSize: 12, fontFamily: 'monospace', textAlign: 'center', padding: '40px 0' }}>Loading…</div>;
}

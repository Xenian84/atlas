'use client';
import { useState, useEffect, useCallback } from 'react';
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid } from 'recharts';
import { getClusterNodes, type ClusterNode } from '@/lib/atlasRpc';

const CLIENT_COLORS: Record<string, string> = {
  tachyon:    'hsl(186 100% 45%)',
  agave:      'hsl(217 80% 62%)',
  jito:       'hsl(270 60% 65%)',
  firedancer: 'hsl(154 60% 52%)',
  other:      'hsl(220 9% 38%)',
};

function detectClient(version: string | null): string {
  if (!version) return 'other';
  const v = version.toLowerCase();
  if (v.includes('tachyon'))         return 'tachyon';
  if (v.includes('jito'))            return 'jito';
  if (v.includes('fire') || v.includes('fd')) return 'firedancer';
  if (v.includes('agave'))           return 'agave';
  return 'agave';
}

interface ClientShare { name: string; count: number; pct: number; color: string; }
interface VersionShare { version: string; count: number; }

function analyze(nodes: ClusterNode[]): { clients: ClientShare[]; versions: VersionShare[] } {
  const clientMap: Record<string, number>  = {};
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
      count, pct: Math.round((count / total) * 1000) / 10,
      color: CLIENT_COLORS[name] ?? CLIENT_COLORS.other,
    }));
  const versions = Object.entries(versionMap)
    .sort((a, b) => b[1] - a[1]).slice(0, 8)
    .map(([version, count]) => ({ version, count }));
  return { clients, versions };
}

const PieTip = ({ active, payload }: { active?: boolean; payload?: { name: string; value: number; payload: ClientShare }[] }) => {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload;
  return (
    <div style={{ background: 'hsl(var(--card))', border: '1px solid hsl(var(--border-strong))', padding: '7px 11px', fontFamily: 'var(--font-mono)', fontSize: 10 }}>
      <b style={{ color: 'hsl(var(--foreground))' }}>{d.name}</b>
      <div style={{ color: 'hsl(var(--foreground-tertiary))' }}>{d.count} nodes · {d.pct}%</div>
    </div>
  );
};

export default function ClientDistribution() {
  const [nodes, setNodes]     = useState<ClusterNode[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try { setNodes(await getClusterNodes()); }
    catch { /* silent */ }
    finally { setLoading(false); }
  }, []);
  useEffect(() => { load(); const iv = setInterval(load, 30_000); return () => clearInterval(iv); }, [load]);

  const { clients, versions } = analyze(nodes);

  return (
    <div className="atlas-card" style={{ padding: '20px 20px 16px' }}>
      <span className="statlabel">CLIENT DISTRIBUTION</span>

      {loading ? (
        <div className="skeleton" style={{ height: 140, width: '100%', marginTop: 12 }} />
      ) : (
        <div style={{ display: 'flex', gap: 0, marginTop: 12, borderTop: '1px solid hsl(var(--border))' }}>
          {/* Donut */}
          <div style={{ flex: '0 0 160px', height: 140 }}>
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie data={clients} dataKey="count" cx="50%" cy="50%" innerRadius={38} outerRadius={58} strokeWidth={0}>
                  {clients.map((c, i) => <Cell key={i} fill={c.color} />)}
                </Pie>
                <Tooltip content={<PieTip />} />
              </PieChart>
            </ResponsiveContainer>
          </div>

          {/* Legend */}
          <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 6, paddingLeft: 8 }}>
            {clients.map(c => (
              <div key={c.name} style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
                <span style={{ width: 7, height: 7, background: c.color, flexShrink: 0 }} />
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-secondary))', flex: 1 }}>{c.name}</span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-tertiary))' }}>{c.pct}%</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Version bar chart */}
      {!loading && versions.length > 0 && (
        <div style={{ marginTop: 16, borderTop: '1px solid hsl(var(--border))', paddingTop: 12 }}>
          <span className="statlabel" style={{ marginBottom: 8, display: 'inline-block' }}>NODE VERSIONS</span>
          <div style={{ height: 80 }}>
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={versions} margin={{ top: 0, right: 0, left: 0, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(220 14% 12%)" vertical={false} />
                <XAxis dataKey="version" tick={{ fill: 'hsl(220 9% 38%)', fontSize: 8, fontFamily: 'var(--font-mono)' }} tickLine={false} axisLine={false} />
                <YAxis tick={{ fill: 'hsl(220 9% 38%)', fontSize: 8, fontFamily: 'var(--font-mono)' }} tickLine={false} axisLine={false} width={24} />
                <Bar dataKey="count" fill="hsl(var(--primary))" radius={0} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      )}

      <div style={{ marginTop: 10, fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
        {nodes.length} cluster nodes · updates every 30s
      </div>
    </div>
  );
}

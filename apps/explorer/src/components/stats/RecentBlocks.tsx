'use client';
import { useState, useEffect, useCallback, useRef } from 'react';
import { rpc } from '@/lib/atlasRpc';
import { clientFetch } from '@/lib/api';

interface Block { slot: number; blockTime: number | null; txCount: number; }

interface AtlasBlock {
  slot: number;
  block_time: number | null;
  tx_count: number;
  success_count: number;
  failed_count: number;
  total_fees: number;
}

async function fetchRecentBlocks(): Promise<Block[]> {
  const currentSlot = await rpc<number>('getSlot');
  // Try the last 12 slots — some may be empty or not yet indexed
  const slots = Array.from({ length: 12 }, (_, i) => currentSlot - i);

  const results = await Promise.allSettled(
    slots.map(slot => clientFetch<AtlasBlock>(`/v1/block/${slot}`))
  );

  return results
    .map((r, i) => r.status === 'fulfilled' && r.value.tx_count > 0
      ? { slot: slots[i], blockTime: r.value.block_time, txCount: r.value.tx_count }
      : null
    )
    .filter((b): b is Block => b !== null)
    .slice(0, 8);
}

const timeAgo = (ts: number | null) => {
  if (!ts) return '—';
  const d = Math.floor(Date.now() / 1000 - ts);
  if (d < 60) return `${d}s ago`;
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  return `${Math.floor(d / 3600)}h ago`;
};

export default function RecentBlocks() {
  const [blocks, setBlocks]   = useState<Block[]>([]);
  const [loading, setLoading] = useState(true);
  const [newSlot, setNewSlot] = useState<number | null>(null);
  const prevSlots = useRef<Set<number>>(new Set());

  const load = useCallback(async () => {
    try {
      const b = await fetchRecentBlocks();
      const fresh = b.filter(x => !prevSlots.current.has(x.slot));
      if (fresh.length) { setNewSlot(fresh[0].slot); setTimeout(() => setNewSlot(null), 1200); }
      b.forEach(x => prevSlots.current.add(x.slot));
      setBlocks(b);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 4_000); return () => clearInterval(iv); }, [load]);

  return (
    <div className="atlas-card" style={{ padding: '20px 0 0' }}>
      <div style={{ padding: '0 20px 12px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderBottom: '1px solid hsl(var(--border))' }}>
        <span className="statlabel">RECENT BLOCKS</span>
        <span className="live-dot" />
      </div>

      {loading ? (
        <div style={{ padding: '0 20px 16px' }}>
          {[...Array(5)].map((_, i) => (
            <div key={i} className="skeleton" style={{ height: 32, marginTop: 8 }} />
          ))}
        </div>
      ) : blocks.length === 0 ? (
        <div style={{ padding: '20px', fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-muted))' }}>
          Waiting for indexed blocks…
        </div>
      ) : (
        <table className="atlas-table">
          <thead>
            <tr>
              <th>SLOT</th>
              <th>TXS</th>
              <th style={{ textAlign: 'right' }}>TIME</th>
            </tr>
          </thead>
          <tbody>
            {blocks.map(b => (
              <tr key={b.slot} style={{
                animation: b.slot === newSlot ? 'flash-row 1.2s ease-out forwards' : 'none',
              }}>
                <td>
                  <a href={`/block/${b.slot}`} style={{
                    fontFamily: 'var(--font-mono)', fontSize: 11,
                    color: 'hsl(var(--primary))', textDecoration: 'none',
                  }}>
                    {b.slot.toLocaleString()}
                  </a>
                </td>
                <td style={{ color: 'hsl(var(--foreground))' }}>{b.txCount.toLocaleString()}</td>
                <td style={{ textAlign: 'right', color: 'hsl(var(--foreground-tertiary))' }}>
                  {timeAgo(b.blockTime)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

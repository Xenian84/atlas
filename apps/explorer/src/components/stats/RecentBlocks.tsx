'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { rpc } from '@/lib/atlasRpc';

interface Block {
  slot: number;
  blockTime: number | null;
  txCount: number;
  leader: string;
}

async function fetchRecentBlocks(): Promise<Block[]> {
  const currentSlot = await rpc<number>('getSlot');
  const slots = await rpc<number[]>('getBlocksWithLimit', [currentSlot - 20, 10]);

  const blocks = await Promise.all(
    slots.slice(-8).reverse().map(async (slot) => {
      try {
        const b = await rpc<{
          blockTime: number | null;
          transactions: unknown[];
          rewards?: { pubkey: string; rewardType: string }[];
        }>('getBlock', [slot, { transactionDetails: 'none', rewards: true, maxSupportedTransactionVersion: 0 }]);
        const leader = b.rewards?.find(r => r.rewardType === 'Fee')?.pubkey ?? '';
        return {
          slot,
          blockTime: b.blockTime,
          txCount: b.transactions?.length ?? 0,
          leader,
        };
      } catch {
        return { slot, blockTime: null, txCount: 0, leader: '' };
      }
    })
  );
  return blocks;
}

function timeAgo(ts: number | null): string {
  if (!ts) return '—';
  const diff = Math.floor(Date.now() / 1000 - ts);
  if (diff < 60)  return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  return `${Math.floor(diff / 3600)}h ago`;
}

function shorten(addr: string) {
  if (!addr) return '—';
  return `${addr.slice(0, 4)}…${addr.slice(-4)}`;
}

export default function RecentBlocks() {
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [loading, setLoading] = useState(true);
  const [newSlot, setNewSlot] = useState<number | null>(null);
  const prevSlots = useRef<Set<number>>(new Set());

  const load = useCallback(async () => {
    try {
      const bs = await fetchRecentBlocks();
      bs.forEach(b => {
        if (!prevSlots.current.has(b.slot)) {
          setNewSlot(b.slot);
          setTimeout(() => setNewSlot(null), 1200);
        }
        prevSlots.current.add(b.slot);
      });
      setBlocks(bs);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 4_000); return () => clearInterval(iv); }, [load]);

  return (
    <div style={{ background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 10, padding: '20px 22px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 14 }}>
        <div style={{ fontSize: 10, letterSpacing: '0.12em', color: '#45475a', textTransform: 'uppercase', fontFamily: 'monospace' }}>
          RECENT BLOCKS
        </div>
        <LiveBadge />
      </div>

      {/* Column headers */}
      <div style={{ display: 'grid', gridTemplateColumns: '1.4fr 0.8fr 0.8fr 1fr', gap: 8, padding: '0 0 8px', borderBottom: '1px solid rgba(255,255,255,0.06)', marginBottom: 6 }}>
        {['SLOT', 'TXS', 'TIME', 'LEADER'].map(h => (
          <span key={h} style={{ fontSize: 9, color: '#45475a', fontFamily: 'monospace', letterSpacing: '0.1em', textTransform: 'uppercase' }}>{h}</span>
        ))}
      </div>

      {loading ? (
        <div style={{ color: '#45475a', fontSize: 12, fontFamily: 'monospace', textAlign: 'center', padding: '30px 0' }}>
          Loading blocks…
        </div>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {blocks.map(b => (
            <BlockRow key={b.slot} block={b} isNew={newSlot === b.slot} />
          ))}
        </div>
      )}
    </div>
  );
}

function BlockRow({ block, isNew }: { block: Block; isNew: boolean }) {
  return (
    <div style={{
      display: 'grid', gridTemplateColumns: '1.4fr 0.8fr 0.8fr 1fr',
      gap: 8, padding: '7px 0',
      borderBottom: '1px solid rgba(255,255,255,0.03)',
      animation: isNew ? 'flash-row 1s ease' : undefined,
      transition: 'background 0.3s',
    }}>
      <a
        href={`/block/${block.slot}`}
        style={{ color: '#89b4fa', fontFamily: 'monospace', fontSize: 12, fontWeight: 600, textDecoration: 'none' }}
      >
        {block.slot.toLocaleString()}
      </a>
      <span style={{ color: '#cdd6f4', fontFamily: 'monospace', fontSize: 12 }}>
        {block.txCount.toLocaleString()}
      </span>
      <span style={{ color: '#6c7086', fontFamily: 'monospace', fontSize: 11 }}>
        {timeAgo(block.blockTime)}
      </span>
      <span style={{ color: '#a6adc8', fontFamily: 'monospace', fontSize: 11 }}>
        {shorten(block.leader)}
      </span>
    </div>
  );
}

function LiveBadge() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6, background: 'rgba(166,227,161,0.1)', border: '1px solid rgba(166,227,161,0.25)', borderRadius: 20, padding: '3px 10px' }}>
      <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#a6e3a1', display: 'inline-block', animation: 'pulse-ring 1.5s ease-out infinite' }} />
      <span style={{ fontSize: 9, color: '#a6e3a1', fontFamily: 'monospace', letterSpacing: '0.1em' }}>LIVE</span>
    </div>
  );
}

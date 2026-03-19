'use client';

import { useState, useEffect, useCallback } from 'react';
import dynamic from 'next/dynamic';
import { getEpochInfo, getNetworkPulse, getVersion, type EpochInfo, type NetworkPulse } from '@/lib/atlasRpc';
import StatCard from '@/components/stats/StatCard';
import EpochCard from '@/components/stats/EpochCard';
import ValidatorTable from '@/components/stats/ValidatorTable';

// Client-only components (use browser APIs / recharts)
const TpsChart            = dynamic(() => import('@/components/stats/TpsChart'),            { ssr: false });
const ClientDistribution  = dynamic(() => import('@/components/stats/ClientDistribution'),  { ssr: false });
const RecentBlocks        = dynamic(() => import('@/components/stats/RecentBlocks'),        { ssr: false });

export default function StatsPage() {
  const [pulse, setPulse]     = useState<NetworkPulse | null>(null);
  const [epoch, setEpoch]     = useState<EpochInfo | null>(null);
  const [version, setVersion] = useState<string>('');
  const [lastUpdate, setLastUpdate] = useState<string>('');

  const load = useCallback(async () => {
    try {
      const [p, e, v] = await Promise.all([
        getNetworkPulse().catch(() => null),
        getEpochInfo().catch(() => null),
        getVersion().catch(() => null),
      ]);
      if (p) setPulse(p);
      if (e) setEpoch(e);
      if (v) setVersion(v['solana-core'] ?? '');
      setLastUpdate(new Date().toLocaleTimeString());
    } catch { /* silent */ }
  }, []);

  useEffect(() => { load(); const iv = setInterval(load, 8_000); return () => clearInterval(iv); }, [load]);

  return (
    <div style={{
      minHeight: '100vh', background: '#050508',
      color: '#cdd6f4', fontFamily: '"Courier New", monospace',
    }}>
      {/* ── Header ─────────────────────────────────────────── */}
      <header style={{
        borderBottom: '1px solid rgba(255,255,255,0.07)',
        padding: '0 32px', height: 52,
        display: 'flex', alignItems: 'center', gap: 0,
        background: 'rgba(5,5,10,0.98)', position: 'sticky', top: 0, zIndex: 100,
      }}>
        <a href="/" style={{ color: '#00e5ff', fontWeight: 900, fontSize: 14, letterSpacing: '0.18em', textDecoration: 'none', marginRight: 24 }}>
          ATLAS
        </a>
        <NavLink href="/"      label="HOME" />
        <NavLink href="/stats" label="STATS" active />

        <div style={{ flex: 1 }} />

        <div style={{ display: 'flex', alignItems: 'center', gap: 16, fontSize: 10, color: '#45475a' }}>
          {version && <span>Tachyon <span style={{ color: '#a6adc8' }}>{version}</span></span>}
          {lastUpdate && <span>Updated <span style={{ color: '#a6adc8' }}>{lastUpdate}</span></span>}
          <div style={{ width: 6, height: 6, borderRadius: '50%', background: '#a6e3a1', animation: 'pulse-ring 1.5s ease-out infinite' }} />
        </div>
      </header>

      <main style={{ maxWidth: 1280, margin: '0 auto', padding: '32px 24px', display: 'flex', flexDirection: 'column', gap: 20 }}>

        {/* ── Page title ───────────────────────────────────── */}
        <div>
          <h1 style={{ fontSize: 22, fontWeight: 700, color: '#cdd6f4', margin: 0, letterSpacing: '0.04em' }}>
            X1 Network Statistics
          </h1>
          <p style={{ fontSize: 11, color: '#45475a', margin: '4px 0 0', letterSpacing: '0.04em' }}>
            Live data via Atlas API · All metrics powered by the X1 Tachyon validator
          </p>
        </div>

        {/* ── Hero stat chips ──────────────────────────────── */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: 14 }}>
          <StatCard
            label="Current Slot"
            value={epoch?.absoluteSlot.toLocaleString() ?? '…'}
            sub={`Block height: ${epoch?.blockHeight.toLocaleString() ?? '…'}`}
            accent="#89b4fa"
            live
          />
          <StatCard
            label="Epoch"
            value={epoch?.epoch ?? '…'}
            sub={epoch ? `${((epoch.slotIndex / epoch.slotsInEpoch) * 100).toFixed(1)}% complete` : '…'}
            accent="#a6e3a1"
          />
          <StatCard
            label="Total Transactions"
            value={epoch ? formatBig(epoch.transactionCount) : '…'}
            sub="All-time on X1 mainnet"
            accent="#cba6f7"
          />
          <StatCard
            label="Indexed 24h"
            value={pulse?.indexed_txs_24h != null ? pulse.indexed_txs_24h.toLocaleString() : '…'}
            sub="Via Atlas indexer"
            accent="#f9e2af"
          />
          <StatCard
            label="XNT Price"
            value={pulse?.xnt_price_usd != null ? `$${pulse.xnt_price_usd.toFixed(4)}` : '…'}
            sub="Via XDex oracle"
            accent="#fab387"
          />
          <StatCard
            label="Active Wallets"
            value={pulse?.active_wallets?.toLocaleString() ?? '…'}
            sub="Indexed unique addresses"
            accent="#94e2d5"
          />
        </div>

        {/* ── TPS Chart (full width) ───────────────────────── */}
        <TpsChart />

        {/* ── Epoch + Recent Blocks ────────────────────────── */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1.2fr', gap: 16 }}>
          <EpochCard />
          <RecentBlocks />
        </div>

        {/* ── Client Distribution + Node Versions ─────────── */}
        <ClientDistribution />

        {/* ── Validator Table ──────────────────────────────── */}
        <ValidatorTable />

      </main>

      {/* CSS animations */}
      <style>{`
        @keyframes pulse-ring {
          0%   { transform: scale(1);   opacity: 1; }
          100% { transform: scale(2.2); opacity: 0; }
        }
        @keyframes flash-row {
          0%   { background: rgba(0,229,255,0.12); }
          100% { background: transparent; }
        }
      `}</style>
    </div>
  );
}

function NavLink({ href, label, active }: { href: string; label: string; active?: boolean }) {
  return (
    <a
      href={href}
      style={{
        fontSize: 10, letterSpacing: '0.12em', textDecoration: 'none',
        color: active ? '#cdd6f4' : '#45475a',
        fontWeight: active ? 600 : 400,
        padding: '0 12px', height: '100%',
        display: 'flex', alignItems: 'center',
        borderBottom: active ? '2px solid #00e5ff' : '2px solid transparent',
        transition: 'color 0.15s',
      }}
    >
      {label}
    </a>
  );
}

function formatBig(n: number): string {
  if (n >= 1e12) return `${(n / 1e12).toFixed(2)}T`;
  if (n >= 1e9)  return `${(n / 1e9).toFixed(1)}B`;
  if (n >= 1e6)  return `${(n / 1e6).toFixed(1)}M`;
  return n.toLocaleString();
}

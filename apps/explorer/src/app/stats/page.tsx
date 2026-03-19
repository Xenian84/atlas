'use client';
import { useState, useEffect, useCallback } from 'react';
import dynamic from 'next/dynamic';
import { getEpochInfo, getNetworkPulse, getVersion, type EpochInfo, type NetworkPulse } from '@/lib/atlasRpc';
import StatCard     from '@/components/stats/StatCard';
import EpochCard    from '@/components/stats/EpochCard';
import ValidatorTable from '@/components/stats/ValidatorTable';

const TpsChart           = dynamic(() => import('@/components/stats/TpsChart'),           { ssr: false });
const ClientDistribution = dynamic(() => import('@/components/stats/ClientDistribution'), { ssr: false });
const RecentBlocks       = dynamic(() => import('@/components/stats/RecentBlocks'),       { ssr: false });

const SOL_LAMPORTS = 1_000_000_000;

export default function StatsPage() {
  const [pulse, setPulse]           = useState<NetworkPulse | null>(null);
  const [epoch, setEpoch]           = useState<EpochInfo | null>(null);
  const [version, setVersion]       = useState<string>('');
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

  const tps        = pulse?.tps_1m         ? Math.round(pulse.tps_1m).toLocaleString()       : epoch?.absoluteSlot ? '—' : '…';
  const slot       = epoch?.absoluteSlot   ? epoch.absoluteSlot.toLocaleString()             : '…';
  const price      = pulse?.xnt_price_usd  ? `$${pulse.xnt_price_usd.toFixed(4)}`           : '—';
  const tvl        = pulse?.indexed_txs_24h ? pulse.indexed_txs_24h.toLocaleString()         : '—';
  const validators = pulse?.active_wallets?.toLocaleString() ?? '—';

  return (
    <div style={{ minHeight: 'calc(100vh - 52px)', background: 'hsl(var(--background))' }}>
      <main style={{ maxWidth: 1360, margin: '0 auto', padding: '28px 24px', display: 'flex', flexDirection: 'column', gap: 20 }}>

        {/* ── Page header ────────────────────────────────────── */}
        <div style={{ display: 'flex', alignItems: 'flex-end', justifyContent: 'space-between', borderBottom: '1px solid hsl(var(--border))', paddingBottom: 14 }}>
          <div>
            <h1 style={{ fontFamily: 'var(--font-sans)', fontSize: 18, fontWeight: 700, color: 'hsl(var(--foreground))', margin: 0, letterSpacing: '-.01em' }}>
              X1 Network Statistics
            </h1>
            <p style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-tertiary))', margin: '4px 0 0', letterSpacing: '.04em' }}>
              Live data via Atlas API — all metrics sourced from the X1 Tachyon validator
            </p>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
            {version && (
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.06em' }}>
                TACHYON <span style={{ color: 'hsl(var(--foreground-tertiary))' }}>{version}</span>
              </span>
            )}
            {lastUpdate && (
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.06em' }}>
                {lastUpdate}
              </span>
            )}
            <span className="live-dot" />
          </div>
        </div>

        {/* ── Hero stat grid ─────────────────────────────────── */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', border: '1px solid hsl(var(--border))' }}>
          {([
            { label: 'CURRENT SLOT',  value: slot,       accentVar: 'primary',       live: true  },
            { label: 'TPS (1m AVG)',   value: tps,        accentVar: 'accent-blue',   live: true  },
            { label: 'X1 PRICE',      value: price,      accentVar: 'accent-green',  live: false },
            { label: 'ACTIVE WALLETS',value: validators, accentVar: 'accent-purple', live: false },
            { label: 'TXS (24h)',     value: tvl,        accentVar: 'accent-amber',  live: false },
          ] as const).map(({ label, value, accentVar, live }, i) => (
            <div key={label} style={{ borderRight: i < 4 ? '1px solid hsl(var(--border))' : 'none' }}>
              <StatCard label={label} value={value} accentVar={accentVar} live={live} />
            </div>
          ))}
        </div>

        {/* ── TPS chart + Epoch ──────────────────────────────── */}
        <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 12 }}>
          <TpsChart />
          <EpochCard />
        </div>

        {/* ── Recent blocks + Client distribution ───────────── */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>
          <RecentBlocks />
          <ClientDistribution />
        </div>

        {/* ── Validator table ─────────────────────────────────── */}
        <div id="validators">
          <ValidatorTable />
        </div>

      </main>
    </div>
  );
}

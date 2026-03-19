'use client';
import { useState, useEffect, useCallback } from 'react';
import { getEpochInfo, type EpochInfo } from '@/lib/atlasRpc';

export default function EpochCard() {
  const [info, setInfo]       = useState<EpochInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    try { setInfo(await getEpochInfo()); }
    catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => {
    load();
    const iv = setInterval(load, 6_000);
    return () => clearInterval(iv);
  }, [load]);

  const pct       = info ? Math.min(100, (info.slotIndex / info.slotsInEpoch) * 100) : 0;
  const remaining = info ? info.slotsInEpoch - info.slotIndex : 0;
  const estHours  = ((remaining * 0.4) / 3600).toFixed(1);

  const Row = ({ k, v }: { k: string; v: string }) => (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '5px 0', borderBottom: '1px solid hsl(var(--border))' }}>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-tertiary))', letterSpacing: '.06em' }}>{k}</span>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-secondary))' }}>{v}</span>
    </div>
  );

  return (
    <div className="atlas-card" style={{ padding: '20px 20px 16px' }}>
      {/* Header */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 14 }}>
        <div>
          <span className="statlabel">EPOCH</span>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 28, fontWeight: 700, color: 'hsl(var(--accent-green))', lineHeight: 1, marginTop: 8 }}>
            {loading ? '—' : (info?.epoch ?? '—')}
          </div>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 22, fontWeight: 700, color: 'hsl(var(--foreground))' }}>
            {pct.toFixed(1)}<span style={{ fontSize: 12, color: 'hsl(var(--foreground-tertiary))' }}>%</span>
          </div>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', marginTop: 2 }}>
            {estHours}h remaining
          </div>
        </div>
      </div>

      {/* Progress bar */}
      <div style={{ height: 4, background: 'hsl(var(--border))', marginBottom: 16, overflow: 'hidden' }}>
        <div style={{
          height: '100%', width: `${pct}%`,
          background: 'linear-gradient(90deg, hsl(var(--accent-green)), hsl(var(--primary)))',
          transition: 'width 0.6s ease',
        }} />
      </div>

      {/* Detail rows */}
      <div>
        <Row k="EPOCH START SLOT"   v={loading ? '…' : (info?.absoluteSlot != null ? (info.absoluteSlot - info.slotIndex).toLocaleString() : '—')} />
        <Row k="CURRENT SLOT"       v={loading ? '…' : (info?.absoluteSlot.toLocaleString() ?? '—')} />
        <Row k="SLOTS IN EPOCH"     v={loading ? '…' : (info?.slotsInEpoch.toLocaleString() ?? '—')} />
        <Row k="SLOTS ELAPSED"      v={loading ? '…' : (info?.slotIndex.toLocaleString() ?? '—')} />
        <Row k="SLOTS REMAINING"    v={loading ? '…' : remaining.toLocaleString()} />
        <Row k="BLOCK HEIGHT"       v={loading ? '…' : (info?.blockHeight?.toLocaleString() ?? '—')} />
      </div>
    </div>
  );
}

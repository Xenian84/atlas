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

  const pct = info ? Math.min(100, (info.slotIndex / info.slotsInEpoch) * 100) : 0;
  const remaining = info ? info.slotsInEpoch - info.slotIndex : 0;
  // Estimate seconds remaining (400ms avg slot time on X1)
  const secsLeft = remaining * 0.4;
  const estHours = (secsLeft / 3600).toFixed(1);

  return (
    <div style={{
      background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)',
      borderRadius: 10, padding: '20px 22px',
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginBottom: 14 }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.12em', color: '#45475a', textTransform: 'uppercase', fontFamily: 'monospace', marginBottom: 4 }}>
            EPOCH
          </div>
          <div style={{ fontSize: 28, fontWeight: 700, color: '#a6e3a1', fontFamily: 'monospace', lineHeight: 1 }}>
            {loading ? '…' : info?.epoch ?? '—'}
          </div>
        </div>
        <div style={{ textAlign: 'right', fontFamily: 'monospace' }}>
          <div style={{ fontSize: 20, fontWeight: 700, color: '#cdd6f4' }}>
            {pct.toFixed(1)}<span style={{ fontSize: 13, color: '#6c7086' }}>%</span>
          </div>
          <div style={{ fontSize: 10, color: '#45475a' }}>complete</div>
        </div>
      </div>

      {/* Progress bar */}
      <div style={{
        height: 6, background: 'rgba(255,255,255,0.06)', borderRadius: 3,
        overflow: 'hidden', marginBottom: 16,
      }}>
        <div style={{
          height: '100%', width: `${pct}%`, borderRadius: 3,
          background: 'linear-gradient(90deg, #a6e3a1, #94e2d5)',
          transition: 'width 0.6s ease',
        }} />
      </div>

      {/* Slot info grid */}
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px 20px' }}>
        <SlotRow label="START SLOT"    value={info ? (info.absoluteSlot - info.slotIndex).toLocaleString() : '…'} />
        <SlotRow label="CURRENT SLOT"  value={info?.absoluteSlot.toLocaleString() ?? '…'} accent />
        <SlotRow label="SLOTS ELAPSED" value={info?.slotIndex.toLocaleString() ?? '…'} />
        <SlotRow label="SLOTS LEFT"    value={info ? remaining.toLocaleString() : '…'} />
        <SlotRow label="SLOTS IN EPOCH" value={info?.slotsInEpoch.toLocaleString() ?? '…'} />
        <SlotRow label="EST. REMAINING" value={info ? `~${estHours}h` : '…'} />
      </div>

      {/* Epochs per year estimate */}
      {info && (
        <div style={{
          marginTop: 14, paddingTop: 12,
          borderTop: '1px solid rgba(255,255,255,0.06)',
          fontFamily: 'monospace', fontSize: 10, color: '#45475a',
          display: 'flex', justifyContent: 'space-between',
        }}>
          <span>Epochs per year (est.)</span>
          <span style={{ color: '#a6adc8' }}>
            ~{Math.round((365.25 * 24 * 3600) / (info.slotsInEpoch * 0.4))}
          </span>
        </div>
      )}
    </div>
  );
}

function SlotRow({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div>
      <div style={{ fontSize: 9, color: '#45475a', letterSpacing: '0.1em', textTransform: 'uppercase', fontFamily: 'monospace', marginBottom: 2 }}>
        {label}
      </div>
      <div style={{ fontSize: 13, fontWeight: accent ? 700 : 500, color: accent ? '#a6e3a1' : '#cdd6f4', fontFamily: 'monospace' }}>
        {value}
      </div>
    </div>
  );
}

'use client';

import { memo } from 'react';
import { Handle, Position, NodeProps } from '@xyflow/react';
import { format } from 'date-fns';

export interface WalletNodeData {
  address: string;
  sol_balance: number;
  token_count: number;
  tx_count: number;
  move_count: number;
  first_seen: number | null;
  last_seen: number | null;
  labels: string[];
  isRoot?: boolean;
}

function shorten(addr: string, chars = 4) {
  return `${addr.slice(0, chars)}...${addr.slice(-chars)}`;
}

function formatTs(ts: number | null) {
  if (!ts) return 'unknown';
  try { return format(new Date(ts * 1000), 'MMM d, yyyy'); }
  catch { return 'unknown'; }
}

function WalletNode({ data, selected }: NodeProps<{ data: WalletNodeData }>) {
  const d = (data as unknown as WalletNodeData);
  const isRoot = d.isRoot;

  return (
    <>
      <Handle type="target" position={Position.Left}  style={{ background: '#00e5ff', border: 'none', width: 8, height: 8 }} />
      <Handle type="source" position={Position.Right} style={{ background: '#00e5ff', border: 'none', width: 8, height: 8 }} />

      <div
        className={[
          'wallet-node',
          isRoot   ? 'wallet-node--root'     : '',
          selected ? 'wallet-node--selected' : '',
        ].join(' ')}
        style={{
          background: isRoot ? 'rgba(0, 229, 255, 0.06)' : 'rgba(10,15,20,0.92)',
          border: isRoot
            ? '1.5px solid #00e5ff'
            : selected
            ? '1px solid rgba(0,229,255,0.6)'
            : '1px solid rgba(255,255,255,0.12)',
          borderRadius: 6,
          padding: '10px 14px',
          minWidth: 200,
          maxWidth: 240,
          fontFamily: '"Courier New", monospace',
          fontSize: 11,
          color: '#cdd6f4',
          boxShadow: isRoot
            ? '0 0 18px rgba(0,229,255,0.35), 0 0 4px rgba(0,229,255,0.2)'
            : selected
            ? '0 0 8px rgba(0,229,255,0.2)'
            : '0 2px 12px rgba(0,0,0,0.6)',
          transition: 'box-shadow 0.2s, border-color 0.2s',
        }}
      >
        {/* Address + label row */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 6 }}>
          <span
            style={{
              color: isRoot ? '#00e5ff' : '#94e2d5',
              fontWeight: 700,
              fontSize: 12,
              letterSpacing: '0.04em',
            }}
          >
            {shorten(d.address)}
          </span>
          {d.labels?.[0] && (
            <span
              style={{
                fontSize: 9,
                background: 'rgba(0,229,255,0.12)',
                color: '#00e5ff',
                border: '1px solid rgba(0,229,255,0.3)',
                borderRadius: 3,
                padding: '1px 5px',
                letterSpacing: '0.06em',
                textTransform: 'uppercase',
              }}
            >
              {d.labels[0]}
            </span>
          )}
        </div>

        {/* Stats grid */}
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '3px 12px' }}>
          <StatRow label="SOL" value={d.sol_balance > 0 ? d.sol_balance.toFixed(3) : '—'} accent />
          <StatRow label="Tokens" value={d.token_count > 0 ? String(d.token_count) : '—'} />
          <StatRow label="Txns" value={String(d.tx_count)} />
          <StatRow label="Moves" value={String(d.move_count)} />
        </div>

        {/* Last seen */}
        {d.last_seen && (
          <div
            style={{
              marginTop: 7,
              paddingTop: 6,
              borderTop: '1px solid rgba(255,255,255,0.07)',
              color: '#6c7086',
              fontSize: 10,
              display: 'flex',
              justifyContent: 'space-between',
            }}
          >
            <span>Last seen</span>
            <span style={{ color: '#a6adc8' }}>{formatTs(d.last_seen)}</span>
          </div>
        )}
      </div>
    </>
  );
}

function StatRow({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
      <span style={{ color: '#45475a', fontSize: 9, textTransform: 'uppercase', letterSpacing: '0.08em' }}>
        {label}
      </span>
      <span style={{ color: accent ? '#cba6f7' : '#cdd6f4', fontWeight: accent ? 600 : 400 }}>
        {value}
      </span>
    </div>
  );
}

export default memo(WalletNode);

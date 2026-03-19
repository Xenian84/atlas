'use client';

import { type ReactNode } from 'react';

interface Props {
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  accent?: string;
  live?: boolean;
}

export default function StatCard({ label, value, sub, accent = '#00e5ff', live }: Props) {
  return (
    <div style={{
      background: 'rgba(10,15,25,0.8)',
      border: '1px solid rgba(255,255,255,0.08)',
      borderRadius: 10,
      padding: '18px 22px',
      display: 'flex',
      flexDirection: 'column',
      gap: 6,
      position: 'relative',
      overflow: 'hidden',
    }}>
      {/* Glow accent line */}
      <div style={{
        position: 'absolute', top: 0, left: 0, right: 0,
        height: 2, background: `linear-gradient(90deg, ${accent}60, transparent)`,
      }} />

      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{
          fontSize: 10, letterSpacing: '0.12em', textTransform: 'uppercase',
          color: '#45475a', fontFamily: 'monospace',
        }}>
          {label}
        </span>
        {live && <PulseDot />}
      </div>

      <div style={{
        fontSize: 26, fontWeight: 700, color: '#cdd6f4',
        fontFamily: 'monospace', lineHeight: 1,
      }}>
        {value}
      </div>

      {sub && (
        <div style={{ fontSize: 11, color: '#6c7086', fontFamily: 'monospace' }}>
          {sub}
        </div>
      )}
    </div>
  );
}

function PulseDot() {
  return (
    <span style={{ position: 'relative', display: 'inline-block', width: 8, height: 8 }}>
      <span style={{
        position: 'absolute', inset: 0, borderRadius: '50%',
        background: '#a6e3a1',
        animation: 'pulse-ring 1.5s ease-out infinite',
      }} />
      <span style={{
        position: 'absolute', inset: 1, borderRadius: '50%',
        background: '#a6e3a1',
      }} />
    </span>
  );
}

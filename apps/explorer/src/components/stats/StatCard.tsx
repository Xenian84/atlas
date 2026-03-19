'use client';
import { type ReactNode } from 'react';

interface Props {
  label: string;
  value: ReactNode;
  sub?: ReactNode;
  accentVar?: string;
  live?: boolean;
}

export default function StatCard({ label, value, sub, accentVar = 'primary', live }: Props) {
  const cssColor = accentVar.startsWith('var(') ? accentVar : `var(--${accentVar})`;
  return (
    <div style={{
      background: 'hsl(var(--card))',
      border: '1px solid hsl(var(--border))',
      padding: '18px 20px',
      display: 'flex', flexDirection: 'column', gap: 8,
      position: 'relative', overflow: 'hidden',
    }}>
      {/* Accent top bar */}
      <div style={{
        position: 'absolute', top: 0, left: 0, right: 0, height: 2,
        background: `linear-gradient(90deg, hsl(${cssColor}) 0%, transparent 70%)`,
      }} />

      {/* Label row */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <span className="statlabel">{label}</span>
        {live && <span className="live-dot" />}
      </div>

      {/* Value */}
      <div style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 24, fontWeight: 700,
        color: 'hsl(var(--foreground))',
        lineHeight: 1, letterSpacing: '-.01em',
      }}>
        {value}
      </div>

      {/* Sub */}
      {sub && (
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-tertiary))' }}>
          {sub}
        </div>
      )}
    </div>
  );
}

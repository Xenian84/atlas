import React from 'react';

const TAG_MAP: Record<string, { bg: string; color: string; border: string }> = {
  spam:          { bg: 'hsla(0,70%,20%,.25)',   color: 'hsl(0 70% 70%)',   border: 'hsla(0,70%,40%,.4)' },
  failed:        { bg: 'hsla(0,70%,20%,.25)',   color: 'hsl(0 70% 70%)',   border: 'hsla(0,70%,40%,.4)' },
  swap:          { bg: 'hsla(142,60%,18%,.3)',  color: 'hsl(142 55% 60%)', border: 'hsla(142,55%,35%,.4)' },
  transfer:      { bg: 'hsla(217,70%,18%,.3)',  color: 'hsl(217 70% 65%)', border: 'hsla(217,70%,40%,.4)' },
  mint:          { bg: 'hsla(270,60%,20%,.3)',  color: 'hsl(270 60% 70%)', border: 'hsla(270,60%,40%,.4)' },
  burn:          { bg: 'hsla(25,80%,18%,.3)',   color: 'hsl(25 80% 65%)',  border: 'hsla(25,80%,40%,.4)' },
  stake:         { bg: 'hsla(48,80%,18%,.3)',   color: 'hsl(48 80% 65%)',  border: 'hsla(48,80%,40%,.4)' },
  deploy:        { bg: 'hsla(185,80%,15%,.3)',  color: 'hsl(185 70% 60%)', border: 'hsla(185,70%,35%,.4)' },
  priority_fee:  { bg: 'hsl(var(--background-secondary))', color: 'hsl(var(--foreground-tertiary))', border: 'hsl(var(--border-strong))' },
  high_compute:  { bg: 'hsl(var(--background-secondary))', color: 'hsl(var(--foreground-tertiary))', border: 'hsl(var(--border-strong))' },
  fee_only:      { bg: 'hsl(var(--background-secondary))', color: 'hsl(var(--foreground-muted))',    border: 'hsl(var(--border))' },
};

const DEFAULT_STYLE = {
  bg: 'hsl(var(--background-secondary))',
  color: 'hsl(var(--foreground-tertiary))',
  border: 'hsl(var(--border-strong))',
};

export function TagBadge({ tag }: { tag: string }) {
  const s = TAG_MAP[tag] ?? DEFAULT_STYLE;
  return (
    <span style={{
      fontFamily: 'var(--font-mono)',
      fontSize: 9,
      letterSpacing: '.08em',
      padding: '2px 7px',
      background: s.bg,
      color: s.color,
      border: `1px solid ${s.border}`,
    }}>
      {tag.toUpperCase().replace(/_/g, ' ')}
    </span>
  );
}

export function SpamBadge() {
  return <TagBadge tag="spam" />;
}

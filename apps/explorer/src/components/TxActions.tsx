'use client';
import { useState } from 'react';
import { clientFetch } from '@/lib/api';

const mono: React.CSSProperties = { fontFamily: 'var(--font-mono)' };

const btn: React.CSSProperties = {
  ...mono,
  fontSize: 11,
  letterSpacing: '.08em',
  padding: '8px 16px',
  border: '1px solid hsl(var(--border-strong))',
  background: 'hsl(var(--card))',
  color: 'hsl(var(--foreground-secondary))',
  cursor: 'pointer',
  transition: 'border-color .15s, color .15s',
};

/* ── Explain panel ─────────────────────────────────────────── */
interface ExplainResult {
  explain: { summary: string; bullets: string[]; confidence: number };
  factsToon: string;
}

function ExplainBtn({ sig }: { sig: string }) {
  const [state,  setState]  = useState<'idle' | 'loading' | 'open' | 'error'>('idle');
  const [result, setResult] = useState<ExplainResult | null>(null);
  const [err,    setErr]    = useState('');

  const handle = async () => {
    if (state === 'open') { setState('idle'); return; }
    if (result)            { setState('open'); return; }
    setState('loading');
    try {
      const data = await clientFetch<ExplainResult>(`/v1/tx/${sig}/explain`, { method: 'POST' });
      setResult(data);
      setState('open');
    } catch (e: unknown) {
      setErr(e instanceof Error ? e.message : 'Failed');
      setState('error');
      setTimeout(() => setState('idle'), 3000);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 10, flex: 1 }}>
      <button
        onClick={handle}
        disabled={state === 'loading'}
        style={{
          ...btn,
          borderColor: 'hsla(var(--primary),.4)',
          color: 'hsl(var(--primary))',
          background: 'hsla(var(--primary),.06)',
        }}
        onMouseEnter={e => { e.currentTarget.style.background = 'hsla(var(--primary),.12)'; }}
        onMouseLeave={e => { e.currentTarget.style.background = 'hsla(var(--primary),.06)'; }}
      >
        {state === 'loading' ? 'Explaining…' : state === 'open' ? '✦ Hide Explanation' : '✦ Explain with AI'}
      </button>

      {state === 'error' && (
        <span style={{ ...mono, fontSize: 10, color: 'hsl(var(--accent-red))' }}>{err}</span>
      )}

      {state === 'open' && result && (
        <div className="atlas-card" style={{ padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ ...mono, fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.1em' }}>CONFIDENCE</span>
            <span style={{
              ...mono, fontSize: 9,
              padding: '2px 7px',
              background: result.explain.confidence >= 0.8
                ? 'hsla(var(--accent-green),.12)'
                : 'hsl(var(--background-secondary))',
              border: `1px solid ${result.explain.confidence >= 0.8 ? 'hsla(var(--accent-green),.3)' : 'hsl(var(--border))'}`,
              color: result.explain.confidence >= 0.8 ? 'hsl(var(--accent-green))' : 'hsl(var(--foreground-tertiary))',
            }}>
              {(result.explain.confidence * 100).toFixed(0)}%
            </span>
          </div>
          <p style={{ ...mono, fontSize: 12, color: 'hsl(var(--foreground))', lineHeight: 1.6, margin: 0 }}>
            {result.explain.summary}
          </p>
          <ul style={{ margin: 0, paddingLeft: 0, listStyle: 'none', display: 'flex', flexDirection: 'column', gap: 4 }}>
            {result.explain.bullets.map((b, i) => (
              <li key={i} style={{ ...mono, fontSize: 11, color: 'hsl(var(--foreground-secondary))', display: 'flex', gap: 8 }}>
                <span style={{ color: 'hsl(var(--primary))', flexShrink: 0 }}>•</span>
                {b}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

/* ── TOON copy ─────────────────────────────────────────────── */
function ToonCopy({ sig }: { sig: string }) {
  const [state, setState] = useState<'idle' | 'copying' | 'copied' | 'error'>('idle');

  const handle = async () => {
    setState('copying');
    try {
      const data = await clientFetch<{ factsToon: string }>(`/v1/tx/${sig}/explain`, { method: 'POST' });
      await navigator.clipboard.writeText(data.factsToon ?? '');
      setState('copied');
      setTimeout(() => setState('idle'), 2000);
    } catch {
      setState('error');
      setTimeout(() => setState('idle'), 2000);
    }
  };

  return (
    <button
      onClick={handle}
      disabled={state === 'copying'}
      style={btn}
      onMouseEnter={e => { e.currentTarget.style.borderColor = 'hsl(var(--primary))'; e.currentTarget.style.color = 'hsl(var(--primary))'; }}
      onMouseLeave={e => { e.currentTarget.style.borderColor = 'hsl(var(--border-strong))'; e.currentTarget.style.color = 'hsl(var(--foreground-secondary))'; }}
    >
      {state === 'idle'    ? '⊡ Copy Facts'  :
       state === 'copying' ? 'Copying…'       :
       state === 'copied'  ? '✓ Copied!'      : '✗ Error'}
    </button>
  );
}

/* ── Combined export ───────────────────────────────────────── */
export function TxActions({ sig }: { sig: string }) {
  return (
    <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'flex-start' }}>
      <ExplainBtn sig={sig} />
      <ToonCopy sig={sig} />
    </div>
  );
}

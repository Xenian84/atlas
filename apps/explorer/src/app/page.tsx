'use client';
import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { Search, Activity, Zap, Users, Database } from 'lucide-react';

const EXAMPLES = [
  '2sgQ7LzA7urZ4joMy4uU3Rcus82ZoLbHa54UvChJc9j3',
  '5YNmS1R9nNSCDzb5a7mMJ1dwK9uHeAAF4CmPEwKgVWr8',
];

const FEATURES = [
  { icon: Activity, label: 'LIVE TPS', desc: 'Real-time transaction throughput and network pulse' },
  { icon: Zap,      label: 'TRACE',    desc: 'Visual wallet intelligence — counterparty graph' },
  { icon: Users,    label: 'VALIDATORS', desc: 'Full validator set with stake, APY, commission' },
  { icon: Database, label: 'INDEXER',  desc: 'Sub-second indexed history for any address or tx' },
];

export default function HomePage() {
  const [query, setQuery] = useState('');
  const router = useRouter();

  function navigate(q: string) {
    const v = q.trim();
    if (!v) return;
    if (v.length > 60) router.push(`/tx/${v}`);
    else if (/^\d+$/.test(v)) router.push(`/block/${v}`);
    else router.push(`/address/${v}`);
  }

  return (
    <div style={{ minHeight: 'calc(100vh - 52px)', display: 'flex', flexDirection: 'column' }}>

      {/* ── Hero ─────────────────────────────────────────────── */}
      <div style={{
        flex: 1,
        display: 'flex', flexDirection: 'column',
        alignItems: 'center', justifyContent: 'center',
        padding: '80px 24px 60px',
        gap: 40,
        background: `
          radial-gradient(ellipse 70% 40% at 50% 0%, hsla(var(--primary),.06) 0%, transparent 70%),
          hsl(var(--background))
        `,
      }}>

        {/* Logo mark */}
        <div style={{ textAlign: 'center' }}>
          <div style={{
            fontFamily: 'var(--font-mono)',
            fontWeight: 700,
            fontSize: 52,
            letterSpacing: '0.08em',
            color: 'hsl(var(--primary))',
            lineHeight: 1,
            marginBottom: 12,
            textShadow: '0 0 60px hsla(var(--primary),.3)',
          }}>
            ◈ ATLAS
          </div>
          <p style={{
            fontFamily: 'var(--font-sans)',
            fontSize: 14,
            color: 'hsl(var(--foreground-tertiary))',
            margin: 0,
            letterSpacing: '0.04em',
          }}>
            X1 Blockchain Explorer — powered by Tachyon
          </p>
        </div>

        {/* Search */}
        <div style={{ width: '100%', maxWidth: 640 }}>
          <form
            onSubmit={e => { e.preventDefault(); navigate(query); }}
            style={{
              display: 'flex',
              border: '1px solid hsl(var(--border-strong))',
              background: 'hsl(var(--card))',
              transition: 'border-color .15s',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', paddingLeft: 14, color: 'hsl(var(--foreground-tertiary))' }}>
              <Search size={14} />
            </div>
            <input
              type="text"
              value={query}
              onChange={e => setQuery(e.target.value)}
              placeholder="Search by address · transaction · block…"
              autoFocus
              style={{
                flex: 1,
                height: 46,
                padding: '0 12px',
                background: 'transparent',
                border: 'none',
                borderRadius: 0,
                fontFamily: 'var(--font-mono)',
                fontSize: 12,
                color: 'hsl(var(--foreground))',
                outline: 'none',
              }}
            />
            <button
              type="submit"
              style={{
                height: 46, width: 52,
                background: 'hsl(var(--primary))',
                border: 'none',
                borderLeft: '1px solid hsl(var(--primary-dim))',
                color: 'hsl(220 17% 4%)',
                cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontFamily: 'var(--font-mono)',
                fontSize: 10,
                fontWeight: 700,
                letterSpacing: '.1em',
                transition: 'background .15s',
              }}
            >
              GO
            </button>
          </form>

          {/* Examples */}
          <div style={{ marginTop: 10, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.08em' }}>
              TRY:
            </span>
            {EXAMPLES.map(ex => (
              <button
                key={ex}
                onClick={() => { setQuery(ex); navigate(ex); }}
                style={{
                  fontFamily: 'var(--font-mono)', fontSize: 9,
                  color: 'hsl(var(--foreground-tertiary))',
                  background: 'none', border: 'none', cursor: 'pointer', padding: 0,
                  letterSpacing: '.04em',
                  textDecoration: 'underline', textDecorationColor: 'hsl(var(--border-strong))',
                  textUnderlineOffset: 3,
                  transition: 'color .15s',
                }}
                onMouseEnter={e => (e.currentTarget.style.color = 'hsl(var(--primary))')}
                onMouseLeave={e => (e.currentTarget.style.color = 'hsl(var(--foreground-tertiary))')}
              >
                {ex.slice(0, 20)}…
              </button>
            ))}
          </div>
        </div>

        {/* Feature grid */}
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(4, 1fr)',
          width: '100%', maxWidth: 800,
          border: '1px solid hsl(var(--border))',
        }}>
          {FEATURES.map(({ icon: Icon, label, desc }, i) => (
            <div key={label} style={{
              padding: '20px 18px',
              borderRight: i < 3 ? '1px solid hsl(var(--border))' : 'none',
              background: 'hsl(var(--card))',
              transition: 'background .15s',
            }}
              onMouseEnter={e => (e.currentTarget.style.background = 'hsl(var(--card-hover))')}
              onMouseLeave={e => (e.currentTarget.style.background = 'hsl(var(--card))')}
            >
              <Icon size={14} color="hsl(var(--primary))" style={{ marginBottom: 8 }} />
              <div style={{ fontFamily: 'var(--font-mono)', fontSize: 9, fontWeight: 700, letterSpacing: '.12em', color: 'hsl(var(--foreground-secondary))', marginBottom: 6 }}>
                {label}
              </div>
              <div style={{ fontFamily: 'var(--font-sans)', fontSize: 11, color: 'hsl(var(--foreground-tertiary))', lineHeight: 1.5 }}>
                {desc}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* ── Footer strip ──────────────────────────────────────── */}
      <div style={{
        borderTop: '1px solid hsl(var(--border))',
        padding: '12px 32px',
        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
        background: 'hsl(var(--background-secondary))',
      }}>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.08em' }}>
          ATLAS EXPLORER — X1 NETWORK
        </span>
        <div style={{ display: 'flex', gap: 16 }}>
          {[['STATS', '/stats'], ['TRACE', '/trace'], ['VALIDATORS', '/stats#validators']].map(([label, href]) => (
            <a key={label} href={href} style={{
              fontFamily: 'var(--font-mono)', fontSize: 9,
              color: 'hsl(var(--foreground-muted))',
              textDecoration: 'none', letterSpacing: '.08em',
              transition: 'color .15s',
            }}
              onMouseEnter={e => (e.currentTarget.style.color = 'hsl(var(--primary))')}
              onMouseLeave={e => (e.currentTarget.style.color = 'hsl(var(--foreground-muted))')}
            >
              {label}
            </a>
          ))}
        </div>
      </div>
    </div>
  );
}

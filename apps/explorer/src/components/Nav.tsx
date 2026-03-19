'use client';

import { useState, useRef, useEffect } from 'react';
import { useRouter, usePathname } from 'next/navigation';

const LINKS = [
  { href: '/',       label: 'HOME'   },
  { href: '/stats',  label: 'STATS'  },
  { href: '/trace',  label: 'TRACE'  },
];

export default function Nav() {
  const router   = useRouter();
  const pathname = usePathname();

  const [query, setQuery]   = useState('');
  const [focused, setFocused] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  /* keyboard shortcut: / focuses search */
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === '/' && document.activeElement?.tagName !== 'INPUT') {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  function handleSearch(e: React.FormEvent) {
    e.preventDefault();
    const q = query.trim();
    if (!q) return;

    /* route by length / prefix heuristics */
    if (q.length === 88 || q.length === 87) {
      router.push(`/tx/${q}`);
    } else if (q.length >= 32 && q.length <= 44) {
      router.push(`/address/${q}`);
    } else if (/^\d+$/.test(q)) {
      router.push(`/block/${q}`);
    } else {
      router.push(`/address/${q}`);
    }
    setQuery('');
  }

  const isActive = (href: string) => {
    if (href === '/') return pathname === '/';
    return pathname.startsWith(href);
  };

  return (
    <header style={{
      position: 'sticky', top: 0, zIndex: 200,
      background: 'hsla(var(--background),.97)',
      backdropFilter: 'blur(12px)',
      borderBottom: '1px solid hsl(var(--border))',
      height: 52,
      display: 'flex', alignItems: 'center',
      padding: '0 24px', gap: 0,
    }}>
      {/* Logo */}
      <a href="/" style={{
        fontFamily: 'var(--font-mono)',
        fontWeight: 700,
        fontSize: 15,
        letterSpacing: '0.15em',
        color: 'hsl(var(--primary))',
        textDecoration: 'none',
        marginRight: 8,
        flexShrink: 0,
      }}>
        ◈ ATLAS
      </a>

      {/* Chain badge */}
      <span style={{
        fontFamily: 'var(--font-mono)',
        fontSize: 9,
        letterSpacing: '0.1em',
        color: 'hsl(var(--foreground-tertiary))',
        border: '1px solid hsl(var(--border-strong))',
        padding: '1px 6px',
        marginRight: 24,
        flexShrink: 0,
      }}>
        X1
      </span>

      {/* Nav links */}
      <nav style={{ display: 'flex', height: '100%', gap: 0 }}>
        {LINKS.map(({ href, label }) => (
          <a key={href} href={href} style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            letterSpacing: '0.12em',
            color: isActive(href) ? 'hsl(var(--foreground))' : 'hsl(var(--foreground-tertiary))',
            fontWeight: isActive(href) ? 600 : 400,
            textDecoration: 'none',
            height: '100%',
            display: 'flex', alignItems: 'center',
            padding: '0 14px',
            borderBottom: isActive(href)
              ? '2px solid hsl(var(--primary))'
              : '2px solid transparent',
            transition: 'color 0.15s',
          }}>
            {label}
          </a>
        ))}
      </nav>

      <div style={{ flex: 1 }} />

      {/* Search */}
      <form onSubmit={handleSearch} style={{ position: 'relative', width: 320 }}>
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          placeholder="Search address · tx · block…"
          style={{
            width: '100%',
            height: 32,
            padding: '0 80px 0 12px',
            fontFamily: 'var(--font-mono)',
            fontSize: 11,
            background: 'hsl(var(--card))',
            color: 'hsl(var(--foreground))',
            border: `1px solid ${focused ? 'hsl(var(--primary))' : 'hsl(var(--border-strong))'}`,
            borderRadius: 0,
            outline: 'none',
            transition: 'border-color 0.15s',
          }}
        />
        <kbd style={{
          position: 'absolute', right: 40, top: '50%', transform: 'translateY(-50%)',
          fontFamily: 'var(--font-mono)', fontSize: 9,
          color: 'hsl(var(--foreground-tertiary))',
          border: '1px solid hsl(var(--border-strong))',
          padding: '1px 5px', pointerEvents: 'none',
          display: query ? 'none' : 'block',
        }}>/</kbd>
        <button type="submit" style={{
          position: 'absolute', right: 0, top: 0, bottom: 0,
          width: 36,
          background: 'hsl(var(--card-hover))',
          border: 'none',
          borderLeft: '1px solid hsl(var(--border-strong))',
          color: 'hsl(var(--foreground-tertiary))',
          cursor: 'pointer',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
          transition: 'background 0.15s, color 0.15s',
        }}
          onMouseEnter={e => {
            (e.currentTarget as HTMLButtonElement).style.background = 'hsl(var(--primary-dim))';
            (e.currentTarget as HTMLButtonElement).style.color = 'hsl(var(--primary))';
          }}
          onMouseLeave={e => {
            (e.currentTarget as HTMLButtonElement).style.background = 'hsl(var(--card-hover))';
            (e.currentTarget as HTMLButtonElement).style.color = 'hsl(var(--foreground-tertiary))';
          }}
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
          </svg>
        </button>
      </form>
    </header>
  );
}

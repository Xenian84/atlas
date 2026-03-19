'use client';
import { useState, useEffect, useCallback } from 'react';
import { getVoteAccounts, type VoteAccount } from '@/lib/atlasRpc';

const MY_VALIDATOR = process.env.NEXT_PUBLIC_MY_VALIDATOR ?? '';

function calcApy(v: VoteAccount): number {
  if (!v.epochCredits || v.epochCredits.length < 2) return 0;
  const recent  = v.epochCredits.slice(-4);
  const earned  = recent.reduce((sum, [, cur, prev]) => sum + (cur - prev), 0);
  const possible = recent.length * 432_000;
  const rate    = possible > 0 ? earned / possible : 0;
  return Math.round(rate * 6.5 * (1 - v.commission / 100) * 10) / 10;
}

const shorten = (a: string) => `${a.slice(0, 5)}…${a.slice(-5)}`;

function StakeBar({ weight }: { weight: number }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <div style={{ flex: 1, height: 3, background: 'hsl(var(--border))', overflow: 'hidden', minWidth: 40 }}>
        <div style={{ width: `${Math.min(100, weight * 100)}%`, height: '100%', background: 'hsl(var(--accent-blue))' }} />
      </div>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-tertiary))', width: 38, textAlign: 'right' }}>
        {(weight * 100).toFixed(2)}%
      </span>
    </div>
  );
}

export default function ValidatorTable() {
  const [validators, setValidators] = useState<VoteAccount[]>([]);
  const [total, setTotal]           = useState(0);
  const [loading, setLoading]       = useState(true);
  const [showActive, setShowActive] = useState(true);
  const [page, setPage]             = useState(0);
  const perPage = 10;

  const load = useCallback(async () => {
    try {
      const { current, delinquent } = await getVoteAccounts();
      const all = showActive ? current : [...current, ...delinquent];
      all.sort((a, b) => b.activatedStake - a.activatedStake);
      setTotal(all.length);
      setValidators(all);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, [showActive]);

  useEffect(() => { load(); }, [load]);

  const totalStake = validators.reduce((s, v) => s + v.activatedStake, 0);
  const paged      = validators.slice(page * perPage, (page + 1) * perPage);
  const pageCount  = Math.ceil(validators.length / perPage);

  return (
    <div className="atlas-card" style={{ padding: '20px 0 0' }}>
      {/* Header */}
      <div style={{ padding: '0 20px 14px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', borderBottom: '1px solid hsl(var(--border))' }}>
        <div>
          <span className="statlabel">VALIDATORS</span>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 20, fontWeight: 700, color: 'hsl(var(--foreground))', lineHeight: 1, marginTop: 8 }}>
            {total.toLocaleString()}
            <span style={{ fontSize: 11, color: 'hsl(var(--foreground-tertiary))', fontWeight: 400, marginLeft: 6 }}>
              {showActive ? 'active' : 'total'}
            </span>
          </div>
        </div>
        <button
          onClick={() => { setShowActive(a => !a); setPage(0); }}
          style={{
            background: showActive ? 'hsla(var(--accent-green),.1)' : 'hsla(var(--accent-red),.1)',
            border: `1px solid ${showActive ? 'hsla(var(--accent-green),.35)' : 'hsla(var(--accent-red),.35)'}`,
            padding: '5px 10px',
            color: showActive ? 'hsl(var(--accent-green))' : 'hsl(var(--accent-red))',
            fontFamily: 'var(--font-mono)', fontSize: 9, cursor: 'pointer', letterSpacing: '.1em',
            transition: 'all .15s',
          }}
        >
          {showActive ? 'ACTIVE ONLY' : 'ALL'}
        </button>
      </div>

      {loading ? (
        <div style={{ padding: '12px 20px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {[...Array(6)].map((_, i) => <div key={i} className="skeleton" style={{ height: 32 }} />)}
        </div>
      ) : (
        <table className="atlas-table">
          <thead>
            <tr>
              <th style={{ width: 40 }}>#</th>
              <th>VALIDATOR</th>
              <th>APY</th>
              <th>STAKE</th>
              <th>WEIGHT</th>
              <th>COMM</th>
              <th style={{ textAlign: 'right' }}>LAST VOTE</th>
            </tr>
          </thead>
          <tbody>
            {paged.map((v, i) => {
              const rank       = page * perPage + i + 1;
              const stake      = (v.activatedStake / 1e9).toFixed(0);
              const weight     = totalStake > 0 ? v.activatedStake / totalStake : 0;
              const apy        = calcApy(v);
              const isMe       = MY_VALIDATOR && (v.nodePubkey === MY_VALIDATOR || v.votePubkey === MY_VALIDATOR);
              return (
                <tr key={v.votePubkey}>
                  <td style={{ color: 'hsl(var(--foreground-muted))', width: 40 }}>{rank}</td>
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <a href={`/address/${v.nodePubkey}`} style={{ color: 'hsl(var(--primary))', textDecoration: 'none', fontFamily: 'var(--font-mono)', fontSize: 11 }}>
                        {shorten(v.nodePubkey)}
                      </a>
                      {isMe && (
                        <span style={{
                          fontFamily: 'var(--font-mono)', fontSize: 8, letterSpacing: '.1em',
                          background: 'hsla(var(--primary),.15)', color: 'hsl(var(--primary))',
                          border: '1px solid hsla(var(--primary),.35)', padding: '1px 5px',
                        }}>YOU</span>
                      )}
                    </div>
                    <div style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
                      {shorten(v.votePubkey)}
                    </div>
                  </td>
                  <td style={{ color: apy > 0 ? 'hsl(var(--accent-green))' : 'hsl(var(--foreground-muted))', fontWeight: 600 }}>
                    {apy > 0 ? `${apy}%` : '—'}
                  </td>
                  <td>{Number(stake).toLocaleString()} XNT</td>
                  <td style={{ minWidth: 120 }}><StakeBar weight={weight} /></td>
                  <td style={{ color: v.commission > 10 ? 'hsl(var(--accent-amber))' : 'hsl(var(--foreground-secondary))' }}>
                    {v.commission}%
                  </td>
                  <td style={{ textAlign: 'right', color: 'hsl(var(--foreground-tertiary))' }}>
                    {v.lastVote?.toLocaleString() ?? '—'}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}

      {/* Pagination */}
      {pageCount > 1 && (
        <div style={{ padding: '10px 20px', borderTop: '1px solid hsl(var(--border))', display: 'flex', alignItems: 'center', gap: 8 }}>
          <button
            disabled={page === 0}
            onClick={() => setPage(p => p - 1)}
            style={{
              fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em',
              background: 'hsl(var(--background-secondary))', border: '1px solid hsl(var(--border))',
              color: page === 0 ? 'hsl(var(--foreground-muted))' : 'hsl(var(--foreground-secondary))',
              padding: '4px 10px', cursor: page === 0 ? 'default' : 'pointer',
            }}
          >
            ← PREV
          </button>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
            {page + 1} / {pageCount}
          </span>
          <button
            disabled={page >= pageCount - 1}
            onClick={() => setPage(p => p + 1)}
            style={{
              fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em',
              background: 'hsl(var(--background-secondary))', border: '1px solid hsl(var(--border))',
              color: page >= pageCount - 1 ? 'hsl(var(--foreground-muted))' : 'hsl(var(--foreground-secondary))',
              padding: '4px 10px', cursor: page >= pageCount - 1 ? 'default' : 'pointer',
            }}
          >
            NEXT →
          </button>
          <span style={{ marginLeft: 'auto', fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
            {validators.length} validators total
          </span>
        </div>
      )}
    </div>
  );
}

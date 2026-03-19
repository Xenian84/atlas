'use client';
import { useState, useEffect, useCallback } from 'react';
import { clientFetch } from '@/lib/api';

const MY_VALIDATOR = process.env.NEXT_PUBLIC_MY_VALIDATOR ?? '';

interface Validator {
  votePubkey:     string;
  nodePubkey:     string;
  activatedStake: number;
  stakeXnt:       number;
  weight:         number;
  commission:     number;
  lastVote:       number;
  apy:            number;
  delinquent:     boolean;
}

interface ValidatorsResponse {
  validators:        Validator[];
  total_active:      number;
  total_delinquent:  number;
  total_stake_xnt:   number;
}

const shorten = (a: string) => `${a.slice(0, 5)}…${a.slice(-5)}`;

function StakeBar({ weight }: { weight: number }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <div style={{ flex: 1, height: 3, background: 'hsl(var(--border))', overflow: 'hidden', minWidth: 60 }}>
        <div style={{ width: `${Math.min(100, weight * 100)}%`, height: '100%', background: 'hsl(var(--accent-blue))' }} />
      </div>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-tertiary))', width: 42, textAlign: 'right' }}>
        {(weight * 100).toFixed(2)}%
      </span>
    </div>
  );
}

export default function ValidatorTable() {
  const [data, setData]         = useState<ValidatorsResponse | null>(null);
  const [loading, setLoading]   = useState(true);
  const [page, setPage]         = useState(0);
  const [search, setSearch]     = useState('');
  const perPage = 15;

  const load = useCallback(async () => {
    try {
      const res = await clientFetch<ValidatorsResponse>('/v1/network/validators?limit=150');
      setData(res);
    } catch { /* silent */ }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { load(); }, [load]);

  const filtered = (data?.validators ?? []).filter(v =>
    !search || v.nodePubkey.includes(search) || v.votePubkey.includes(search)
  );
  const pageCount = Math.ceil(filtered.length / perPage);
  const paged     = filtered.slice(page * perPage, (page + 1) * perPage);

  return (
    <div className="atlas-card" style={{ padding: '20px 0 0' }}>
      {/* Header */}
      <div style={{ padding: '0 20px 14px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: 10, borderBottom: '1px solid hsl(var(--border))' }}>
        <div>
          <span className="statlabel">VALIDATORS</span>
          <div style={{ fontFamily: 'var(--font-mono)', fontSize: 20, fontWeight: 700, color: 'hsl(var(--foreground))', lineHeight: 1, marginTop: 8 }}>
            {data ? data.total_active.toLocaleString() : '—'}
            <span style={{ fontSize: 11, color: 'hsl(var(--foreground-tertiary))', fontWeight: 400, marginLeft: 6 }}>active</span>
            {data && (
              <span style={{ fontSize: 11, color: 'hsl(var(--foreground-muted))', fontWeight: 400, marginLeft: 12 }}>
                {data.total_delinquent} delinquent · {(data.total_stake_xnt / 1e6).toFixed(1)}M XNT staked
              </span>
            )}
          </div>
        </div>
        {/* Search */}
        <input
          type="text"
          placeholder="Filter by pubkey…"
          value={search}
          onChange={e => { setSearch(e.target.value); setPage(0); }}
          style={{
            fontFamily: 'var(--font-mono)', fontSize: 10,
            background: 'hsl(var(--background-secondary))',
            border: '1px solid hsl(var(--border-strong))',
            color: 'hsl(var(--foreground))',
            padding: '5px 10px', outline: 'none', width: 220,
          }}
        />
      </div>

      {loading ? (
        <div style={{ padding: '12px 20px', display: 'flex', flexDirection: 'column', gap: 8 }}>
          {[...Array(8)].map((_, i) => <div key={i} className="skeleton" style={{ height: 36 }} />)}
        </div>
      ) : (
        <table className="atlas-table">
          <thead>
            <tr>
              <th style={{ width: 36 }}>#</th>
              <th>VALIDATOR</th>
              <th style={{ textAlign: 'right' }}>APY</th>
              <th style={{ textAlign: 'right' }}>STAKE (XNT)</th>
              <th style={{ minWidth: 110 }}>WEIGHT</th>
              <th style={{ textAlign: 'center' }}>COMM</th>
              <th style={{ textAlign: 'right' }}>LAST VOTE</th>
            </tr>
          </thead>
          <tbody>
            {paged.map((v, i) => {
              const rank  = page * perPage + i + 1;
              const isMe  = MY_VALIDATOR && (v.nodePubkey === MY_VALIDATOR || v.votePubkey === MY_VALIDATOR);
              return (
                <tr key={v.votePubkey} style={{ background: isMe ? 'hsla(var(--primary),.04)' : undefined }}>
                  <td style={{ color: 'hsl(var(--foreground-muted))' }}>{rank}</td>
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                      <a href={`/address/${v.nodePubkey}`} style={{ color: 'hsl(var(--primary))', textDecoration: 'none', fontFamily: 'var(--font-mono)', fontSize: 11 }}>
                        {shorten(v.nodePubkey)}
                      </a>
                      {isMe && (
                        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 8, letterSpacing: '.1em', background: 'hsla(var(--primary),.15)', color: 'hsl(var(--primary))', border: '1px solid hsla(var(--primary),.35)', padding: '1px 5px' }}>
                          YOU
                        </span>
                      )}
                    </div>
                    <div style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
                      vote: {shorten(v.votePubkey)}
                    </div>
                  </td>
                  <td style={{ textAlign: 'right', color: v.apy > 0 ? 'hsl(var(--accent-green))' : 'hsl(var(--foreground-muted))', fontWeight: 600 }}>
                    {v.apy > 0 ? `${v.apy}%` : '—'}
                  </td>
                  <td style={{ textAlign: 'right' }}>
                    {v.stakeXnt.toLocaleString()}
                  </td>
                  <td><StakeBar weight={v.weight} /></td>
                  <td style={{ textAlign: 'center', color: v.commission > 10 ? 'hsl(var(--accent-amber))' : 'hsl(var(--foreground-secondary))' }}>
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

      {/* Pagination + summary */}
      {!loading && pageCount > 1 && (
        <div style={{ padding: '10px 20px', borderTop: '1px solid hsl(var(--border))', display: 'flex', alignItems: 'center', gap: 8 }}>
          <button disabled={page === 0} onClick={() => setPage(p => p - 1)}
            style={{ fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em', background: 'hsl(var(--background-secondary))', border: '1px solid hsl(var(--border))', color: page === 0 ? 'hsl(var(--foreground-muted))' : 'hsl(var(--foreground-secondary))', padding: '4px 10px', cursor: page === 0 ? 'default' : 'pointer' }}>
            ← PREV
          </button>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
            {page + 1} / {pageCount}
          </span>
          <button disabled={page >= pageCount - 1} onClick={() => setPage(p => p + 1)}
            style={{ fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em', background: 'hsl(var(--background-secondary))', border: '1px solid hsl(var(--border))', color: page >= pageCount - 1 ? 'hsl(var(--foreground-muted))' : 'hsl(var(--foreground-secondary))', padding: '4px 10px', cursor: page >= pageCount - 1 ? 'default' : 'pointer' }}>
            NEXT →
          </button>
          <span style={{ marginLeft: 'auto', fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))' }}>
            showing top 150 of {data?.total_active.toLocaleString()} by stake
          </span>
        </div>
      )}
    </div>
  );
}

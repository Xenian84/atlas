'use client';

import { useState, useEffect, useCallback } from 'react';
import { getVoteAccounts, type VoteAccount } from '@/lib/atlasRpc';

function calcApy(v: VoteAccount): number {
  if (!v.epochCredits || v.epochCredits.length < 2) return 0;
  const recent = v.epochCredits.slice(-4);
  const earned = recent.reduce((sum, [, cur, prev]) => sum + (cur - prev), 0);
  const possible = recent.length * 432_000;
  const rate = possible > 0 ? earned / possible : 0;
  // Rough APY: 6.5% base inflation * vote rate * (1 - commission/100)
  return Math.round(rate * 6.5 * (1 - v.commission / 100) * 10) / 10;
}

function shorten(addr: string) {
  return `${addr.slice(0, 5)}…${addr.slice(-5)}`;
}

function StakeBar({ weight }: { weight: number }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <div style={{ flex: 1, height: 4, background: 'rgba(255,255,255,0.06)', borderRadius: 2, overflow: 'hidden', minWidth: 40 }}>
        <div style={{ width: `${Math.min(100, weight * 100)}%`, height: '100%', background: '#89b4fa', borderRadius: 2 }} />
      </div>
      <span style={{ fontSize: 10, color: '#6c7086', fontFamily: 'monospace', width: 38, textAlign: 'right' }}>
        {(weight * 100).toFixed(2)}%
      </span>
    </div>
  );
}

const COLS = ['RANK', 'VALIDATOR', 'APY', 'ACTIVE STAKE', 'WEIGHT', 'COMMISSION', 'LAST VOTE'];

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
  const paged = validators.slice(page * perPage, (page + 1) * perPage);
  const pageCount = Math.ceil(validators.length / perPage);

  return (
    <div style={{ background: 'rgba(10,15,25,0.8)', border: '1px solid rgba(255,255,255,0.08)', borderRadius: 10, padding: '20px 22px' }}>
      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
        <div>
          <div style={{ fontSize: 10, letterSpacing: '0.12em', color: '#45475a', textTransform: 'uppercase', fontFamily: 'monospace', marginBottom: 4 }}>
            ACTIVE VALIDATORS
          </div>
          <div style={{ fontSize: 18, fontWeight: 700, color: '#cdd6f4', fontFamily: 'monospace' }}>
            {total.toLocaleString()}
            <span style={{ fontSize: 11, color: '#45475a', fontWeight: 400, marginLeft: 6 }}>validators</span>
          </div>
        </div>
        <button
          onClick={() => { setShowActive(a => !a); setPage(0); }}
          style={{
            background: showActive ? 'rgba(166,227,161,0.1)' : 'rgba(243,139,168,0.1)',
            border: `1px solid ${showActive ? 'rgba(166,227,161,0.3)' : 'rgba(243,139,168,0.3)'}`,
            borderRadius: 6, padding: '5px 12px',
            color: showActive ? '#a6e3a1' : '#f38ba8',
            fontFamily: 'monospace', fontSize: 10, cursor: 'pointer', letterSpacing: '0.08em',
          }}
        >
          {showActive ? 'Active only' : 'Show all'}
        </button>
      </div>

      {/* Table */}
      {loading ? (
        <div style={{ color: '#45475a', fontSize: 12, fontFamily: 'monospace', textAlign: 'center', padding: '40px 0' }}>Loading validators…</div>
      ) : (
        <>
          {/* Header row */}
          <div style={{ display: 'grid', gridTemplateColumns: '40px 1fr 70px 110px 140px 90px 90px', gap: 8, padding: '0 0 8px', borderBottom: '1px solid rgba(255,255,255,0.07)', marginBottom: 4 }}>
            {COLS.map(c => (
              <span key={c} style={{ fontSize: 9, color: '#45475a', fontFamily: 'monospace', letterSpacing: '0.1em', textTransform: 'uppercase' }}>{c}</span>
            ))}
          </div>

          {paged.map((v, i) => {
            const weight = totalStake > 0 ? v.activatedStake / totalStake : 0;
            const apy = calcApy(v);
            const isOurs = v.nodePubkey === '6YLpqj2PQiy12cG8YzUZ6hspoGY5RixQp7wM5d4veXp5';
            return (
              <div
                key={v.votePubkey}
                style={{
                  display: 'grid', gridTemplateColumns: '40px 1fr 70px 110px 140px 90px 90px',
                  gap: 8, padding: '9px 0',
                  borderBottom: '1px solid rgba(255,255,255,0.03)',
                  background: isOurs ? 'rgba(0,229,255,0.03)' : 'transparent',
                  borderLeft: isOurs ? '2px solid #00e5ff' : '2px solid transparent',
                  paddingLeft: isOurs ? 6 : 0,
                }}
              >
                <span style={{ color: '#45475a', fontFamily: 'monospace', fontSize: 12 }}>
                  {page * perPage + i + 1}
                </span>
                <div>
                  <div style={{ color: '#cdd6f4', fontFamily: 'monospace', fontSize: 11, fontWeight: 600 }}>
                    {shorten(v.nodePubkey)}
                    {isOurs && <span style={{ marginLeft: 6, fontSize: 9, color: '#00e5ff', background: 'rgba(0,229,255,0.1)', border: '1px solid rgba(0,229,255,0.3)', borderRadius: 3, padding: '1px 5px' }}>YOU</span>}
                  </div>
                  <div style={{ color: '#45475a', fontFamily: 'monospace', fontSize: 9 }}>{shorten(v.votePubkey)}</div>
                </div>
                <span style={{ color: apy > 6 ? '#a6e3a1' : '#cdd6f4', fontFamily: 'monospace', fontSize: 12, fontWeight: 600 }}>
                  {apy > 0 ? `${apy}%` : '—'}
                </span>
                <span style={{ color: '#a6adc8', fontFamily: 'monospace', fontSize: 11 }}>
                  {(v.activatedStake / 1e9).toLocaleString(undefined, { maximumFractionDigits: 0 })} ◎
                </span>
                <StakeBar weight={weight} />
                <span style={{ color: v.commission > 10 ? '#f38ba8' : '#cdd6f4', fontFamily: 'monospace', fontSize: 12 }}>
                  {v.commission}%
                </span>
                <span style={{ color: '#6c7086', fontFamily: 'monospace', fontSize: 11 }}>
                  {v.lastVote.toLocaleString()}
                </span>
              </div>
            );
          })}

          {/* Pagination */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 14, fontFamily: 'monospace', fontSize: 11 }}>
            <span style={{ color: '#45475a' }}>
              Showing {page * perPage + 1}–{Math.min((page + 1) * perPage, validators.length)} of {validators.length}
            </span>
            <div style={{ display: 'flex', gap: 6 }}>
              <PageBtn label="←" disabled={page === 0} onClick={() => setPage(p => p - 1)} />
              <PageBtn label="→" disabled={page >= pageCount - 1} onClick={() => setPage(p => p + 1)} />
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function PageBtn({ label, disabled, onClick }: { label: string; disabled: boolean; onClick: () => void }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{
        background: 'rgba(255,255,255,0.04)', border: '1px solid rgba(255,255,255,0.1)',
        borderRadius: 5, padding: '4px 12px', color: disabled ? '#313244' : '#a6adc8',
        fontFamily: 'monospace', fontSize: 11, cursor: disabled ? 'not-allowed' : 'pointer',
      }}
    >
      {label}
    </button>
  );
}

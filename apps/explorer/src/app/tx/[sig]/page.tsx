import { fetchTx, abbrev, lamportsToXnt, type TxFacts } from '@/lib/api';
import { ActionCard } from '@/components/ActionCard';
import { TokenDeltaTable } from '@/components/TokenDeltaTable';
import { TagBadge } from '@/components/TagBadge';
import { TxActions } from '@/components/TxActions';

export default async function TxPage({ params }: { params: Promise<{ sig: string }> }) {
  const { sig: sigParam } = await params;
  let facts: TxFacts | null = null;
  let error: string | null  = null;

  try { facts = await fetchTx(sigParam); }
  catch (e: unknown) { error = e instanceof Error ? e.message : 'Failed to load transaction'; }

  if (error || !facts) {
    return (
      <div style={{ maxWidth: 1200, margin: '0 auto', padding: '28px 24px' }}>
        <div className="atlas-card" style={{
          padding: '20px',
          borderLeft: '3px solid hsl(var(--accent-red))',
          fontFamily: 'var(--font-mono)', fontSize: 12,
          color: 'hsl(var(--accent-red))',
        }}>
          {error ?? 'Transaction not found'}
        </div>
      </div>
    );
  }

  const sig       = sigParam;
  const isSuccess = facts.status === 'success';

  return (
    <div style={{ maxWidth: 1200, margin: '0 auto', padding: '28px 24px', display: 'flex', flexDirection: 'column', gap: 14 }}>

      {/* ── Header ──────────────────────────────────────────── */}
      <div className="atlas-card" style={{ padding: '16px 20px' }}>
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 12 }}>
          <div style={{ flex: 1 }}>
            <span className="statlabel" style={{ marginBottom: 8, display: 'inline-block' }}>TRANSACTION</span>
            <div style={{ fontFamily: 'var(--font-mono)', fontSize: 12, color: 'hsl(var(--foreground))', wordBreak: 'break-all', marginTop: 6, lineHeight: 1.6 }}>
              {sig}
            </div>
          </div>
          <span style={{
            fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.1em', flexShrink: 0,
            padding: '5px 10px',
            background: isSuccess ? 'hsla(var(--accent-green),.1)' : 'hsla(var(--accent-red),.1)',
            border: `1px solid ${isSuccess ? 'hsla(var(--accent-green),.3)' : 'hsla(var(--accent-red),.3)'}`,
            color: isSuccess ? 'hsl(var(--accent-green))' : 'hsl(var(--accent-red))',
          }}>
            {isSuccess ? '✓ SUCCESS' : '✗ FAILED'}
          </span>
        </div>
      </div>

      {/* ── Meta grid ───────────────────────────────────────── */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', border: '1px solid hsl(var(--border))' }}>
        {([
          ['SLOT',       facts.slot.toLocaleString()],
          ['BLOCK TIME', facts.block_time ? new Date(facts.block_time * 1000).toLocaleString() : '—'],
          ['FEE',        `${lamportsToXnt(facts.fee_lamports)} XNT`],
          ['COMPUTE',    facts.compute_units.consumed
            ? `${facts.compute_units.consumed.toLocaleString()} / ${(facts.compute_units.limit ?? 0).toLocaleString()}`
            : '—'],
        ] as [string, string][]).map(([label, value], i) => (
          <div key={label} style={{
            padding: '14px 16px',
            borderRight: i < 3 ? '1px solid hsl(var(--border))' : 'none',
            background: 'hsl(var(--card))',
          }}>
            <span className="statlabel" style={{ marginBottom: 8, display: 'inline-block' }}>{label}</span>
            <div style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'hsl(var(--foreground))', marginTop: 6 }}>
              {value}
            </div>
          </div>
        ))}
      </div>

      {/* ── Tags ────────────────────────────────────────────── */}
      {facts.tags.length > 0 && (
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
          {facts.tags.map(t => <TagBadge key={t} tag={t} />)}
        </div>
      )}

      {/* ── Actions ─────────────────────────────────────────── */}
      {facts.actions.length > 0 && (
        <Section label="ACTIONS">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: '12px 16px' }}>
            {facts.actions.map((a, i) => <ActionCard key={i} action={a} />)}
          </div>
        </Section>
      )}

      {/* ── Token deltas ────────────────────────────────────── */}
      {facts.token_deltas.length > 0 && (
        <Section label="TOKEN CHANGES">
          <TokenDeltaTable deltas={facts.token_deltas} />
        </Section>
      )}

      {/* ── XNT deltas ──────────────────────────────────────── */}
      {facts.xnt_deltas && facts.xnt_deltas.length > 0 && (
        <Section label="XNT BALANCE CHANGES">
          <table className="atlas-table">
            <thead>
              <tr>
                <th>ACCOUNT</th>
                <th style={{ textAlign: 'right' }}>BEFORE</th>
                <th style={{ textAlign: 'right' }}>AFTER</th>
                <th style={{ textAlign: 'right' }}>DELTA</th>
              </tr>
            </thead>
            <tbody>
              {facts.xnt_deltas.map((d, i) => (
                <tr key={i}>
                  <td>
                    <a href={`/address/${d.owner}`} style={{ color: 'hsl(var(--primary))', textDecoration: 'none', fontFamily: 'var(--font-mono)', fontSize: 11 }}>
                      {abbrev(d.owner, 10)}
                    </a>
                  </td>
                  <td style={{ textAlign: 'right' }}>{lamportsToXnt(d.pre_lamports)}</td>
                  <td style={{ textAlign: 'right' }}>{lamportsToXnt(d.post_lamports)}</td>
                  <td style={{ textAlign: 'right', color: d.delta_lamports > 0 ? 'hsl(var(--accent-green))' : 'hsl(var(--accent-red))', fontWeight: 600 }}>
                    {d.delta_lamports > 0 ? '+' : ''}{lamportsToXnt(d.delta_lamports)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
      )}

      {/* ── Programs ────────────────────────────────────────── */}
      {facts.programs.length > 0 && (
        <Section label="PROGRAMS INVOKED">
          <div style={{ padding: '10px 16px', display: 'flex', flexDirection: 'column', gap: 6 }}>
            {facts.programs.map(p => (
              <a key={p} href={`/address/${p}`} style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'hsl(var(--foreground-secondary))', textDecoration: 'none' }}>
                {p}
              </a>
            ))}
          </div>
        </Section>
      )}

      {/* ── AI Explain + Copy ───────────────────────────────── */}
      <TxActions sig={sig} />
    </div>
  );
}

function Section({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="atlas-card" style={{ overflow: 'hidden' }}>
      <div style={{ padding: '10px 16px', borderBottom: '1px solid hsl(var(--border))' }}>
        <span className="statlabel">{label}</span>
      </div>
      {children}
    </div>
  );
}

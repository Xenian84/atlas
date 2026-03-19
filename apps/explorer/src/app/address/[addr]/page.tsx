import Link from 'next/link';
import { fetchAddressTxs, fetchWalletProfile } from '@/lib/api';
import { WalletScoreBadge } from '@/components/WalletScoreBadge';
import { TxHistoryList } from '@/components/TxHistoryList';

const TX_TYPES = ['all', 'swap', 'transfer', 'balanceChanged'] as const;

function shorten(addr: string) {
  return `${addr.slice(0, 8)}…${addr.slice(-8)}`;
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', padding: '7px 0', borderBottom: '1px solid hsl(var(--border))' }}>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-tertiary))', letterSpacing: '.06em' }}>{label}</span>
      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'hsl(var(--foreground-secondary))' }}>{value}</span>
    </div>
  );
}

export default async function AddressPage({
  params,
  searchParams,
}: {
  params: Promise<{ addr: string }>;
  searchParams: Promise<{ before?: string; type?: string }>;
}) {
  const { addr }              = await params;
  const { before, type }      = await searchParams;
  const txType                = type ?? 'all';

  const [txPage, profile] = await Promise.allSettled([
    fetchAddressTxs(addr, before, 50, txType),
    fetchWalletProfile(addr, '7d'),
  ]);

  const page   = txPage.status  === 'fulfilled' ? txPage.value  : null;
  const walletP = profile.status === 'fulfilled' ? profile.value : null;

  return (
    <div style={{ maxWidth: 1200, margin: '0 auto', padding: '28px 24px', display: 'flex', flexDirection: 'column', gap: 16 }}>

      {/* ── Address header ─────────────────────────────────── */}
      <div className="atlas-card" style={{ padding: '16px 20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12, flexWrap: 'wrap' }}>
          <div>
            <span className="statlabel" style={{ marginBottom: 8, display: 'inline-block' }}>ADDRESS</span>
            <div style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'hsl(var(--foreground))', wordBreak: 'break-all', marginTop: 6 }}>
              {addr}
            </div>
          </div>
          <a
            href={`/trace/${addr}`}
            style={{
              fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.1em',
              background: 'hsla(var(--primary),.1)', border: '1px solid hsla(var(--primary),.3)',
              color: 'hsl(var(--primary))', padding: '6px 12px', textDecoration: 'none',
              whiteSpace: 'nowrap', transition: 'background .15s',
            }}
          >
            ◈ TRACE
          </a>
        </div>
      </div>

      {/* ── Wallet profile ──────────────────────────────────── */}
      {walletP && (
        <div className="atlas-card" style={{ padding: '16px 20px' }}>
          <span className="statlabel" style={{ marginBottom: 12, display: 'inline-block' }}>WALLET PROFILE</span>
          <div style={{ display: 'flex', alignItems: 'flex-start', gap: 24, flexWrap: 'wrap' }}>
            <div style={{ flex: 1 }}>
              <InfoRow label="TYPE"       value={walletP.wallet_type.replace('_', ' ').toUpperCase()} />
              <InfoRow label="CONFIDENCE" value={`${(walletP.confidence * 100).toFixed(0)}%`} />
              {walletP.updated_at && (
                <InfoRow label="UPDATED" value={new Date(walletP.updated_at).toLocaleString()} />
              )}
            </div>
            <div>
              <WalletScoreBadge scores={walletP.scores} />
            </div>
          </div>
        </div>
      )}

      {/* ── Filter bar ──────────────────────────────────────── */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 0, border: '1px solid hsl(var(--border))', width: 'fit-content' }}>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: 9, color: 'hsl(var(--foreground-muted))', letterSpacing: '.1em', padding: '6px 10px', borderRight: '1px solid hsl(var(--border))' }}>
          TYPE
        </span>
        {TX_TYPES.map(t => (
          <Link
            key={t}
            href={`/address/${addr}?type=${t}`}
            style={{
              fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em',
              padding: '6px 12px',
              background: txType === t ? 'hsl(var(--primary))' : 'transparent',
              color: txType === t ? 'hsl(220 17% 4%)' : 'hsl(var(--foreground-tertiary))',
              textDecoration: 'none',
              borderRight: t !== 'balanceChanged' ? '1px solid hsl(var(--border))' : 'none',
              transition: 'background .15s, color .15s',
            }}
          >
            {t.toUpperCase()}
          </Link>
        ))}
      </div>

      {/* ── Tx history ──────────────────────────────────────── */}
      {page ? (
        <TxHistoryList page={page} addr={addr} txType={txType} />
      ) : (
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'hsl(var(--foreground-muted))', padding: '20px 0' }}>
          No transactions found.
        </div>
      )}
    </div>
  );
}

import { notFound } from 'next/navigation';
import Link from 'next/link';
import { serverFetch, abbrev, lamportsToXnt } from '@/lib/api';

interface BlockTx {
  sig:          string;
  pos:          number;
  status:       'success' | 'failed';
  fee_lamports: number;
  tags:         string[];
}

interface BlockDetail {
  slot:          number;
  block_time:    number | null;
  tx_count:      number;
  success_count: number;
  failed_count:  number;
  total_fees:    number;
  programs:      string[];
  transactions:  BlockTx[];
}

function timeAgo(ts: number | null) {
  if (!ts) return '—';
  const d = Math.floor(Date.now() / 1000 - ts);
  if (d < 60)   return `${d}s ago`;
  if (d < 3600) return `${Math.floor(d / 60)}m ago`;
  return `${Math.floor(d / 3600)}h ago`;
}

function formatTs(ts: number | null) {
  if (!ts) return '—';
  return new Date(ts * 1000).toUTCString();
}

export default async function BlockPage({ params }: { params: Promise<{ slot: string }> }) {
  const { slot: slotStr } = await params;
  const slot = parseInt(slotStr, 10);

  if (isNaN(slot)) notFound();

  let block: BlockDetail;
  try {
    block = await serverFetch<BlockDetail>(`/v1/block/${slot}`);
  } catch {
    notFound();
  }

  const mono: React.CSSProperties = { fontFamily: 'var(--font-mono)' };
  const label: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.1em', color: 'hsl(var(--foreground-muted))', textTransform: 'uppercase' };
  const value: React.CSSProperties = { fontFamily: 'var(--font-mono)', fontSize: 13, color: 'hsl(var(--foreground))' };
  const cell: React.CSSProperties  = { padding: '14px 20px', borderBottom: '1px solid hsl(var(--border))' };

  return (
    <div style={{ maxWidth: 1100, margin: '0 auto', padding: '28px 24px', display: 'flex', flexDirection: 'column', gap: 20 }}>

      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, borderBottom: '1px solid hsl(var(--border))', paddingBottom: 14 }}>
        <Link href="/stats" style={{ ...mono, fontSize: 10, color: 'hsl(var(--foreground-muted))', textDecoration: 'none' }}>← Stats</Link>
        <span style={{ color: 'hsl(var(--border-strong))' }}>/</span>
        <span style={{ ...mono, fontSize: 10, color: 'hsl(var(--foreground-tertiary))' }}>Block</span>
        <span style={{ color: 'hsl(var(--border-strong))' }}>/</span>
        <span style={{ ...mono, fontSize: 13, color: 'hsl(var(--foreground))' }}>{slot.toLocaleString()}</span>
      </div>

      {/* Summary grid */}
      <div className="atlas-card" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', borderRadius: 0 }}>
        {[
          { label: 'Slot',          val: slot.toLocaleString() },
          { label: 'Block Time',    val: formatTs(block.block_time) },
          { label: 'Age',           val: timeAgo(block.block_time) },
          { label: 'Transactions',  val: block.tx_count.toLocaleString() },
          { label: 'Successful',    val: block.success_count.toLocaleString() },
          { label: 'Failed',        val: block.failed_count.toLocaleString() },
          { label: 'Total Fees',    val: `${lamportsToXnt(block.total_fees)} XNT` },
          { label: 'Programs',      val: `${block.programs.length} unique` },
        ].map(({ label: l, val }, i) => (
          <div key={l} style={{
            ...cell,
            borderRight: (i % 3 !== 2) ? '1px solid hsl(var(--border))' : 'none',
          }}>
            <div style={label}>{l}</div>
            <div style={{ ...value, marginTop: 4 }}>{val}</div>
          </div>
        ))}
      </div>

      {/* Programs */}
      {block.programs.length > 0 && (
        <div className="atlas-card" style={{ padding: 20, borderRadius: 0 }}>
          <div style={{ ...label, marginBottom: 10 }}>Programs Invoked</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            {block.programs.map(p => (
              <Link key={p} href={`/address/${p}`} style={{
                ...mono, fontSize: 10,
                padding: '3px 8px',
                background: 'hsl(var(--background-secondary))',
                border: '1px solid hsl(var(--border))',
                color: 'hsl(var(--primary))',
                textDecoration: 'none',
              }}>
                {abbrev(p, 6)}
              </Link>
            ))}
          </div>
        </div>
      )}

      {/* Transactions */}
      <div className="atlas-card" style={{ padding: '20px 0 0', borderRadius: 0 }}>
        <div style={{ padding: '0 20px 12px', borderBottom: '1px solid hsl(var(--border))', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={label}>Transactions ({block.tx_count > 20 ? `first 20 of ${block.tx_count}` : block.tx_count})</span>
        </div>
        <table className="atlas-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Signature</th>
              <th>Status</th>
              <th>Fee (XNT)</th>
              <th>Tags</th>
            </tr>
          </thead>
          <tbody>
            {block.transactions.map(tx => (
              <tr key={tx.sig}>
                <td style={{ color: 'hsl(var(--foreground-muted))', width: 36 }}>{tx.pos}</td>
                <td>
                  <Link href={`/tx/${tx.sig}`} style={{
                    ...mono, fontSize: 11,
                    color: 'hsl(var(--primary))',
                    textDecoration: 'none',
                  }}>
                    {abbrev(tx.sig, 10)}
                  </Link>
                </td>
                <td>
                  <span style={{
                    ...mono, fontSize: 9, padding: '2px 6px',
                    background: tx.status === 'success' ? 'hsla(var(--accent-green),.12)' : 'hsla(var(--accent-red),.12)',
                    color: tx.status === 'success' ? 'hsl(var(--accent-green))' : 'hsl(var(--accent-red))',
                    letterSpacing: '.06em',
                  }}>
                    {tx.status.toUpperCase()}
                  </span>
                </td>
                <td style={{ ...mono, fontSize: 11 }}>{lamportsToXnt(tx.fee_lamports)}</td>
                <td>
                  <div style={{ display: 'flex', gap: 4 }}>
                    {tx.tags.map(tag => (
                      <span key={tag} style={{
                        ...mono, fontSize: 9, padding: '2px 5px',
                        background: 'hsl(var(--background-secondary))',
                        border: '1px solid hsl(var(--border))',
                        color: 'hsl(var(--foreground-secondary))',
                      }}>
                        {tag}
                      </span>
                    ))}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Prev / Next navigation */}
      <div style={{ display: 'flex', justifyContent: 'space-between', paddingTop: 4 }}>
        <Link href={`/block/${slot - 1}`} style={{ ...mono, fontSize: 11, color: 'hsl(var(--foreground-secondary))', textDecoration: 'none' }}>
          ← Block {(slot - 1).toLocaleString()}
        </Link>
        <Link href={`/block/${slot + 1}`} style={{ ...mono, fontSize: 11, color: 'hsl(var(--foreground-secondary))', textDecoration: 'none' }}>
          Block {(slot + 1).toLocaleString()} →
        </Link>
      </div>
    </div>
  );
}

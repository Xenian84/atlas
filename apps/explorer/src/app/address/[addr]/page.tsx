import Link from 'next/link'
import { fetchAddressTxs, fetchWalletProfile, abbrev } from '@/lib/api'
import { WalletScoreBadge } from '@/components/WalletScoreBadge'
import { TxHistoryList } from '@/components/TxHistoryList'

export default async function AddressPage({
  params,
  searchParams,
}: {
  params: { addr: string }
  searchParams: { before?: string; type?: string }
}) {
  const addr   = params.addr
  const before = searchParams.before
  const txType = searchParams.type ?? 'all'

  const [txPage, profile] = await Promise.allSettled([
    // Pass txType to the server-side fetch so the filter is applied at the DB layer
    fetchAddressTxs(addr, before, 50, txType),
    fetchWalletProfile(addr, '7d'),
  ])

  const page    = txPage.status    === 'fulfilled' ? txPage.value    : null
  const walletP = profile.status === 'fulfilled' ? profile.value : null

  return (
    <div className="space-y-6">
      {/* Address header */}
      <div>
        <div className="text-xs text-gray-500 font-mono mb-1">ADDRESS</div>
        <div className="font-mono text-sm text-gray-200 break-all">{addr}</div>
      </div>

      {/* Wallet profile card */}
      {walletP && (
        <div className="bg-surface-raised border border-surface-border rounded-lg p-4">
          <div className="flex items-center justify-between gap-4 flex-wrap">
            <div>
              <div className="text-xs text-gray-500 mb-1">WALLET TYPE</div>
              <span className="font-semibold text-brand capitalize">
                {walletP.wallet_type.replace('_', ' ')}
              </span>
              <span className="text-gray-500 text-xs ml-2">
                ({(walletP.confidence * 100).toFixed(0)}% confidence)
              </span>
              {walletP.updated_at && (
                <span className="text-gray-600 text-xs ml-2 hidden md:inline">
                  updated {new Date(walletP.updated_at).toLocaleString()}
                </span>
              )}
            </div>
            <WalletScoreBadge scores={walletP.scores} />
          </div>
        </div>
      )}

      {/* Filter bar — uses Next.js <Link> for client-side navigation (no full reload) */}
      <div className="flex items-center gap-3 flex-wrap">
        <span className="text-xs text-gray-500">Filter:</span>
        {(['all', 'swap', 'transfer', 'balanceChanged'] as const).map(t => (
          <Link
            key={t}
            href={`/address/${addr}?type=${t}`}
            className={`px-3 py-1 rounded text-xs font-mono transition-colors ${
              txType === t
                ? 'bg-brand text-black'
                : 'bg-surface-raised border border-surface-border text-gray-400 hover:border-brand'
            }`}
          >
            {t}
          </Link>
        ))}
      </div>

      {/* Transaction history — spam toggle and infinite scroll are client-side */}
      {page ? (
        <TxHistoryList page={page} addr={addr} txType={txType} />
      ) : (
        <div className="text-gray-500 text-sm font-mono">No transactions found.</div>
      )}
    </div>
  )
}

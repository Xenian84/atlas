'use client'
import { useState } from 'react'
import Link from 'next/link'
import { TxHistoryPage, TxSummary, abbrev, lamportsToXnt, fetchAddressTxs } from '@/lib/api'
import { TagBadge } from './TagBadge'

export function TxHistoryList({
  page: initialPage,
  addr,
  txType,
}: {
  page:   TxHistoryPage
  addr:   string
  txType: string
}) {
  const [pages,     setPages]    = useState<TxHistoryPage[]>([initialPage])
  const [loading,   setLoading]  = useState(false)
  const [showSpam,  setShowSpam] = useState(false)

  const allTxs   = pages.flatMap(p => p.transactions)
  const lastPage = pages[pages.length - 1]
  const hasMore  = !!lastPage.next_cursor

  const loadMore = async () => {
    if (!hasMore || loading) return
    setLoading(true)
    try {
      // Pass txType so pagination respects the active filter
      const next = await fetchAddressTxs(addr, lastPage.next_cursor, 50, txType)
      setPages(ps => [...ps, next])
    } finally {
      setLoading(false)
    }
  }

  const visibleTxs = showSpam
    ? allTxs
    : allTxs.filter(tx => !tx.tags.includes('spam'))

  return (
    <div className="space-y-3">
      {/* Spam toggle */}
      <label className="flex items-center gap-2 text-xs text-gray-400 cursor-pointer w-fit">
        <input
          type="checkbox"
          className="accent-brand"
          checked={showSpam}
          onChange={e => setShowSpam(e.target.checked)}
        />
        Show spam transactions
      </label>

      <div className="space-y-2">
        {visibleTxs.map(tx => (
          <TxRow key={tx.signature} tx={tx} />
        ))}
        {visibleTxs.length === 0 && (
          <div className="text-gray-500 text-sm font-mono py-4">No transactions.</div>
        )}
      </div>

      {hasMore && (
        <button
          onClick={loadMore}
          disabled={loading}
          className="w-full py-3 border border-surface-border rounded-lg text-sm text-gray-500
                     hover:border-brand hover:text-brand transition-colors disabled:opacity-50"
        >
          {loading ? 'Loading…' : 'Load more'}
        </button>
      )}
    </div>
  )
}

function TxRow({ tx }: { tx: TxSummary }) {
  const isSuccess = tx.status === 'success'
  const time      = tx.block_time
    ? new Date(tx.block_time * 1000).toLocaleString()
    : `slot ${tx.slot}`

  return (
    <Link href={`/tx/${tx.signature}`}>
      <div className="flex items-center gap-4 bg-surface-raised border border-surface-border
                      rounded-lg px-4 py-3 hover:border-brand/50 transition-colors cursor-pointer">
        <div className={`w-2 h-2 rounded-full shrink-0 ${
          isSuccess ? 'bg-green-400' : 'bg-red-400'
        }`} />

        <div className="font-mono text-xs text-gray-300 w-36 truncate shrink-0">
          {abbrev(tx.signature, 8)}
        </div>

        <div className="flex-1 text-xs text-gray-500 truncate">
          {tx.action_types.length > 0
            ? tx.action_types.join(', ')
            : 'no actions'}
        </div>

        <div className="flex gap-1 shrink-0">
          {tx.tags.slice(0, 3).map(t => <TagBadge key={t} tag={t} />)}
        </div>

        <div className="font-mono text-xs text-gray-500 w-24 text-right shrink-0">
          {lamportsToXnt(tx.fee_lamports)} XNT
        </div>

        <div className="text-xs text-gray-600 w-36 text-right shrink-0 hidden md:block">
          {time}
        </div>
      </div>
    </Link>
  )
}

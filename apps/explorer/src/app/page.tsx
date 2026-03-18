'use client'
import { useState } from 'react'
import { useRouter } from 'next/navigation'

export default function HomePage() {
  const [query, setQuery] = useState('')
  const router = useRouter()

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault()
    const q = query.trim()
    if (!q) return
    // Signatures are 87-88 chars base58; addresses are 32-44 chars
    if (q.length > 60) {
      router.push(`/tx/${q}`)
    } else {
      router.push(`/address/${q}`)
    }
  }

  return (
    <div className="flex flex-col items-center justify-center min-h-[60vh] gap-8">
      {/* Hero */}
      <div className="text-center space-y-3">
        <div className="text-5xl font-mono font-semibold text-brand">◈ Atlas</div>
        <p className="text-gray-400 text-lg">X1 Blockchain Explorer</p>
      </div>

      {/* Search */}
      <form onSubmit={handleSearch} className="w-full max-w-2xl">
        <div className="flex gap-2">
          <input
            className="flex-1 bg-surface-raised border border-surface-border rounded-lg px-4 py-3
                       font-mono text-sm text-gray-100 placeholder-gray-600
                       focus:outline-none focus:border-brand transition-colors"
            placeholder="Search by signature or wallet address…"
            value={query}
            onChange={e => setQuery(e.target.value)}
            autoFocus
          />
          <button
            type="submit"
            className="bg-brand text-black font-semibold px-6 py-3 rounded-lg
                       hover:bg-brand-dark transition-colors"
          >
            Search
          </button>
        </div>
      </form>

      {/* Quick links */}
      <div className="text-xs text-gray-600 font-mono space-x-4">
        <span>Try: address • transaction signature</span>
      </div>
    </div>
  )
}

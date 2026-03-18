'use client'
import { useState } from 'react'
import { explainTx, ExplainResult } from '@/lib/api'

export function ExplainPanel({ sig }: { sig: string }) {
  const [open,    setOpen]    = useState(false)
  const [result,  setResult]  = useState<ExplainResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [error,   setError]   = useState<string | null>(null)

  const handleExplain = async () => {
    if (result) { setOpen(o => !o); return }
    setLoading(true)
    setError(null)
    try {
      const data = await explainTx(sig)
      setResult(data)
      setOpen(true)
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to explain')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div>
      <button
        onClick={handleExplain}
        disabled={loading}
        className="px-4 py-2 bg-brand/10 border border-brand/30 text-brand text-sm rounded-lg
                   hover:bg-brand/20 transition-colors disabled:opacity-50 font-mono"
      >
        {loading ? 'Explaining…' : '✦ Explain'}
      </button>
      {error && <p className="text-red-400 text-xs mt-2">{error}</p>}
      {open && result && (
        <div className="mt-3 bg-surface-raised border border-surface-border rounded-lg p-4 space-y-3">
          {/* Confidence badge */}
          <div className="flex items-center gap-2">
            <span className="text-xs text-gray-500">Confidence:</span>
            <span className={`text-xs font-mono px-2 py-0.5 rounded-full ${
              result.explain.confidence >= 0.8
                ? 'bg-green-900 text-green-300'
                : result.explain.confidence >= 0.5
                  ? 'bg-yellow-900 text-yellow-300'
                  : 'bg-gray-800 text-gray-400'
            }`}>
              {(result.explain.confidence * 100).toFixed(0)}%
            </span>
          </div>
          <p className="text-sm text-gray-200">{result.explain.summary}</p>
          <ul className="space-y-1">
            {result.explain.bullets.map((b, i) => (
              <li key={i} className="text-xs text-gray-400 flex gap-2">
                <span className="text-brand shrink-0">•</span>
                {b}
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}

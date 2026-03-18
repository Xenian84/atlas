'use client'
import { useState } from 'react'

export function CopyButton({
  label,
  getValue,
}: {
  label:    string
  getValue: () => Promise<string>
}) {
  const [state, setState] = useState<'idle' | 'copying' | 'copied' | 'error'>('idle')

  const handleCopy = async () => {
    setState('copying')
    try {
      const text = await getValue()
      await navigator.clipboard.writeText(text)
      setState('copied')
      setTimeout(() => setState('idle'), 2000)
    } catch {
      setState('error')
      setTimeout(() => setState('idle'), 2000)
    }
  }

  return (
    <button
      onClick={handleCopy}
      disabled={state === 'copying'}
      className="px-4 py-2 bg-surface-raised border border-surface-border text-gray-400 text-sm
                 rounded-lg hover:border-brand hover:text-brand transition-colors font-mono
                 disabled:opacity-50"
    >
      {state === 'idle'    ? label       :
       state === 'copying' ? 'Copying…'  :
       state === 'copied'  ? '✓ Copied'  : '✗ Error'}
    </button>
  )
}

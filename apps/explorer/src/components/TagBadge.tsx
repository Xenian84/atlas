const TAG_STYLES: Record<string, string> = {
  spam:          'bg-red-950 text-red-300 border-red-800',
  failed:        'bg-red-950 text-red-300 border-red-800',
  swap:          'bg-green-950 text-green-300 border-green-800',
  transfer:      'bg-blue-950 text-blue-300 border-blue-800',
  mint:          'bg-purple-950 text-purple-300 border-purple-800',
  burn:          'bg-orange-950 text-orange-300 border-orange-800',
  stake:         'bg-yellow-950 text-yellow-300 border-yellow-800',
  deploy:        'bg-cyan-950 text-cyan-300 border-cyan-800',
  priority_fee:  'bg-gray-800 text-gray-300 border-gray-600',
  high_compute:  'bg-gray-800 text-gray-300 border-gray-600',
}

export function TagBadge({ tag }: { tag: string }) {
  const style = TAG_STYLES[tag] ?? 'bg-gray-900 text-gray-400 border-gray-700'
  return (
    <span className={`px-2 py-0.5 rounded border text-xs font-mono ${style}`}>
      {tag}
    </span>
  )
}

export function SpamBadge() {
  return <TagBadge tag="spam" />
}

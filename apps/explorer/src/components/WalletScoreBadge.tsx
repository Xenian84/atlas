interface Scores {
  automation: number
  sniper:     number
  whale:      number
  risk:       number
}

export function WalletScoreBadge({ scores }: { scores: Scores }) {
  return (
    <div className="flex gap-4">
      <Score label="Auto" value={scores.automation} />
      <Score label="Sniper" value={scores.sniper} />
      <Score label="Whale" value={scores.whale} />
      <Score label="Risk" value={scores.risk} danger />
    </div>
  )
}

function Score({ label, value, danger }: { label: string; value: number; danger?: boolean }) {
  const color = danger
    ? value > 60 ? 'text-red-400' : value > 30 ? 'text-orange-400' : 'text-gray-400'
    : value > 60 ? 'text-brand' : value > 30 ? 'text-yellow-400' : 'text-gray-400'

  return (
    <div className="text-center">
      <div className={`text-lg font-mono font-semibold ${color}`}>{value}</div>
      <div className="text-xs text-gray-500">{label}</div>
    </div>
  )
}

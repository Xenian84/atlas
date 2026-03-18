import { TokenDelta, abbrev } from '@/lib/api'

export function TokenDeltaTable({ deltas }: { deltas: TokenDelta[] }) {
  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs font-mono">
        <thead>
          <tr className="text-gray-500 border-b border-surface-border">
            <th className="text-left py-2 pr-4">Mint</th>
            <th className="text-left py-2 pr-4">Owner</th>
            <th className="text-right py-2 pr-4">Delta</th>
            <th className="text-center py-2">Dir</th>
          </tr>
        </thead>
        <tbody>
          {deltas.map((d, i) => (
            <tr key={i} className="border-b border-surface-border hover:bg-surface-raised">
              <td className="py-2 pr-4 text-gray-300">
                {d.symbol ?? abbrev(d.mint, 8)}
              </td>
              <td className="py-2 pr-4 text-gray-500">{abbrev(d.owner, 8)}</td>
              <td className={`py-2 pr-4 text-right font-semibold ${
                d.direction === 'in'  ? 'text-green-400' :
                d.direction === 'out' ? 'text-red-400'   : 'text-gray-400'
              }`}>
                {d.direction === 'in' ? '+' : ''}{d.delta}
              </td>
              <td className="py-2 text-center">
                {d.direction === 'in'  ? '↓' :
                 d.direction === 'out' ? '↑' : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

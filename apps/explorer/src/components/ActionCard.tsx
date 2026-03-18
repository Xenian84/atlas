import { Action, abbrev } from '@/lib/api'

const ACTION_ICON: Record<string, string> = {
  TRANSFER:  '→',
  SWAP:      '⇄',
  MINT:      '✦',
  BURN:      '🔥',
  STAKE:     '⬆',
  UNSTAKE:   '⬇',
  DEPLOY:    '⚙',
  NFT_SALE:  '🖼',
  UNKNOWN:   '?',
}

const PROTO_COLOR: Record<string, string> = {
  SYSTEM:      'text-blue-400',
  TOKEN:       'text-purple-400',
  X1DEX:       'text-green-400',
  JUPITERLIKE: 'text-emerald-400',
  STAKE:       'text-yellow-400',
  NFTPROG:     'text-pink-400',
  UNKNOWN:     'text-gray-400',
}

export function ActionCard({ action }: { action: Action }) {
  const icon  = ACTION_ICON[action.t] ?? '?'
  const color = PROTO_COLOR[action.p] ?? 'text-gray-400'

  return (
    <div className="flex items-start gap-3 bg-surface border border-surface-border rounded-lg p-3">
      <span className="text-xl w-6 text-center shrink-0">{icon}</span>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-xs font-semibold font-mono">{action.t}</span>
          <span className={`text-xs font-mono ${color}`}>{action.p}</span>
        </div>
        <div className="font-mono text-xs text-gray-400 truncate">
          {abbrev(action.s, 12)}
          {action.x && (
            <>
              <span className="text-gray-600 mx-1">→</span>
              {abbrev(action.x, 12)}
            </>
          )}
        </div>
        {action.amt != null && (
          <div className="text-xs text-gray-500 mt-1">
            {JSON.stringify(action.amt)}
          </div>
        )}
      </div>
    </div>
  )
}

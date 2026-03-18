import { fetchTx, explainTx, abbrev, lamportsToXnt, TxFacts } from '@/lib/api'
import { ActionCard } from '@/components/ActionCard'
import { TokenDeltaTable } from '@/components/TokenDeltaTable'
import { TagBadge } from '@/components/TagBadge'
import { ExplainPanel } from '@/components/ExplainPanel'
import { CopyButton } from '@/components/CopyButton'

export default async function TxPage({ params }: { params: { sig: string } }) {
  let facts: TxFacts | null = null
  let error: string | null = null

  try {
    facts = await fetchTx(params.sig)
  } catch (e: unknown) {
    error = e instanceof Error ? e.message : 'Failed to load transaction'
  }

  if (error || !facts) {
    return (
      <div className="text-red-400 font-mono text-sm p-6 bg-surface-raised rounded-lg border border-red-900">
        {error ?? 'Transaction not found'}
      </div>
    )
  }

  const sig       = facts.sig  // capture before JSX to avoid non-null assertions in closures
  const isSuccess = facts.status === 'success'

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-xs text-gray-500 font-mono mb-1">TRANSACTION</div>
          <div className="font-mono text-sm text-gray-200 break-all">{sig}</div>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className={`px-3 py-1 rounded-full text-xs font-semibold ${
            isSuccess ? 'bg-green-900 text-green-300' : 'bg-red-900 text-red-300'
          }`}>
            {isSuccess ? '✓ Success' : '✗ Failed'}
          </span>
        </div>
      </div>

      {/* Meta row */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <MetaCard label="Slot"       value={facts.slot.toLocaleString()} />
        <MetaCard label="Block Time" value={facts.block_time
          ? new Date(facts.block_time * 1000).toLocaleString()
          : '—'
        } />
        <MetaCard label="Fee"        value={`${lamportsToXnt(facts.fee_lamports)} XNT`} />
        <MetaCard label="Compute"    value={
          facts.compute_units.consumed
            ? `${facts.compute_units.consumed.toLocaleString()} / ${(facts.compute_units.limit ?? 0).toLocaleString()}`
            : '—'
        } />
      </div>

      {/* Tags */}
      {facts.tags.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {facts.tags.map(t => <TagBadge key={t} tag={t} />)}
        </div>
      )}

      {/* Actions */}
      {facts.actions.length > 0 && (
        <Section title="Actions">
          <div className="space-y-2">
            {facts.actions.map((a, i) => <ActionCard key={i} action={a} />)}
          </div>
        </Section>
      )}

      {/* Token Deltas */}
      {facts.token_deltas.length > 0 && (
        <Section title="Token Changes">
          <TokenDeltaTable deltas={facts.token_deltas} />
        </Section>
      )}

      {/* Native XNT Deltas */}
      {facts.xnt_deltas && facts.xnt_deltas.length > 0 && (
        <Section title="XNT Balance Changes">
          <div className="overflow-x-auto">
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="text-gray-500 text-left border-b border-surface-border">
                  <th className="pb-2 pr-4">Account</th>
                  <th className="pb-2 pr-4 text-right">Before</th>
                  <th className="pb-2 pr-4 text-right">After</th>
                  <th className="pb-2 text-right">Delta</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-surface-border">
                {facts.xnt_deltas.map((d, i) => (
                  <tr key={i} className="py-1">
                    <td className="py-2 pr-4 text-gray-300">{abbrev(d.owner, 10)}</td>
                    <td className="py-2 pr-4 text-right text-gray-400">
                      {lamportsToXnt(d.pre_lamports)}
                    </td>
                    <td className="py-2 pr-4 text-right text-gray-400">
                      {lamportsToXnt(d.post_lamports)}
                    </td>
                    <td className={`py-2 text-right font-semibold ${
                      d.delta_lamports > 0 ? 'text-green-400' : 'text-red-400'
                    }`}>
                      {d.delta_lamports > 0 ? '+' : ''}
                      {lamportsToXnt(d.delta_lamports)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </Section>
      )}

      {/* Programs */}
      {facts.programs.length > 0 && (
        <Section title="Programs Invoked">
          <div className="space-y-1">
            {facts.programs.map(p => (
              <div key={p} className="font-mono text-xs text-gray-400">{p}</div>
            ))}
          </div>
        </Section>
      )}

      {/* Explain + Copy actions */}
      <div className="flex gap-3">
        <ExplainPanel sig={sig} />
        {/* CopyButton calls the proxy endpoint via explainTx — API key injected server-side */}
        <CopyButton
          label="Copy Facts (TOON)"
          getValue={async () => {
            const { explainTx: fetchExplain } = await import('@/lib/api')
            const data = await fetchExplain(sig)
            return data.factsToon ?? ''
          }}
        />
      </div>
    </div>
  )
}

function MetaCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-surface-raised border border-surface-border rounded-lg p-3">
      <div className="text-xs text-gray-500 mb-1">{label}</div>
      <div className="font-mono text-sm text-gray-100">{value}</div>
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-surface-raised border border-surface-border rounded-lg p-4 space-y-3">
      <div className="text-xs text-gray-500 font-semibold uppercase tracking-wider">{title}</div>
      {children}
    </div>
  )
}

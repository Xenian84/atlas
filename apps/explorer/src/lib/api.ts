/**
 * API client for Atlas backend.
 *
 * Server components use ATLAS_API_URL + ATLAS_API_KEY directly.
 * Client components route through /api/atlas/* proxy (server-side key injection).
 */

const SERVER_API_BASE = process.env.ATLAS_API_URL ?? 'http://localhost:8080'
const CLIENT_PROXY    = '/api/atlas'

function apiBase(forceServer = false): string {
  // On the server (during SSR / RSC) we can access the API directly.
  // On the client we route through the Next.js proxy.
  if (forceServer || typeof window === 'undefined') return SERVER_API_BASE
  return CLIENT_PROXY
}

export interface TxFacts {
  sig:          string
  slot:         number
  pos:          number
  block_time:   number | null
  status:       'success' | 'failed'
  fee_lamports: number
  compute_units: { consumed?: number; limit?: number; price_micro_lamports?: number }
  programs:     string[]
  tags:         string[]
  actions:      Action[]
  token_deltas: TokenDelta[]
  xnt_deltas:   NativeDelta[]
  err?:         unknown
}

export interface Action {
  t:    string
  p:    string
  s:    string
  x?:   string
  amt?: unknown
  meta?: unknown
}

export interface TokenDelta {
  mint:      string
  owner:     string
  delta:     string
  decimals:  number
  symbol?:   string
  direction: 'in' | 'out' | 'none'
}

export interface NativeDelta {
  owner:          string
  pre_lamports:   number
  post_lamports:  number
  delta_lamports: number
}

export interface TxSummary {
  signature:    string
  slot:         number
  pos:          number
  block_time:   number | null
  status:       'success' | 'failed'
  fee_lamports: number
  tags:         string[]
  action_types: string[]
  actions:      Action[]
  token_deltas: TokenDelta[]
}

export interface TxHistoryPage {
  address:      string
  limit:        number
  next_cursor?: string
  transactions: TxSummary[]
}

export interface WalletProfile {
  address:          string
  window:           string
  wallet_type:      string
  confidence:       number
  updated_at?:      string
  scores:           { automation: number; sniper: number; whale: number; risk: number }
  features:         Record<string, unknown>
  top_programs?:    { program_id: string; count: number }[]
  top_tokens?:      { mint: string; count: number }[]
}

export interface ExplainResult {
  facts:     TxFacts
  explain:   { summary: string; bullets: string[]; confidence: number }
  factsToon: string
}

/**
 * Fetch wrapper for server components — uses direct API URL with server-side key.
 */
export async function serverFetch<T>(path: string, opts?: RequestInit): Promise<T> {
  const apiKey = process.env.ATLAS_API_KEY ?? ''
  const res = await fetch(`${SERVER_API_BASE}${path}`, {
    ...opts,
    headers: {
      'Content-Type': 'application/json',
      'X-API-Key':    apiKey,
      ...(opts?.headers ?? {}),
    },
    next: { revalidate: 10 },
  })
  if (!res.ok) throw new Error(`API ${res.status}: ${await res.text()}`)
  return res.json()
}

/**
 * Fetch wrapper for client components — routes through /api/atlas proxy.
 */
export async function clientFetch<T>(path: string, opts?: RequestInit): Promise<T> {
  const res = await fetch(`${CLIENT_PROXY}${path}`, {
    ...opts,
    headers: {
      'Content-Type': 'application/json',
      ...(opts?.headers ?? {}),
    },
  })
  if (!res.ok) throw new Error(`API ${res.status}: ${await res.text()}`)
  return res.json()
}

// Convenience wrappers — auto-select server or client fetch context
function apiFetch<T>(path: string, opts?: RequestInit): Promise<T> {
  if (typeof window === 'undefined') return serverFetch<T>(path, opts)
  return clientFetch<T>(path, opts)
}

export const fetchTx = (sig: string) =>
  serverFetch<TxFacts>(`/v1/tx/${sig}`)

export const fetchAddressTxs = (
  addr:    string,
  cursor?: string,
  limit  = 50,
  txType?: string,
) =>
  apiFetch<TxHistoryPage>(
    `/v1/address/${addr}/txs?limit=${limit}${cursor ? `&before=${cursor}` : ''}${txType ? `&type=${txType}` : ''}`
  )

export const fetchWalletProfile = (addr: string, window = '7d') =>
  serverFetch<WalletProfile>(`/v1/address/${addr}/profile?window=${window}`)

export const explainTx = (sig: string) =>
  clientFetch<ExplainResult>(`/v1/tx/${sig}/explain`, { method: 'POST' })

export const abbrev = (s: string, n = 8) =>
  s.length > n * 2 + 2 ? `${s.slice(0, n)}…${s.slice(-4)}` : s

export const lamportsToXnt = (l: number) => (l / 1e9).toFixed(6)
/** @deprecated use lamportsToXnt */
export const lamportsToSol = lamportsToXnt

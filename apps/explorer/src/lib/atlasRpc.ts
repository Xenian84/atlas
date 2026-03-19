/**
 * Atlas RPC client — all calls go through the Atlas API proxy.
 * Server components use ATLAS_API_URL directly.
 * Client components use /api/atlas/rpc (Next.js proxy with auth).
 */

const IS_SERVER = typeof window === 'undefined';
const BASE_URL  = IS_SERVER
  ? (process.env.ATLAS_API_URL ?? 'http://localhost:8888')
  : '';  // client-side: use relative /api/atlas proxy

const API_KEY = process.env.ATLAS_API_KEY ?? '';

let _rpcId = 1;

/** Send a JSON-RPC request through Atlas's /rpc endpoint */
export async function rpc<T = unknown>(
  method: string,
  params: unknown[] = [],
): Promise<T> {
  const url = IS_SERVER ? `${BASE_URL}/rpc` : '/api/atlas/rpc';

  const headers: HeadersInit = { 'Content-Type': 'application/json' };
  if (IS_SERVER && API_KEY) headers['X-API-Key'] = API_KEY;

  const res = await fetch(url, {
    method: 'POST',
    headers,
    body: JSON.stringify({ jsonrpc: '2.0', id: _rpcId++, method, params }),
    next: { revalidate: 0 },
  });

  if (!res.ok) throw new Error(`RPC HTTP ${res.status}`);
  const json = await res.json();
  if (json.error) throw new Error(json.error.message ?? JSON.stringify(json.error));
  return json.result as T;
}

/** GET a REST endpoint through the Atlas API */
export async function atlasGet<T = unknown>(path: string): Promise<T> {
  const url     = IS_SERVER ? `${BASE_URL}${path}` : `/api/atlas${path}`;
  const headers: HeadersInit = {};
  if (IS_SERVER && API_KEY) headers['X-API-Key'] = API_KEY;

  const res = await fetch(url, { headers, next: { revalidate: 0 } });
  if (!res.ok) throw new Error(`GET ${path} → HTTP ${res.status}`);
  return res.json() as Promise<T>;
}

/* ── Typed helpers ────────────────────────────────────────── */

export interface EpochInfo {
  epoch: number;
  slotIndex: number;
  slotsInEpoch: number;
  absoluteSlot: number;
  blockHeight: number;
  transactionCount: number;
}

export interface PerformanceSample {
  slot: number;
  numTransactions: number;
  numNonVoteTransactions?: number;
  numSlots: number;
  samplePeriodSecs: number;
}

export interface ClusterNode {
  pubkey: string;
  gossip: string | null;
  tpu: string | null;
  rpc: string | null;
  version: string | null;
  featureSet: number | null;
  shredVersion: number | null;
}

export interface VoteAccount {
  votePubkey: string;
  nodePubkey: string;
  activatedStake: number;
  epochVoteAccount: boolean;
  commission: number;
  lastVote: number;
  epochCredits: [number, number, number][];
  rootSlot: number;
}

export interface VoteAccountsResult {
  current: VoteAccount[];
  delinquent: VoteAccount[];
}

export interface NetworkPulse {
  slot:                number;
  block_time?:         number;
  tps_1m?:             number;
  indexed_txs_24h?:    number;
  active_wallets_24h?: number;
  xnt_price_usd?:      number;
  indexer?: {
    indexed_slot:      number;
    lag_slots:         number;
    indexed_accounts:  number;
    indexed_tokens:    number;
    pending_webhooks:  number;
  };
}

export const getEpochInfo    = () => rpc<EpochInfo>('getEpochInfo');
export const getVoteAccounts = () => rpc<VoteAccountsResult>('getVoteAccounts');
export const getClusterNodes = () => rpc<ClusterNode[]>('getClusterNodes');
export const getVersion      = () => rpc<{ 'solana-core': string; 'feature-set': number }>('getVersion');
export const getSlot         = () => rpc<number>('getSlot');

export const getPerformanceSamples = (limit = 60) =>
  rpc<PerformanceSample[]>('getRecentPerformanceSamples', [limit]);

export const getNetworkPulse = () => atlasGet<NetworkPulse>('/v1/network/pulse');

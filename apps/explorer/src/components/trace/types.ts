export interface WalletNodeData {
  address: string;
  sol_balance: number;
  token_count: number;
  tx_count: number;
  move_count: number;
  first_seen: number | null;
  last_seen: number | null;
  labels: string[];
  isRoot?: boolean;
}

export interface TraceEdge {
  id: string;
  source: string;
  target: string;
  tx_count: number;
  total_lamports: number;
  mints: string[];
  first_ts: number | null;
  last_ts: number | null;
  direction: 'out' | 'in' | 'both';
}

export interface CounterpartyRow {
  address: string;
  label: string | null;
  direction: string;
  tx_count: number;
  total_lamports: number;
  mint: string | null;
  first_ts: number | null;
  last_ts: number | null;
}

export interface TraceData {
  root: string;
  nodes: WalletNodeData[];
  edges: TraceEdge[];
  transfers: CounterpartyRow[];
  total_outflow_lamports: number;
  total_inflow_lamports: number;
  total_transfers: number;
  cps: number;
}

export interface TraceFilters {
  from_ts: string;
  to_ts: string;
  mint: string;
  hide_dust: boolean;
  min_amount: string;
  max_amount: string;
  direction: 'all' | 'out' | 'in';
}

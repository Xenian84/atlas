'use client';

import { useState, useCallback, useEffect } from 'react';
import { useParams, useRouter } from 'next/navigation';
import dynamic from 'next/dynamic';
import type { TraceData, TraceFilters } from '@/components/trace/types';
import TraceSidebar from '@/components/trace/TraceSidebar';

// React Flow must be client-only (no SSR)
const TraceGraph = dynamic(() => import('@/components/trace/TraceGraph'), {
  ssr: false,
  loading: () => (
    <div style={{
      flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center',
      fontFamily: 'var(--font-mono)', color: 'hsl(var(--foreground-tertiary))', fontSize: 12,
    }}>
      Loading graph…
    </div>
  ),
});

const DEFAULT_FILTERS: TraceFilters = {
  from_ts: '',
  to_ts: '',
  mint: '',
  hide_dust: false,
  min_amount: '',
  max_amount: '',
  direction: 'all',
};

function shorten(addr: string, chars = 6) {
  return `${addr.slice(0, chars)}...${addr.slice(-chars)}`;
}

export default function TracePage() {
  const { address } = useParams<{ address: string }>();
  const router = useRouter();

  const [data, setData]         = useState<TraceData | null>(null);
  const [loading, setLoading]   = useState(true);
  const [error, setError]       = useState<string | null>(null);
  const [filters, setFilters]   = useState<TraceFilters>(DEFAULT_FILTERS);
  const [selected, setSelected] = useState<string | null>(null);
  const [nodeCount, setNodeCount] = useState(0);
  const [edgeCount, setEdgeCount] = useState(0);

  const loadTrace = useCallback(async (addr: string, f: TraceFilters) => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      if (f.from_ts)    params.set('from_ts',    String(new Date(f.from_ts).getTime() / 1000 | 0));
      if (f.to_ts)      params.set('to_ts',      String(new Date(f.to_ts).getTime() / 1000 | 0));
      if (f.hide_dust)  params.set('hide_dust',  'true');
      if (f.min_amount) params.set('min_amount', String(parseFloat(f.min_amount) * 1e9 | 0));
      if (f.max_amount) params.set('max_amount', String(parseFloat(f.max_amount) * 1e9 | 0));

      const url = `/api/atlas/v1/trace/${addr}${params.size ? `?${params}` : ''}`;
      const resp = await fetch(url, {
        headers: { 'x-api-key': process.env.NEXT_PUBLIC_ATLAS_API_KEY ?? '' },
      });
      if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
      const json: TraceData = await resp.json();

      // Apply direction filter client-side
      if (f.direction !== 'all') {
        json.transfers = json.transfers.filter(t => t.direction === f.direction);
        json.edges     = json.edges.filter(e => e.direction === f.direction || e.direction === 'both');
      }

      setData(json);
      setNodeCount(json.nodes.length);
      setEdgeCount(json.edges.length);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : 'Failed to load trace');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (address) loadTrace(address, filters);
  }, [address, filters, loadTrace]);

  const handleFilterChange = useCallback((partial: Partial<TraceFilters>) => {
    setFilters(prev => ({ ...prev, ...partial }));
  }, []);

  const handleNodeClick = useCallback((addr: string) => {
    setSelected(addr);
    // Expand: navigate to new trace for double-clicked peer
  }, []);

  const handleNodeDoubleClick = useCallback((addr: string) => {
    if (addr !== address) {
      router.push(`/trace/${addr}`);
    }
  }, [address, router]);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100vh',
        background: 'hsl(var(--background))',
        color: 'hsl(var(--foreground))',
        fontFamily: 'var(--font-mono)',
        overflow: 'hidden',
      }}
    >
      {/* ── Header ────────────────────────────────────── */}
      <header
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 0,
          padding: '0 20px',
          height: 44,
          borderBottom: '1px solid hsl(var(--border))',
          flexShrink: 0,
          background: 'hsla(var(--background),.98)',
        }}
      >
        {/* Logo */}
        <a href="/"
          style={{
            color: 'hsl(var(--primary))',
            fontWeight: 900,
            fontSize: 13,
            letterSpacing: '0.18em',
            marginRight: 20,
            textDecoration: 'none',
          }}
        >
          ◈ ATLAS
        </a>

        {/* Breadcrumb */}
        <BreadcrumbItem
          label="COUNTERPARTIES"
          onClick={() => router.push(`/address/${address}`)}
          dimmed
        />
        <Chevron />
        <BreadcrumbItem label="TRACE" active />

        {/* Spacer */}
        <div style={{ flex: 1 }} />

        {/* Back button */}
        <button
          onClick={() => router.back()}
          style={headerBtnStyle}
        >
          ← Back
        </button>

        {/* Address chip */}
        <span
          style={{
            background: 'hsla(var(--primary),.08)',
            border: '1px solid hsla(var(--primary),.25)',
            padding: '3px 10px',
            color: 'hsl(var(--primary))',
            fontFamily: 'var(--font-mono)',
            fontSize: 10,
            letterSpacing: '0.04em',
            marginLeft: 12,
          }}
        >
          {shorten(address)}
        </span>

        {/* Node / Edge counts */}
        {!loading && (
          <>
            <StatChip label="Nodes" value={nodeCount} />
            <StatChip label="Edges" value={edgeCount} />
          </>
        )}
      </header>

      {/* ── Body ──────────────────────────────────────── */}
      <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>

        {/* Graph canvas */}
        <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
          {loading && (
            <div style={{
              position: 'absolute', inset: 0, zIndex: 10,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              background: 'hsla(var(--background),.85)', flexDirection: 'column', gap: 12,
            }}>
              <Spinner />
              <span style={{ fontFamily: 'var(--font-mono)', color: 'hsl(var(--foreground-tertiary))', fontSize: 11, letterSpacing: '.08em' }}>
                TRACING COUNTERPARTIES…
              </span>
            </div>
          )}

          {error && (
            <div style={{
              position: 'absolute', inset: 0,
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              flexDirection: 'column', gap: 12,
            }}>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'hsl(var(--accent-red))', fontSize: 11, letterSpacing: '.06em' }}>⚠ {error}</span>
              <button onClick={() => loadTrace(address, filters)} style={headerBtnStyle}>
                Retry
              </button>
            </div>
          )}

          {data && !loading && !error && (
            <TraceGraph
              data={data}
              onNodeClick={handleNodeClick}
            />
          )}

          {/* Flow summary overlay (top-right inside canvas) */}
          {data && !loading && (
            <div
              style={{
                position: 'absolute',
                top: 16,
                right: 16,
                background: 'hsl(var(--card))',
                border: '1px solid hsl(var(--border-strong))',
                padding: '10px 14px',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                pointerEvents: 'none',
              }}
            >
              <div style={{ color: 'hsl(var(--primary))', fontWeight: 700, marginBottom: 6, fontSize: 11, letterSpacing: '.08em' }}>
                FLOWS OF {shorten(address)}
              </div>
              <div style={{ color: 'hsl(var(--foreground-secondary))', fontSize: 10 }}>
                {data.total_transfers} transfers · {data.cps} cps
              </div>
              {selected && (
                <div style={{ marginTop: 8, paddingTop: 8, borderTop: '1px solid hsl(var(--border))' }}>
                  <span style={{ color: 'hsl(var(--foreground-muted))' }}>Selected: </span>
                  <span style={{ color: 'hsl(var(--foreground))' }}>{shorten(selected)}</span>
                </div>
              )}
            </div>
          )}

          {/* Hint */}
          {data && !loading && (
            <div style={{
              position: 'absolute', bottom: 52, left: '50%', transform: 'translateX(-50%)',
              fontFamily: 'var(--font-mono)', fontSize: 9,
              color: 'hsl(var(--foreground-muted))', letterSpacing: '.08em',
              pointerEvents: 'none', whiteSpace: 'nowrap',
            }}>
              Click to select · Double-click to expand · Scroll to zoom
            </div>
          )}
        </div>

        {/* Right sidebar */}
        {data && (
          <TraceSidebar
            data={data}
            filters={filters}
            onFilterChange={handleFilterChange}
            onAddressClick={setSelected}
            selectedAddress={selected}
          />
        )}
      </div>
    </div>
  );
}

/* ── Small helpers ──────────────────────────────────────────── */

function Spinner() {
  return (
    <div style={{
      width: 28, height: 28,
      border: '2px solid hsla(var(--primary),.15)',
      borderTop: '2px solid hsl(var(--primary))',
      borderRadius: '50%',
      animation: 'spin 0.8s linear infinite',
    }} />
  );
}

function BreadcrumbItem({ label, onClick, dimmed, active }: { label: string; onClick?: () => void; dimmed?: boolean; active?: boolean }) {
  return (
    <span onClick={onClick} style={{
      fontFamily: 'var(--font-mono)', fontSize: 10, letterSpacing: '.12em',
      color: active ? 'hsl(var(--foreground))' : dimmed ? 'hsl(var(--foreground-muted))' : 'hsl(var(--foreground-tertiary))',
      cursor: onClick ? 'pointer' : 'default',
      fontWeight: active ? 600 : 400,
      padding: '0 4px', transition: 'color .15s',
    }}>
      {label}
    </span>
  );
}

function Chevron() {
  return <span style={{ color: 'hsl(var(--border-strong))', fontSize: 10, padding: '0 2px' }}>//</span>;
}

function StatChip({ label, value }: { label: string; value: number }) {
  return (
    <span style={{
      marginLeft: 8,
      background: 'hsl(var(--background-secondary))',
      border: '1px solid hsl(var(--border))',
      padding: '2px 8px',
      fontFamily: 'var(--font-mono)', fontSize: 9, letterSpacing: '.08em',
      color: 'hsl(var(--foreground-tertiary))',
    }}>
      <span style={{ color: 'hsl(var(--foreground-secondary))' }}>{value}</span> {label}
    </span>
  );
}

const headerBtnStyle: React.CSSProperties = {
  background: 'transparent',
  border: '1px solid hsl(var(--border-strong))',
  color: 'hsl(var(--foreground-tertiary))',
  fontFamily: 'var(--font-mono)',
  fontSize: 9, letterSpacing: '.1em',
  padding: '4px 10px',
  cursor: 'pointer',
  transition: 'color .15s, border-color .15s',
};

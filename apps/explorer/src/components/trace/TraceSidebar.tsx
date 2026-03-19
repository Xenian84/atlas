'use client';

import { format } from 'date-fns';
import type { CounterpartyRow, TraceData, TraceFilters } from './types';

interface Props {
  data: TraceData;
  filters: TraceFilters;
  onFilterChange: (f: Partial<TraceFilters>) => void;
  onAddressClick: (addr: string) => void;
  selectedAddress: string | null;
}

function shorten(addr: string, chars = 5) {
  return `${addr.slice(0, chars)}...${addr.slice(-chars)}`;
}

function formatTs(ts: number | null) {
  if (!ts) return '';
  try { return format(new Date(ts * 1000), 'MMM d, \'\'yy'); }
  catch { return ''; }
}

function formatXnt(lamports: number) {
  const xnt = lamports / 1e9;
  if (xnt >= 1000) return `${(xnt / 1000).toFixed(1)}K XNT`;
  if (xnt >= 0.001) return `${xnt.toFixed(3)} XNT`;
  return `${lamports.toLocaleString()} L`;
}

export default function TraceSidebar({ data, filters, onFilterChange, onAddressClick, selectedAddress }: Props) {
  const outflow = data.transfers.filter(t => t.direction === 'out');
  const inflow  = data.transfers.filter(t => t.direction !== 'out');

  return (
    <aside
      style={{
        width: 300,
        flexShrink: 0,
        background: 'rgba(5,5,10,0.97)',
        borderLeft: '1px solid rgba(255,255,255,0.07)',
        display: 'flex',
        flexDirection: 'column',
        fontFamily: '"Courier New", monospace',
        fontSize: 11,
        color: '#cdd6f4',
        overflowY: 'auto',
      }}
    >
      {/* ── Summary ─────────────────────────────────────── */}
      <div style={{ padding: '14px 16px', borderBottom: '1px solid rgba(255,255,255,0.07)' }}>
        <div style={{ color: '#00e5ff', fontWeight: 700, fontSize: 12, marginBottom: 8, letterSpacing: '0.06em' }}>
          FLOW SUMMARY
        </div>
        <SummaryRow label="Transfers" value={String(data.total_transfers)} />
        <SummaryRow label="Counterparties" value={String(data.cps)} />
        <SummaryRow
          label="Total Outflow"
          value={formatXnt(data.total_outflow_lamports)}
          color="#cba6f7"
        />
        <SummaryRow
          label="Total Inflow"
          value={formatXnt(data.total_inflow_lamports)}
          color="#94e2d5"
        />
      </div>

      {/* ── Filters ─────────────────────────────────────── */}
      <div style={{ padding: '12px 16px', borderBottom: '1px solid rgba(255,255,255,0.07)' }}>
        <div style={{ color: '#6c7086', fontSize: 9, textTransform: 'uppercase', letterSpacing: '0.1em', marginBottom: 10 }}>
          FILTER
        </div>

        <FieldGroup label="FROM">
          <DateInput
            value={filters.from_ts}
            onChange={v => onFilterChange({ from_ts: v })}
          />
        </FieldGroup>

        <FieldGroup label="TO">
          <DateInput
            value={filters.to_ts}
            onChange={v => onFilterChange({ to_ts: v })}
          />
        </FieldGroup>

        <FieldGroup label="DIRECTION">
          <select
            value={filters.direction}
            onChange={e => onFilterChange({ direction: e.target.value as TraceFilters['direction'] })}
            style={selectStyle}
          >
            <option value="all">All</option>
            <option value="out">Outflow only</option>
            <option value="in">Inflow only</option>
          </select>
        </FieldGroup>

        <div style={{ display: 'flex', gap: 8 }}>
          <FieldGroup label="MIN XNT">
            <input
              type="number"
              placeholder="0"
              value={filters.min_amount}
              onChange={e => onFilterChange({ min_amount: e.target.value })}
              style={{ ...inputStyle, width: '100%' }}
            />
          </FieldGroup>
          <FieldGroup label="MAX XNT">
            <input
              type="number"
              placeholder="∞"
              value={filters.max_amount}
              onChange={e => onFilterChange({ max_amount: e.target.value })}
              style={{ ...inputStyle, width: '100%' }}
            />
          </FieldGroup>
        </div>

        <label style={{ display: 'flex', alignItems: 'center', gap: 8, cursor: 'pointer', marginTop: 8 }}>
          <input
            type="checkbox"
            checked={filters.hide_dust}
            onChange={e => onFilterChange({ hide_dust: e.target.checked })}
            style={{ accentColor: '#00e5ff' }}
          />
          <span style={{ color: '#a6adc8', fontSize: 10 }}>Hide $0 dust (&lt; 0.001 XNT)</span>
        </label>
      </div>

      {/* ── Outflow ─────────────────────────────────────── */}
      <SectionList
        title="OUTFLOW"
        color="#cba6f7"
        rows={outflow}
        selectedAddress={selectedAddress}
        onAddressClick={onAddressClick}
      />

      {/* ── Inflow ──────────────────────────────────────── */}
      <SectionList
        title="INFLOW"
        color="#94e2d5"
        rows={inflow}
        selectedAddress={selectedAddress}
        onAddressClick={onAddressClick}
      />
    </aside>
  );
}

function SectionList({
  title, color, rows, selectedAddress, onAddressClick,
}: {
  title: string;
  color: string;
  rows: CounterpartyRow[];
  selectedAddress: string | null;
  onAddressClick: (a: string) => void;
}) {
  if (rows.length === 0) return null;
  return (
    <div style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
      <div
        style={{
          padding: '10px 16px 6px',
          color,
          fontSize: 9,
          fontWeight: 700,
          letterSpacing: '0.12em',
          display: 'flex',
          justifyContent: 'space-between',
        }}
      >
        <span>{title}</span>
        <span style={{ color: '#6c7086' }}>{rows.length} cps</span>
      </div>
      {rows.map(row => (
        <TransferRow
          key={row.address}
          row={row}
          isSelected={selectedAddress === row.address}
          onClick={() => onAddressClick(row.address)}
          accentColor={color}
        />
      ))}
    </div>
  );
}

function TransferRow({
  row, isSelected, onClick, accentColor,
}: {
  row: CounterpartyRow;
  isSelected: boolean;
  onClick: () => void;
  accentColor: string;
}) {
  return (
    <div
      onClick={onClick}
      style={{
        padding: '8px 16px',
        cursor: 'pointer',
        borderLeft: isSelected ? `2px solid ${accentColor}` : '2px solid transparent',
        background: isSelected ? 'rgba(0,229,255,0.04)' : 'transparent',
        transition: 'background 0.15s',
        display: 'grid',
        gridTemplateColumns: '1fr auto',
        gap: '2px 8px',
        alignItems: 'center',
      }}
      onMouseEnter={e => { if (!isSelected) (e.currentTarget as HTMLDivElement).style.background = 'rgba(255,255,255,0.03)'; }}
      onMouseLeave={e => { if (!isSelected) (e.currentTarget as HTMLDivElement).style.background = 'transparent'; }}
    >
      <div>
        <div style={{ color: accentColor, fontWeight: 600, fontSize: 11 }}>
          {row.label ?? shorten(row.address)}
        </div>
        {row.label && (
          <div style={{ color: '#45475a', fontSize: 9 }}>{shorten(row.address)}</div>
        )}
      </div>
      <div style={{ textAlign: 'right' }}>
        <div style={{ color: '#cdd6f4', fontWeight: 600 }}>{row.tx_count}</div>
        <div style={{ color: '#45475a', fontSize: 9 }}>{formatTs(row.last_ts)}</div>
      </div>

      {row.total_lamports > 0 && (
        <>
          <div style={{ color: '#6c7086', fontSize: 9, gridColumn: '1/-1', marginTop: 2 }}>
            {formatXnt(row.total_lamports)}
          </div>
        </>
      )}
    </div>
  );
}

function SummaryRow({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 5 }}>
      <span style={{ color: '#6c7086' }}>{label}</span>
      <span style={{ color: color ?? '#cdd6f4', fontWeight: 600 }}>{value}</span>
    </div>
  );
}

function FieldGroup({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 8 }}>
      <div style={{ color: '#45475a', fontSize: 9, letterSpacing: '0.1em', marginBottom: 4 }}>
        {label}
      </div>
      {children}
    </div>
  );
}

function DateInput({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  return (
    <input
      type="date"
      value={value}
      onChange={e => onChange(e.target.value)}
      style={inputStyle}
    />
  );
}

const inputStyle: React.CSSProperties = {
  width: '100%',
  background: 'rgba(255,255,255,0.04)',
  border: '1px solid rgba(255,255,255,0.1)',
  borderRadius: 4,
  color: '#cdd6f4',
  fontFamily: '"Courier New", monospace',
  fontSize: 11,
  padding: '5px 8px',
  outline: 'none',
  boxSizing: 'border-box',
};

const selectStyle: React.CSSProperties = {
  ...inputStyle,
  cursor: 'pointer',
  appearance: 'none',
  backgroundImage: `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8'%3E%3Cpath d='M0 0l6 8 6-8z' fill='%236c7086'/%3E%3C/svg%3E")`,
  backgroundRepeat: 'no-repeat',
  backgroundPosition: 'right 8px center',
  paddingRight: 24,
};

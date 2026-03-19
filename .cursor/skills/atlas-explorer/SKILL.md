---
name: atlas-explorer
description: Build, fix, or extend the Atlas Next.js explorer. Use when working on apps/explorer — pages, components, API proxy routes, or the design system.
---

# Atlas Explorer Skill

## Stack
- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript
- **Styling**: CSS custom properties (HSL variables) + utility classes in `globals.css`. NO Tailwind CSS class names — everything uses inline styles or the custom classes below.
- **Charts**: `recharts`
- **Graph**: `@xyflow/react` (React Flow) — used for the wallet trace page
- **Icons**: `lucide-react`
- **Port**: `3000`

## Design system (DO NOT deviate)

All tokens live in `src/app/globals.css` as CSS custom properties:

```css
--background / --background-secondary   Page backgrounds
--card / --card-hover                   Card surfaces
--foreground / -secondary / -tertiary / -muted   Text hierarchy
--border / --border-strong              Borders
--primary / --primary-dim               Cyan accent (#00d6e3)
--accent-green/red/amber/purple/blue    Semantic colours
--font-sans   IBM Plex Sans
--font-mono   IBM Plex Mono
--radius      0rem  ← zero everywhere (Orb signature)
```

**Reusable CSS classes** (use these instead of inventing new styles):
- `.atlas-card` — card surface with border
- `.atlas-table` — full-width table with mono headers/cells
- `.statlabel` — tiny uppercase mono chip for section labels
- `.skeleton` — shimmer loading placeholder
- `.live-dot` — pulsing green dot for live data
- `.divide-grid > *` — bordered grid cell pattern (for hero stat strips)

## File structure
```
apps/explorer/src/
├── app/
│   ├── layout.tsx              Root layout (Nav + body styles)
│   ├── globals.css             Design system tokens + utility classes
│   ├── page.tsx                Home / search page
│   ├── stats/page.tsx          Network stats dashboard
│   ├── address/[addr]/page.tsx Wallet history + profile
│   ├── tx/[sig]/page.tsx       Transaction detail
│   ├── trace/[address]/page.tsx Wallet counterparty graph (React Flow)
│   └── api/atlas/[...path]/
│       └── route.ts            Server-side API proxy (injects API key)
├── components/
│   ├── Nav.tsx                 Sticky top nav with search
│   ├── stats/
│   │   ├── StatCard.tsx        Single stat chip (label + value + accent bar)
│   │   ├── TpsChart.tsx        recharts AreaChart for TPS history
│   │   ├── EpochCard.tsx       Epoch progress bar + slot detail
│   │   ├── RecentBlocks.tsx    Live-updating block feed
│   │   ├── ClientDistribution.tsx  Validator client donut + version bar chart
│   │   └── ValidatorTable.tsx  Paginated validator table
│   └── trace/
│       ├── TraceGraph.tsx      React Flow canvas (radial layout, animated edges)
│       ├── WalletNode.tsx      Custom node component
│       ├── TraceSidebar.tsx    Filter panel + counterparty list
│       └── types.ts            TraceData, TraceFilters, WalletNodeData interfaces
└── lib/
    ├── atlasRpc.ts             rpc<T>() and atlasGet<T>() — all backend calls
    └── api.ts                  fetchTx, fetchAddressTxs, fetchWalletProfile, etc.
```

## API calls — ALWAYS use the proxy
Client components MUST call `/api/atlas/...` (the Next.js proxy route), never `http://localhost:8080` directly. The proxy injects `ATLAS_API_KEY` server-side.

```typescript
// ✅ Correct (uses proxy)
const data = await atlasGet<T>('/v1/network/pulse');  // from atlasRpc.ts

// ❌ Wrong (leaks API key, broken in browser)
fetch('http://localhost:8080/v1/network/pulse', { headers: { 'X-API-Key': '...' } })
```

## Adding a new page
1. Create `src/app/<route>/page.tsx`
2. Use `atlas-card` + `statlabel` + CSS variable tokens — no hardcoded hex colors
3. Add the link to `Nav.tsx` LINKS array
4. Fetch data via `atlasRpc.ts` or `api.ts` (server components can import directly; client components use `atlasGet`/`rpc`)

## Environment variables
```
ATLAS_API_URL=http://localhost:8080       (server-side only)
ATLAS_API_KEY=atlas-dev-key-change-me    (server-side only, never NEXT_PUBLIC_)
NEXT_PUBLIC_MY_VALIDATOR=<your-node-pubkey>   (optional — highlights your validator in table)
```

## Common debugging
```bash
cd apps/explorer

# Dev server
npm run dev

# Type check
npx tsc --noEmit

# Build
npm run build
```

import { NextRequest, NextResponse } from 'next/server'

const ATLAS_API = process.env.ATLAS_API_URL ?? 'http://localhost:8080'
const ATLAS_KEY = process.env.ATLAS_API_KEY ?? ''

/**
 * Server-side proxy to the Atlas API.
 * Adds X-API-Key without exposing it to the browser.
 * Accessible from client components as /api/atlas/v1/...
 */
async function handler(
  req: NextRequest,
  { params }: { params: { path: string[] } }
) {
  const path     = params.path.join('/')
  const search   = req.nextUrl.search
  const upstream = `${ATLAS_API}/${path}${search}`

  const headers = new Headers()
  headers.set('X-API-Key', ATLAS_KEY)
  headers.set('Content-Type', 'application/json')
  // Forward Accept header (TOON negotiation)
  const accept = req.headers.get('Accept')
  if (accept) headers.set('Accept', accept)

  const init: RequestInit = {
    method:  req.method,
    headers,
  }

  if (req.method !== 'GET' && req.method !== 'HEAD') {
    init.body = req.body
    // @ts-ignore — duplex required for streaming body
    init.duplex = 'half'
  }

  const upstream_res = await fetch(upstream, init)
  const body         = await upstream_res.text()

  return new NextResponse(body, {
    status:  upstream_res.status,
    headers: {
      'Content-Type': upstream_res.headers.get('Content-Type') ?? 'application/json',
    },
  })
}

export { handler as GET, handler as POST, handler as DELETE, handler as PATCH }

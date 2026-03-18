import type { Metadata } from 'next'
import './globals.css'

export const metadata: Metadata = {
  title: 'Atlas Explorer — X1',
  description: 'Production blockchain explorer for X1',
}

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link
          href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600&family=Inter:wght@400;500;600&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="bg-surface text-gray-100 font-sans antialiased min-h-screen">
        <nav className="border-b border-surface-border px-6 py-3 flex items-center gap-6">
          <a href="/" className="text-brand font-mono font-semibold text-lg tracking-tight">
            ◈ Atlas
          </a>
          <span className="text-xs text-gray-500 border border-surface-border rounded px-2 py-0.5">
            {process.env.NEXT_PUBLIC_CHAIN?.toUpperCase() ?? 'X1'}
          </span>
        </nav>
        <main className="max-w-6xl mx-auto px-4 py-8">
          {children}
        </main>
      </body>
    </html>
  )
}

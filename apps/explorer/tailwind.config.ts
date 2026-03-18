import type { Config } from 'tailwindcss'

const config: Config = {
  content: ['./src/**/*.{js,ts,jsx,tsx,mdx}'],
  theme: {
    extend: {
      colors: {
        brand: {
          DEFAULT: '#00D4FF',
          dark:    '#0099BB',
          muted:   '#003344',
        },
        surface: {
          DEFAULT: '#0A0E1A',
          raised:  '#111827',
          border:  '#1F2937',
        },
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'monospace'],
      },
    },
  },
  plugins: [],
}

export default config

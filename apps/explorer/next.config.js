/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'standalone',
  env: {
    NEXT_PUBLIC_API_URL:  process.env.NEXT_PUBLIC_API_URL  || 'http://localhost:8080',
    NEXT_PUBLIC_CHAIN:   process.env.NEXT_PUBLIC_CHAIN    || 'x1',
  },
};

module.exports = nextConfig;

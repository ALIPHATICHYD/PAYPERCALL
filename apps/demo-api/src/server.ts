// A stand-in for a seller's real API, so the paid flow can be demoed end to
// end. Every response is SAMPLE data and says so: swap in a real API (and
// point the listing's "upstream" at it) before showing real numbers.
import { createServer } from 'node:http';

const port = Number(process.env.DEMO_API_PORT || 4100);

const routes: Record<string, (q: URLSearchParams) => unknown> = {
  '/market-prices': (q) => {
    const city = (q.get('city') ?? 'lagos').toLowerCase();
    return {
    sample: true,
    note: 'Sample data for demos, not real prices.',
    city,
    currency: city === 'accra' ? 'GHS' : 'NGN',
    items: [
      { item: 'rice (50kg bag)', price: null },
      { item: 'garri (1 paint bucket)', price: null },
      { item: 'tomatoes (1 basket)', price: null },
    ],
    };
  },
  '/fx/ngn': () => ({
    sample: true,
    note: 'Sample data for demos, not real rates.',
    base: 'NGN',
    rates: { USD: null, GBP: null, GHS: null },
  }),
};

export const server = createServer((req, res) => {
  const url = new URL(req.url ?? '/', 'http://localhost');
  const handler = req.method === 'GET' ? routes[url.pathname] : undefined;
  const body = handler ? handler(url.searchParams) : { error: 'Not found' };
  res.writeHead(handler ? 200 : 404, { 'content-type': 'application/json' });
  res.end(JSON.stringify(body));
});

if (import.meta.main ?? process.argv[1]?.endsWith('server.ts')) {
  server.listen(port, () => console.log(`Demo API on http://localhost:${port}`));
}

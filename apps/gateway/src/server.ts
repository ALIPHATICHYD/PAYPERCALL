import express from 'express';
import { HTTPFacilitatorClient } from '@x402/core/server';
import { paymentMiddleware, x402ResourceServer } from '@x402/express';
import { ExactStellarScheme } from '@x402/stellar/exact/server';
import { readConfig } from './config.ts';
import { Ledger } from './ledger.ts';
import { loadListings, publicView, routePath, toRoutesConfig } from './listings.ts';
import { forwardGet } from './proxy.ts';

const config = readConfig();
const listings = await loadListings(config.listingsFile, config.payTo);
const ledger = new Ledger(listings);

// The facilitator verifies each signed payment and settles it on Stellar.
// The OpenZeppelin facilitator needs a Bearer API key; x402.org's does not.
const apiKey = config.facilitatorApiKey;
const facilitator = new HTTPFacilitatorClient({
  url: config.facilitatorUrl,
  ...(apiKey && {
    createAuthHeaders: async () => {
      const headers = { Authorization: `Bearer ${apiKey}` };
      return { verify: headers, settle: headers, supported: headers };
    },
  }),
});

const resourceServer = new x402ResourceServer(facilitator)
  .register(config.network, new ExactStellarScheme())
  .onAfterSettle(async (ctx: any) => {
    // SettleResponse fields per x402 v2: success, transaction, network, payer
    const result = ctx?.result ?? {};
    ledger.recordSettlement({
      transaction: result.transaction ?? null,
      payer: result.payer ?? null,
      network: result.network ?? config.network,
    });
    console.log(`[settled] tx=${result.transaction ?? '?'} payer=${result.payer ?? '?'}`);
  });

const app = express();
app.disable('x-powered-by');

// Free routes: health, the public directory and seller stats.
app.get('/health', (_req, res) => {
  res.json({ ok: true, network: config.network, listings: listings.length });
});
app.get('/v1/listings', (_req, res) => {
  res.json({ listings: listings.map((l) => publicView(l, config.network)) });
});
app.get('/v1/stats', (_req, res) => {
  res.json(ledger.stats());
});

// Paid routes: x402 answers 402 with the price until a valid payment arrives.
app.use(paymentMiddleware(toRoutesConfig(listings, config.network), resourceServer));

for (const listing of listings) {
  app.get(routePath(listing), async (req, res) => {
    const search = req.originalUrl.includes('?') ? req.originalUrl.slice(req.originalUrl.indexOf('?')) : '';
    const upstream = await forwardGet(listing.upstream, search);
    // x402 only settles (charges) when the response succeeds, so only count those.
    if (upstream.status < 400) ledger.recordCall(listing.id);
    res.status(upstream.status).type(upstream.contentType).send(upstream.body);
  });
}

app.use((_req, res) => {
  res.status(404).json({ error: 'Not found' });
});

app.listen(config.port, () => {
  console.log(`PayPerCall gateway on http://localhost:${config.port} (${config.network})`);
  for (const l of listings) console.log(`  ${l.price.padEnd(8)} GET ${routePath(l)}  -> ${l.name}`);
});

import { readFile } from 'node:fs/promises';
import { parsePrice } from './price.ts';

/** One paid API sold through the gateway. */
export interface Listing {
  /** URL-safe id; the paid route is GET /v1/call/<id> */
  id: string;
  name: string;
  description: string;
  /** Where the gateway forwards a paid request (the seller's real API). */
  upstream: string;
  /** Price per call as a dollar string, e.g. "$0.005" (USDC). */
  price: string;
  /** Stellar G... address that receives payment. Falls back to PAY_TO. */
  payTo?: string;
  category?: string;
}

const ID_RE = /^[a-z0-9][a-z0-9-]{1,62}$/;
const STELLAR_ACCOUNT_RE = /^G[A-Z2-7]{55}$/;

export function isStellarAccount(value: string): boolean {
  return STELLAR_ACCOUNT_RE.test(value);
}

export function routePath(listing: Pick<Listing, 'id'>): string {
  return `/v1/call/${listing.id}`;
}

/**
 * Check listings and fill in the default payout address.
 * Throws one error listing every problem, so a bad file fails fast at startup.
 */
export function validateListings(input: unknown, defaultPayTo?: string): Listing[] {
  if (!Array.isArray(input)) throw new Error('Listings file must contain a JSON array');
  const problems: string[] = [];
  const seen = new Set<string>();
  const out: Listing[] = [];

  input.forEach((raw, i) => {
    const where = `listing[${i}]`;
    if (!raw || typeof raw !== 'object') {
      problems.push(`${where}: must be an object`);
      return;
    }
    const l = raw as Record<string, unknown>;
    const id = typeof l.id === 'string' ? l.id : '';
    if (!ID_RE.test(id)) problems.push(`${where}: id must be lowercase letters, digits or "-" (2-63 chars)`);
    else if (seen.has(id)) problems.push(`${where}: duplicate id "${id}"`);
    seen.add(id);

    for (const key of ['name', 'description', 'upstream', 'price'] as const) {
      if (typeof l[key] !== 'string' || !(l[key] as string).trim()) problems.push(`${where}: "${key}" is required`);
    }
    if (typeof l.upstream === 'string') {
      try {
        const u = new URL(l.upstream);
        if (u.protocol !== 'http:' && u.protocol !== 'https:') problems.push(`${where}: upstream must be http(s)`);
      } catch {
        problems.push(`${where}: upstream is not a valid URL`);
      }
    }
    if (typeof l.price === 'string') {
      try {
        parsePrice(l.price);
      } catch (e) {
        problems.push(`${where}: ${(e as Error).message}`);
      }
    }
    const payTo = typeof l.payTo === 'string' && l.payTo ? l.payTo : defaultPayTo;
    if (!payTo) problems.push(`${where}: no payTo address and PAY_TO is not set`);
    else if (!isStellarAccount(payTo)) problems.push(`${where}: payTo must be a Stellar account address (G...)`);

    out.push({
      id,
      name: String(l.name ?? ''),
      description: String(l.description ?? ''),
      upstream: String(l.upstream ?? ''),
      price: String(l.price ?? ''),
      payTo: payTo ?? '',
      category: typeof l.category === 'string' ? l.category : undefined,
    });
  });

  if (problems.length) throw new Error(`Invalid listings:\n  - ${problems.join('\n  - ')}`);
  return out;
}

export async function loadListings(file: string, defaultPayTo?: string): Promise<Listing[]> {
  const text = await readFile(file, 'utf8');
  return validateListings(JSON.parse(text), defaultPayTo);
}

/** The x402 route config: one paid GET route per listing. */
export function toRoutesConfig(listings: Listing[], network: string) {
  return Object.fromEntries(
    listings.map((l) => [
      `GET ${routePath(l)}`,
      {
        accepts: { scheme: 'exact', price: l.price, network, payTo: l.payTo as string },
        description: l.description,
      },
    ]),
  );
}

/** What the public directory shows for a listing (never the upstream URL). */
export function publicView(l: Listing, network: string) {
  return {
    id: l.id,
    name: l.name,
    description: l.description,
    price: l.price,
    asset: 'USDC',
    network,
    payTo: l.payTo,
    category: l.category ?? null,
    path: routePath(l),
  };
}

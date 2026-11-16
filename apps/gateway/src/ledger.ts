import { formatUnits, parsePrice } from './price.ts';
import type { Listing } from './listings.ts';

export interface Settlement {
  at: string;
  transaction: string | null;
  payer: string | null;
  network: string | null;
}

/**
 * In-memory record of paid calls and settlements for the seller dashboard.
 * Resets when the gateway restarts; a database comes later.
 */
export class Ledger {
  private calls = new Map<string, number>();
  private settlements: Settlement[] = [];
  private payers = new Set<string>();
  private readonly listings: Listing[];
  private readonly keep: number;

  constructor(listings: Listing[], keep = 50) {
    this.listings = listings;
    this.keep = keep;
  }

  /** A request passed payment verification and reached the upstream API. */
  recordCall(listingId: string): void {
    this.calls.set(listingId, (this.calls.get(listingId) ?? 0) + 1);
  }

  /** The facilitator settled a payment on-chain. */
  recordSettlement(s: Omit<Settlement, 'at'> & { at?: string }): void {
    const entry: Settlement = { at: s.at ?? new Date().toISOString(), transaction: s.transaction, payer: s.payer, network: s.network };
    this.settlements.unshift(entry);
    if (this.settlements.length > this.keep) this.settlements.length = this.keep;
    if (entry.payer) this.payers.add(entry.payer);
  }

  stats() {
    let totalCalls = 0;
    let totalUnits = 0n;
    const perListing = this.listings.map((l) => {
      const calls = this.calls.get(l.id) ?? 0;
      const earned = parsePrice(l.price) * BigInt(calls);
      totalCalls += calls;
      totalUnits += earned;
      return { id: l.id, name: l.name, price: l.price, calls, earnedUsdc: formatUnits(earned) };
    });
    return {
      totalCalls,
      totalEarnedUsdc: formatUnits(totalUnits),
      uniquePayers: this.payers.size,
      listings: perListing,
      recentSettlements: [...this.settlements],
    };
  }
}

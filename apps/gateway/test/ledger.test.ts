import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Ledger } from '../src/ledger.ts';
import type { Listing } from '../src/listings.ts';

const listings: Listing[] = [
  { id: 'a', name: 'A', description: '', upstream: 'http://x', price: '$0.005', payTo: 'G' },
  { id: 'b', name: 'B', description: '', upstream: 'http://x', price: '$0.001', payTo: 'G' },
];

test('stats add up calls and earnings exactly', () => {
  const ledger = new Ledger(listings);
  for (let i = 0; i < 3; i++) ledger.recordCall('a');
  ledger.recordCall('b');
  const s = ledger.stats();
  assert.equal(s.totalCalls, 4);
  assert.equal(s.totalEarnedUsdc, '0.016');
  assert.deepEqual(s.listings.map((l) => [l.id, l.calls, l.earnedUsdc]), [['a', 3, '0.015'], ['b', 1, '0.001']]);
});

test('settlements are newest first, capped, and count unique payers', () => {
  const ledger = new Ledger(listings, 2);
  ledger.recordSettlement({ transaction: 't1', payer: 'P1', network: 'stellar:testnet' });
  ledger.recordSettlement({ transaction: 't2', payer: 'P1', network: 'stellar:testnet' });
  ledger.recordSettlement({ transaction: 't3', payer: 'P2', network: 'stellar:testnet' });
  const s = ledger.stats();
  assert.deepEqual(s.recentSettlements.map((x) => x.transaction), ['t3', 't2']);
  assert.equal(s.uniquePayers, 2);
});

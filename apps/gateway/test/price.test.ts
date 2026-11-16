import { test } from 'node:test';
import assert from 'node:assert/strict';
import { formatUnits, parsePrice } from '../src/price.ts';

test('parsePrice turns dollar strings into USDC base units', () => {
  assert.equal(parsePrice('$0.005'), 50_000n);
  assert.equal(parsePrice('$1'), 10_000_000n);
  assert.equal(parsePrice('$0.0000001'), 1n);
  assert.equal(parsePrice(' $2.5 '), 25_000_000n);
});

test('parsePrice rejects bad prices', () => {
  for (const bad of ['0.005', '$', '$0', '$0.00000001', '$-1', '$1,000', 'free']) {
    assert.throws(() => parsePrice(bad), /Invalid price/, bad);
  }
});

test('formatUnits round-trips and trims zeros', () => {
  assert.equal(formatUnits(50_000n), '0.005');
  assert.equal(formatUnits(10_000_000n), '1');
  assert.equal(formatUnits(0n), '0');
  assert.equal(formatUnits(parsePrice('$12.3456789')), '12.3456789');
});

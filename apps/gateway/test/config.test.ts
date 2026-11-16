import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readConfig } from '../src/config.ts';

test('defaults to testnet and the OpenZeppelin facilitator', () => {
  const c = readConfig({});
  assert.equal(c.network, 'stellar:testnet');
  assert.equal(c.port, 4021);
  assert.equal(c.facilitatorUrl, 'https://channels.openzeppelin.com/x402/testnet');
  assert.equal(c.facilitatorApiKey, undefined);
  assert.match(c.listingsFile, /apps\/gateway\/listings\.json$/);
});

test('rejects non-Stellar networks and bad ports', () => {
  assert.throws(() => readConfig({ NETWORK: 'eip155:8453' }), /NETWORK must be/);
  assert.throws(() => readConfig({ PORT: 'abc' }), /PORT must be/);
});

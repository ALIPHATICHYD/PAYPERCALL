import { test } from 'node:test';
import assert from 'node:assert/strict';
import { publicView, routePath, toRoutesConfig, validateListings } from '../src/listings.ts';

const PAY_TO = 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5';
const good = {
  id: 'market-prices',
  name: 'Market prices',
  description: 'Prices',
  upstream: 'http://localhost:4100/market-prices',
  price: '$0.005',
};

test('valid listings get the default payout address', () => {
  const [l] = validateListings([good], PAY_TO);
  assert.equal(l.payTo, PAY_TO);
  assert.equal(routePath(l), '/v1/call/market-prices');
});

test('a listing can override the payout address', () => {
  const own = 'GA' + 'A'.repeat(54);
  const [l] = validateListings([{ ...good, payTo: own }], PAY_TO);
  assert.equal(l.payTo, own);
});

test('every problem is reported at once', () => {
  assert.throws(
    () => validateListings([{ ...good, id: 'Bad ID', price: 'free', upstream: 'ftp://x' }, good, good], undefined),
    (err: Error) => {
      for (const part of ['id must be', 'Invalid price', 'upstream must be http', 'no payTo', 'duplicate id']) {
        assert.match(err.message, new RegExp(part));
      }
      return true;
    },
  );
});

test('non-array input is rejected', () => {
  assert.throws(() => validateListings({}, PAY_TO), /JSON array/);
});

test('route config has one exact-scheme route per listing', () => {
  const routes = toRoutesConfig(validateListings([good], PAY_TO), 'stellar:testnet');
  assert.deepEqual(routes, {
    'GET /v1/call/market-prices': {
      accepts: { scheme: 'exact', price: '$0.005', network: 'stellar:testnet', payTo: PAY_TO },
      description: 'Prices',
    },
  });
});

test('the public view never exposes the upstream URL', () => {
  const [l] = validateListings([good], PAY_TO);
  const view = publicView(l, 'stellar:testnet');
  assert.equal('upstream' in view, false);
  assert.equal(view.path, '/v1/call/market-prices');
});

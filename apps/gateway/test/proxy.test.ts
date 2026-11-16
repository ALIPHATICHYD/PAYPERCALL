import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import type { AddressInfo } from 'node:net';
import { server } from '../../demo-api/src/server.ts';
import { forwardGet } from '../src/proxy.ts';

await new Promise<void>((resolve) => server.listen(0, resolve));
const base = `http://localhost:${(server.address() as AddressInfo).port}`;
after(() => server.close());

test('forwards the query string to the upstream API', async () => {
  const r = await forwardGet(`${base}/market-prices`, '?city=accra');
  assert.equal(r.status, 200);
  assert.match(r.contentType, /application\/json/);
  const body = JSON.parse(r.body.toString());
  assert.equal(body.city, 'accra');
  assert.equal(body.sample, true);
});

test('passes upstream errors through', async () => {
  const r = await forwardGet(`${base}/nope`, '');
  assert.equal(r.status, 404);
});

test('an unreachable upstream becomes a 502 without leaking details', async () => {
  const r = await forwardGet('http://127.0.0.1:1/x', '');
  assert.equal(r.status, 502);
  assert.deepEqual(JSON.parse(r.body.toString()), { error: 'Upstream API unreachable' });
});

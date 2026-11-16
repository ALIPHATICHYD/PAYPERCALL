/** Result of forwarding a paid request to the seller's API. */
export interface UpstreamResult {
  status: number;
  contentType: string;
  body: Buffer;
}

/**
 * Forward a GET to the upstream API, keeping the caller's query string.
 * Upstream failures become a 502 so the gateway never leaks internals.
 */
export async function forwardGet(upstream: string, search: string, timeoutMs = 10_000): Promise<UpstreamResult> {
  const target = new URL(upstream);
  const incoming = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);
  for (const [k, v] of incoming) target.searchParams.append(k, v);

  try {
    const res = await fetch(target, {
      method: 'GET',
      headers: { accept: 'application/json, */*' },
      signal: AbortSignal.timeout(timeoutMs),
    });
    return {
      status: res.status,
      contentType: res.headers.get('content-type') ?? 'application/octet-stream',
      body: Buffer.from(await res.arrayBuffer()),
    };
  } catch (err) {
    const reason = (err as Error).name === 'TimeoutError' ? 'timed out' : 'unreachable';
    return {
      status: 502,
      contentType: 'application/json',
      body: Buffer.from(JSON.stringify({ error: `Upstream API ${reason}` })),
    };
  }
}

// Prices are written as dollar strings ("$0.005"), the format x402 accepts
// for USDC. Stellar USDC has 7 decimal places, so we keep money as integer
// base units (1 USDC = 10_000_000) to avoid floating-point drift.

export const USDC_DECIMALS = 7;
const SCALE = 10n ** BigInt(USDC_DECIMALS);
const PRICE_RE = /^\$(\d+)(?:\.(\d{1,7}))?$/;

/** Parse "$0.005" into USDC base units (50000n). Throws on bad input. */
export function parsePrice(price: string): bigint {
  const m = PRICE_RE.exec(price.trim());
  if (!m) {
    throw new Error(`Invalid price "${price}": use a dollar string like "$0.005" (max 7 decimals)`);
  }
  const whole = BigInt(m[1]);
  const frac = BigInt((m[2] ?? '').padEnd(USDC_DECIMALS, '0'));
  const units = whole * SCALE + frac;
  if (units <= 0n) throw new Error(`Invalid price "${price}": must be greater than zero`);
  return units;
}

/** Format base units as a plain USDC amount, e.g. 50000n -> "0.005". */
export function formatUnits(units: bigint): string {
  const neg = units < 0n;
  const abs = neg ? -units : units;
  const whole = abs / SCALE;
  const frac = (abs % SCALE).toString().padStart(USDC_DECIMALS, '0').replace(/0+$/, '');
  return `${neg ? '-' : ''}${whole}${frac ? '.' + frac : ''}`;
}

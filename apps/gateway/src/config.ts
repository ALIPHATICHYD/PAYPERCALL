import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(here, '..');

export interface GatewayConfig {
  port: number;
  network: string;
  facilitatorUrl: string;
  facilitatorApiKey: string | undefined;
  payTo: string | undefined;
  listingsFile: string;
}

const NETWORK_RE = /^stellar:(testnet|pubnet)$/;

export function readConfig(env: NodeJS.ProcessEnv = process.env): GatewayConfig {
  const network = env.NETWORK || 'stellar:testnet';
  if (!NETWORK_RE.test(network)) throw new Error(`NETWORK must be stellar:testnet or stellar:pubnet, got "${network}"`);

  const port = Number(env.PORT || 4021);
  if (!Number.isInteger(port) || port <= 0) throw new Error(`PORT must be a positive integer, got "${env.PORT}"`);

  return {
    port,
    network,
    facilitatorUrl: env.FACILITATOR_URL || 'https://channels.openzeppelin.com/x402/testnet',
    facilitatorApiKey: env.FACILITATOR_API_KEY || undefined,
    payTo: env.PAY_TO || undefined,
    listingsFile: path.resolve(appRoot, env.LISTINGS_FILE || 'listings.json'),
  };
}

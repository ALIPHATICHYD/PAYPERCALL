// Demo agent: pays for one call to a PayPerCall listing, the way an AI agent
// would. Follows the official Stellar x402 quickstart client.
//
//   npm run pay -- sample-market-prices "city=lagos"
import { Transaction, TransactionBuilder } from '@stellar/stellar-sdk';
import { x402Client, x402HTTPClient } from '@x402/fetch';
import { createEd25519Signer, getNetworkPassphrase } from '@x402/stellar';
import { ExactStellarScheme } from '@x402/stellar/exact/client';

const NETWORK = process.env.NETWORK || 'stellar:testnet';
const GATEWAY = process.env.GATEWAY_URL || 'http://localhost:4021';
const RPC_URL = process.env.STELLAR_RPC_URL || 'https://soroban-testnet.stellar.org';
const SECRET = process.env.STELLAR_PRIVATE_KEY;

const [listingId = 'sample-market-prices', query = ''] = process.argv.slice(2);

async function main() {
  if (!SECRET) throw new Error('Set STELLAR_PRIVATE_KEY (a funded testnet secret key, S...) in .env');

  const url = new URL(`/v1/call/${listingId}`, GATEWAY);
  if (query) url.search = query;

  const signer = createEd25519Signer(SECRET, NETWORK);
  const client = new x402Client().register('stellar:*', new ExactStellarScheme(signer, { url: RPC_URL }));
  const http = new x402HTTPClient(client);
  console.log(`agent   ${signer.address}\ntarget  ${url}`);

  // 1. Ask without paying: the gateway answers 402 with the price.
  const first = await fetch(url);
  if (first.status !== 402) {
    console.log(`No payment needed (${first.status}):`, await first.text());
    return;
  }
  const required = http.getPaymentRequiredResponse((name) => first.headers.get(name));
  console.log('402     payment required');

  // 2. Sign the USDC payment (a Soroban authorization entry).
  let payload = await client.createPaymentPayload(required);
  const passphrase = getNetworkPassphrase(NETWORK);
  const tx = new Transaction(payload.payload.transaction, passphrase);
  const envelope = tx.toEnvelope();
  if (envelope.type !== 'envelopeTypeTx') throw new Error('Expected a v1 transaction envelope');
  const ext = envelope.value.tx.ext;
  const sorobanData = ext.type === 'sorobanData' ? ext.value : undefined;
  if (sorobanData) {
    // 1-stroop fee, as in the official quickstart, to stay within the
    // testnet facilitator's fee limit (the facilitator pays network fees).
    payload = {
      ...payload,
      payload: {
        ...payload.payload,
        transaction: TransactionBuilder.cloneFrom(tx, { fee: '1', sorobanData, networkPassphrase: passphrase }).build().toXDR(),
      },
    };
  }

  // 3. Retry with the signed payment; the facilitator settles on Stellar.
  const paid = await fetch(url, { headers: http.encodePaymentSignatureHeader(payload) });
  const body = await paid.text();
  const settle = http.getPaymentSettleResponse((name) => paid.headers.get(name));
  console.log(`${paid.status}     ${body}`);
  console.log('settled', settle);
}

main().catch((err) => {
  console.error('Agent failed:', err);
  process.exit(1);
});

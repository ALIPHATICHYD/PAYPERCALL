# PayPerCall

**Put a price on any API in minutes and get paid in USDC by AI agents worldwide, per request, on Stellar.**

PayPerCall lets developers in Africa sell access to their APIs, datasets and AI tools to anyone in the world, including AI agents, without card billing, a foreign bank account or API keys. Each call is paid for in USDC, a cent or less, and the money settles straight into the seller's own Stellar wallet within seconds.

> Built for the **Stellar Buildathon 2026** (Nigeria & Ghana). Work in progress.

---

## The problem

- **Developers here struggle to get paid by customers abroad.** International payment processors and card billing are hard to access from Nigeria and Ghana, and selling API access usually means building accounts, keys and subscriptions first. So many useful tools never earn outside their home market.
- **AI agents need to pay for things on their own.** An agent doing a task can't sign up for a subscription or type in a card number. It needs to pay per request, automatically.

PayPerCall connects the two. The buyer is a global agent or developer that already holds stablecoins, so the seller doesn't depend on local stablecoin adoption to get paid.

## How it works

Think of a vending machine: no account, no contract. You put in a coin and get what you came for. PayPerCall turns an API into one.

1. **The agent calls the API**, e.g. `GET /v1/market-prices?city=lagos`.
2. **The PayPerCall gateway replies `402 Payment Required`** with the price, network and payment details.
3. **The agent signs a Soroban authorization entry** for the USDC payment and retries the request.
4. **The payment is verified and settled on Stellar** through an x402 facilitator.
5. **The gateway returns the data.** The seller has already been paid.

```
Agent ──GET──▶ Gateway ──402 + price──▶ Agent
Agent ──signed payment──▶ Gateway ──verify/settle──▶ Stellar (USDC)
Gateway ──200 OK + data──▶ Agent        Seller wallet ◀── USDC
```

## Features (Buildathon scope)

- **Seller onboarding:** wrap an existing API, set a price per call and a payout wallet.
- **x402 gateway:** middleware that protects an endpoint and charges per request.
- **On-chain endpoint registry:** a Soroban contract that lists each paid API (price, payout address, description), so agents can discover APIs without a central marketplace.
- **Seller dashboard:** paid calls, earnings and a log of payments with their Stellar transaction IDs.
- **MCP listing:** paid APIs exposed as tools, so assistants such as Claude can find and pay for them.
- **Two live demo APIs** to show the full flow end to end.

## Built with

| Layer | Technology |
|---|---|
| Payments | [x402 on Stellar](https://developers.stellar.org/docs/build/agentic-payments), USDC |
| Smart contracts | Soroban (Rust) |
| Gateway / backend | Node.js, TypeScript |
| Frontend | Next.js, React |
| Agent discovery | Model Context Protocol (MCP) |

## Why Stellar

- **Settlement in seconds** and **fees of a tiny fraction of a cent**, which is what makes charging per call viable.
- **Native USDC**, a stable unit for pricing.
- **Soroban authorization**, which lets an agent approve one exact payment without handing over its keys.
- **An x402 facilitator already live on Stellar**, so we build the seller experience instead of rebuilding payment plumbing.

## Project status

| Milestone | Status |
|---|---|
| Idea, scope and UI mockups | Done |
| x402 gateway with test payments | Built, needs a live testnet run |
| Soroban endpoint registry | Built and tested, not yet deployed |
| Seller onboarding and dashboard | Planned |
| MCP listing and demo APIs | Planned |
| Builder Showcase demo | Planned |

## Team

- **Fuad Adegoke**: team lead, full-stack developer ([@ALIPHATICHYD](https://github.com/ALIPHATICHYD))
- **Shobiye AbdulAzeem**: team member ([@Abdulazeem-code](https://github.com/Abdulazeem-code))

## Repository layout

| Path | What it is |
|---|---|
| `apps/gateway` | The x402 gateway. Reads `listings.json`, answers `402` with each listing's price, forwards paid requests to the seller's API, and tracks calls and settlements. |
| `apps/agent` | A demo AI-agent client that pays for one call with testnet USDC. |
| `apps/demo-api` | A stand-in for a seller's API that returns clearly-marked sample data. |
| `contracts/registry` | The Soroban endpoint registry: an on-chain list of paid APIs (price, payout address, description) that agents can read directly. |

## Getting started (testnet)

Requires Node.js 22.18 or newer (TypeScript runs directly, no build step).

1. **Install:** `npm install`
2. **Configure:** copy `.env.example` to `.env`.
3. **Seller wallet:** create a testnet account, fund it with XLM at [Stellar Lab](https://lab.stellar.org/account/fund), and add a USDC trustline (issuer `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5`). Put its `G...` address in `PAY_TO`.
4. **Facilitator key:** get a free testnet API key at [channels.openzeppelin.com/testnet/gen](https://channels.openzeppelin.com/testnet/gen) and set `FACILITATOR_API_KEY`.
5. **Agent wallet:** create a second testnet account with a USDC trustline, get testnet USDC from [faucet.circle.com](https://faucet.circle.com) (Stellar Testnet), and put its `S...` secret in `STELLAR_PRIVATE_KEY`. Testnet keys only.
6. **Run** each in its own terminal:

```bash
npm run dev:demo-api
npm run dev:gateway
npm run pay -- sample-market-prices "city=lagos"
```

The agent gets a `402`, signs a USDC payment, retries, and prints the data and the settlement. `GET /v1/stats` on the gateway then shows the paid call.

### Gateway routes

| Route | Paid | Purpose |
|---|---|---|
| `GET /health` | No | Status and network |
| `GET /v1/listings` | No | Public directory of paid APIs (upstream URLs are never exposed) |
| `GET /v1/stats` | No | Calls, earnings and recent settlements (in memory for now) |
| `GET /v1/call/<id>` | Yes | The paid route for each listing |

### Adding an API

Add an entry to `apps/gateway/listings.json`:

```json
{
  "id": "my-api",
  "name": "My API",
  "description": "What it returns, so agents know when to call it.",
  "upstream": "https://api.example.com/v1/data",
  "price": "$0.005"
}
```

`payTo` is optional and defaults to `PAY_TO`. The gateway checks every listing at startup and refuses to start if one is invalid.

### Tests

```bash
npm test
```

## Endpoint registry (Soroban)

`contracts/registry` is a permissionless Soroban contract that lists paid APIs on-chain, so agents can find them without a central marketplace.

- Anyone can `register` a listing; the signing address becomes its **seller**.
- Only the seller can `update`, pause/resume (`set_active`) or `remove` it.
- An **admin** (set at deploy) can `admin_remove` abusive listings, `set_admin` and `upgrade` the contract.
- Reads: `get`, `count`, `list(start, limit)` (max 50 per page) and `listings_of(seller)`.
- Events: `registered`, `updated` and `removed`, for indexers and the dashboard.

Each listing stores `id`, `seller`, `pay_to`, `name`, `description`, `endpoint`, `price`, `category`, `active` and the ledgers it was created and last updated in. `price` is in USDC base units (7 decimals, so `$0.005` = `50000`). `endpoint` is the public paid URL (the gateway's `/v1/call/<id>`). The seller's real upstream URL is never stored on-chain. Ids follow the same rule as `listings.json`. A seller can hold up to 50 listings.

Requires Rust with the `wasm32v1-none` target and the [Stellar CLI](https://developers.stellar.org/docs/tools/cli).

```bash
npm run contract:test     # cargo test
npm run contract:build    # target/wasm32v1-none/release/paypercall_registry.wasm
```

Deploy to testnet and register a listing:

```bash
stellar keys generate deployer --network testnet --fund
stellar contract deploy --wasm target/wasm32v1-none/release/paypercall_registry.wasm \
  --source deployer --network testnet --alias registry -- --admin deployer

stellar contract invoke --id registry --source deployer --network testnet -- register \
  --seller deployer --id sample-fx \
  --input '{"name":"Sample naira FX feed","description":"Sample naira exchange rates.","endpoint":"http://localhost:4021/v1/call/sample-fx","price":"10000","pay_to":"G...","category":"finance"}'

stellar contract invoke --id registry --network testnet -- list --start 0 --limit 10
```

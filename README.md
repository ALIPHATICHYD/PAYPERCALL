# PayPerCall

**Put a price on any API in minutes and get paid in USDC by AI agents worldwide, per request, on Stellar.**

PayPerCall lets developers in Africa sell access to their APIs, datasets and AI tools to anyone in the world, including AI agents, without card billing, a foreign bank account or API keys. Each call is paid for in USDC, a cent or less, and the money settles straight into the seller's own Stellar wallet within seconds.

> Built for the **Stellar Buildathon 2026** (Nigeria & Ghana). 🚧 Work in progress.

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
| Idea, scope and UI mockups | ✅ Done |
| x402 gateway with test payments | ⏳ In progress |
| Soroban endpoint registry | ⏳ Planned |
| Seller onboarding and dashboard | ⏳ Planned |
| MCP listing and demo APIs | ⏳ Planned |
| Builder Showcase demo | ⏳ Planned |

## Team

- **Fuad Adegoke**: team lead, full-stack developer ([@ALIPHATICHYD](https://github.com/ALIPHATICHYD))
- **Shobiye AbdulAzeem**: team member

## Getting started

Setup instructions will be added as the code lands.

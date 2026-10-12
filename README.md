# Marketplace Studio

An independent first-party Rust app for Amazon SP-API, eBay Sell and Kaufland Seller API.
Provider code, UI, manifest, packaging and tests stay in this directory. The Core
contains only generic authorized channel projections and external paid-order intake.

## Run

From the repository root:

```sh
cargo build --bin marketplace-gateway
APP_TOKEN='<gateway token>' APP_MARKETPLACE_TENANTS='<operator JSON>' \
  target/debug/marketplace-gateway
```

Deploy `extensions/apps/marketplaces/Dockerfile` with the repository as build context.
The runtime is non-root Rust with no Python, database administrator credential or
commerce server. `/health` reports process health, not working seller authorization.

Operator configuration, native app installation/consent, credential scopes,
provider prerequisites, walkthrough and current limitations:
[Marketplace channels](../../../docs/marketplaces.md).

## Ownership and verification

- `service/config.rs`, `transport.rs`: operator-only credentials, fixed official
  destinations, seller OAuth/HMAC and bounded responses.
- `service/amazon.rs`, `ebay.rs`, `kaufland.rs`: listing protocols.
- `service/mapping.rs`, `sync.rs`: inherited fields, per-offer policies, fresh native
  price/stock, CAS transmission fence and explicit synchronization.
- `service/providers.rs`, `intake.rs`, `review.rs`: paginated provider retrieval,
  private source inbox, unique SKU suggestions and reviewed native order import.
- `service/jobs.rs`: existing Core job leases, heartbeat, admission and cancellation.
- `studio/`: app-owned native Studio interface; one content-language selector.
- `tests/integration.py`: two isolated shops, synthetic seller HTTP APIs, signed
  requests, real PostgreSQL/native jobs, replay/restart/stock/document boundaries.

```sh
DATABASE_URL='<disposable PostgreSQL URL>' python3 scripts/marketplaces.py
```

No external provider requests or secret seller credentials are needed for this
fixture. Passing it does not certify production seller accounts or category schemas.

# Veyndo Marketplace Studio

Connect Amazon, eBay and Kaufland to a commerce host. Map product fields, maintain
channel-specific content and images, synchronize offers and review imported orders.
The same Rust provider code powers Marketplace Studio inside Veyndo Commerce.

This repository owns the provider gateway, manifest, Studio views, container and
tests. Its history was extracted from `veyndo/veyndo`, recorded in
[PROVENANCE.json](PROVENANCE.json). Changes belong here; Veyndo consumes a pinned,
verified source snapshot so ordinary clones and existing frontend builds work.

## Build and run

```sh
cargo build --locked --release
APP_TOKEN='<server gateway token, at least 24 characters>' \
APP_MARKETPLACE_TENANTS='<approved host and seller configuration>' \
  target/release/marketplace-gateway
# Or: docker build -t veyndo-marketplaces .
```

The component needs no Core source or database credentials. It calls its approved
host for durable records/jobs, current product/channel data and atomic paid-order
intake. Read the [host contract](docs/host-contract.md) before connecting another
platform. The existing [Veyndo setup and merchant workflow](https://veyndo.github.io/veyndo/docs/marketplaces.html)
contains the operator configuration and seller prerequisites.

The bundled Studio views integrate with Veyndo's existing UI host. A separately
embeddable frontend package and independent durable host adapter remain planned.

## HTTP, MCP and JavaScript

The server exposes `/health`, `/actions/{action}`, `/events` and `/mcp`. HTTP and
MCP share actions and effects. MCP schemas come from the manifest. Only trusted
server gateways may use the service credential; browser/user clients go through
the host's current permissions and installation consent.

[Run the JavaScript example](examples/prepare.mjs) with Node 22 or newer:

```sh
npm ci --ignore-scripts
MARKETPLACE_URL=https://marketplaces.example.com \
MARKETPLACE_TOKEN='<private gateway token>' MARKETPLACE_TENANT=your-shop \
MARKETPLACE_LISTING=your-offer node examples/prepare.mjs
```

## Verify

```sh
cargo test --locked
cargo build --locked
npm ci --ignore-scripts
npm test
```

Seven Rust tests exercise mapping, exact currency, provider prerequisites, source
addresses, event admission and fixed provider origins. The standalone test starts
the real binary with an independent host and exercises HTTP/MCP/SDK, shared state,
wrong credentials, foreign tenants, rejected imports and language inheritance.
It makes no external provider requests. CI also checks the production container.

`tests/integration.py` is the connected Veyndo/PostgreSQL/provider-fixture suite.
Run it through `python3 scripts/marketplaces.py` in the Veyndo composition. It
covers listing sync, paid import, replay/restart, stock, documents and permissions.
Its fixture results do not certify live seller accounts or category schemas.

Licensed under the original [Sustainable Use License](LICENSE).

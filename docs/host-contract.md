# Host boundary, version 1

The provider gateway builds and starts without the Veyndo process, its source or
its database. It uses a trusted host for durable records, leases, product/channel
projections and atomic order intake. The standalone test implements that host in
memory to exercise the wire contract. Durable independent storage is the next
extraction task, not supplied by the test host.

`APP_MARKETPLACE_TENANTS` maps operator-approved tenant IDs to a fixed HTTPS origin,
a private callback token and seller credential aliases. `coreUrl` is the legacy
configuration field name for this host origin. HTTP is allowed only on loopback.
The gateway appends `/api/apps/marketplaces/core/{operation}`. Every callback sends
`Authorization: Bearer <callbackToken>` and `x-tenant`. Redirects are disabled;
responses are bounded at 64 KiB. Tokens and provider secrets stay on the server.

| Operation | Host responsibility |
| --- | --- |
| `record_action` | Private entity reads/writes. Enforce tenant, current grants, installation consent/digest, revisions, unique listing/inbox identities and bounded indexed reads. |
| `channel`, `channel_product` | Current channel/catalog membership, enabled language/currency and exact native price/stock projection, with source revision. |
| `job_claim`, `job_progress` | Durable idempotent jobs, worker leases, fencing, renewal, cancellation and uncertain external effects. |
| `order_import` | Validate reviewed immutable source identity, exact paid amounts/taxes, product mappings and current authority; atomically commit order, stock and outbox. Replays must return the same order. |

The existing manifest defines record fields, actions, scopes and job input. The
production host must implement every operation used by the chosen workflow. It
must not accept a request tenant or amount as proof of authority. The Veyndo
adapter already performs these checks; use it when embedding in Veyndo.

`POST /actions/prepare`, `/actions/review_order` and `/actions/import_order` and
`POST /mcp` share the same dispatch. MCP exposes only these service actions, with
schemas read from `manifest.json`. Record CRUD and job creation remain host APIs.
`/events` admits the existing authenticated batches and claims jobs before work.

This is the stateless JSON MCP subset: initialize, tools/list and tools/call with
request IDs, protocol 2025-11-25. No SSE, session transport or notification stream.
The gateway token is an operator credential with authority over configured tenants.
Never distribute it to a browser, end user or autonomous agent. Such clients use
the host's per-user grant checks and public gateway, including Veyndo's `/mcp`.
Host callback keys remain revocable and scoped independently for each tenant.

A transport failure after a write is uncertain. Neither the SDK nor gateway
blindly retries. Inspect the durable job/listing fence and provider result before
explicit continuation. Process health does not establish seller authorization.

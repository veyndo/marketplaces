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

## Versioned host port and complete action surface

A tenant may set `hostPath`, for example `/commerce/v1/`, to connect an independent
host without implementing Veyndo URL names. The existing default path is unchanged.
Paths are operator configuration, fixed at startup, and reject foreign origins,
encoded separators, traversal, query and fragment data. Callbacks send
`x-veyndo-host-version: 1`. Veyndo accepts legacy callers without that header and
rejects an unsupported explicit version before executing an operation.

The canonical generic input contract is exported as
`@veyndo/sdk/host-contract`; `@veyndo/sdk/host` supplies generated TypeScript inputs
and a bounded server client. Its provenance points to the native source. See
[host.mjs](../examples/host.mjs) for an independent host URL.

All declared record list/save actions now work through `/actions/{name}` and MCP,
with unchanged record schemas and revisions. `sync` additionally accepts an
explicit `requestKey` (1..100 bytes). HTTP and MCP both delegate to `job_enqueue`:

```json
{"connectionId":"ebay-de","kind":"orders","requestKey":"stable-request-1"}
```

The host `job_enqueue` input is `{action, input, requestKey}`. It must select a
registered job action, validate its schema, current actor rights and additional
permissions, then persist the native idempotent job and its event atomically.
In Veyndo it enters the original app gateway and queue. It cannot call a service,
save data or execute arbitrary actions through this callback. Missing capability,
PII scope, revoked credential, foreign tenant, stale package and private workspace
retain the existing admission. Reusing the same key with changed input conflicts.

Other host operations remain `record_action`, `channel`, `channel_product`,
`order_import`, `job_claim` and `job_progress`. The host owns durable state and
commerce transactions. The direct gateway credential remains server authority;
per-user browser/agent clients must use the host permission gateway.

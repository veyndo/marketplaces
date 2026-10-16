# extensions/apps/marketplaces ownership

Root AGENTS.md also applies. Rules below were moved from the root, not weakened.

## Marketplace ownership

Seller protocols, mappings, credentials, inbox logic and Studio UI belong only in
`extensions/apps/marketplaces/`. Generic paid external-order/channel host contracts
belong to their existing Core domain owners. Use native app jobs/consent, tenant
references, exact currencies, stock, orders, guest projection, receipts and outbox;
never implement another marketplace commerce ledger, money calculator or queue.
An accepted submission is not a published offer; ambiguous writes must remain
fenced. Missing source tax/payment facts require explicit review, never estimates.
Run the registered `marketplaces` provider suite and marketplace UI regressions;
see `docs/marketplaces.md` for current provider boundaries.


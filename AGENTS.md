# Marketplace component ownership

This repository is the upstream owner of provider code, manifest, Studio views,
packaging and tests. Veyndo consumes an exact source snapshot; do not hand-edit
that snapshot. Standalone verification: cargo test --locked, cargo clippy --locked
and npm test. The connected fixture uses VEYNDO_ROOT and disposable PostgreSQL.



Seller protocols, mappings, credentials, inbox logic and Studio UI belong only in
`extensions/apps/marketplaces/`. Generic paid external-order/channel host contracts
belong to their existing Core domain owners. Use native app jobs/consent, tenant
references, exact currencies, stock, orders, guest projection, receipts and outbox;
never implement another marketplace commerce ledger, money calculator or queue.
An accepted submission is not a published offer; ambiguous writes must remain
fenced. Missing source tax/payment facts require explicit review, never estimates.
Run the registered `marketplaces` provider suite and marketplace UI regressions;
see `docs/host-contract.md` for current provider boundaries.


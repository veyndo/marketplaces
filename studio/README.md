# Marketplace workspace

App-owned connections, channel-specific offer editor, mapping preview, source-order
review and native job status. All data/actions use the existing authenticated Studio
request transport and app manifest. No provider credential, extra ledger or UI queue.

`MarketplacePanel` coordinates views; `MarketplaceOffers` renders cards;
`ConnectionEditor` configures seller aliases and native channels; `OfferEditor`
uses one content-language selector and inherited overrides; `OrderReview` and
`OrderAddress` review exact paid-order snapshots. `marketplace-i18n` is the typed
EN/DE/FR/ES vocabulary. Styles are scoped to `.market-studio` and use seLLM tokens.

Tests: frontend marketplace interaction tests and app-owned HTTP integration suite.
See [setup and boundaries](../../../../docs/marketplaces.md).

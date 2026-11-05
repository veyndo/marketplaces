/** Server-side integration: gateway credentials never enter storefront JavaScript. */
import { createClient } from '@veyndo/sdk';
const client = createClient({
  baseUrl: process.env.MARKETPLACE_URL,
  token: process.env.MARKETPLACE_TOKEN,
  shop: process.env.MARKETPLACE_TENANT,
});
await client.initialize();
console.log(await client.tool('prepare', { listingId: process.env.MARKETPLACE_LISTING }));

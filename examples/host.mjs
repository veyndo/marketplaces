/** Server component calling an independent commerce host through the shared versioned SDK. */
import { createHostClient } from '@veyndo/sdk/host';
const host = createHostClient({ baseUrl: process.env.MARKETPLACE_HOST_URL,
  shop: process.env.MARKETPLACE_TENANT, appId:'marketplaces',
  token: process.env.MARKETPLACE_HOST_TOKEN, hostPath: '/commerce/v1/' });
const channel = await host.call('channel', { channelId:'de' });
if (!channel.data.locales.includes('it-IT')) throw new Error('Host channel unavailable');
console.log('Independent host SDK channel read completed');

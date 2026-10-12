/** Native app records and provider policy metadata. IDs never change tenant scope. */
export type Row = { id: string; revision: number; [key: string]: any };
export type Job = Row & {
  status: string;
  progress: number;
  message: Record<string, string>;
  result?: { nextCursor?: string; connectionId?: string; after?: string };
};
export const policyFields = {
  amazon: ["productType"],
  ebay: [
    "categoryId",
    "merchantLocationKey",
    "paymentPolicyId",
    "returnPolicyId",
    "fulfillmentPolicyId",
  ],
  kaufland: ["ean", "vatIndicator", "shippingGroupId"],
} as const;
export const fields = (row: Row) =>
  Object.fromEntries(
    Object.entries(row)
      .filter(([k]) => !["id", "revision"].includes(k))
      .map(([k, v]) => [
        k,
        ["title", "description"].includes(k) && v && Object.keys(v).length === 0
          ? null
          : v,
      ]),
  );
export const freshConnection = (provider = "amazon"): Row => ({
  id: crypto.randomUUID(),
  revision: 0,
  name: {},
  channel_id: "",
  provider,
  account: provider,
  destination:
    provider === "ebay"
      ? "EBAY_DE"
      : provider === "kaufland"
        ? "de"
        : "A1PA6795UKMFR9",
  locale: "de-DE",
  currency: "EUR",
  active: true,
  policy: {},
});
export const freshOffer = (connection?: Row): Row => ({
  id: crypto.randomUUID(),
  revision: 0,
  connection_id: connection?.id ?? "",
  product_id: "",
  listing_key: "",
  sku: "",
  title: {},
  description: {},
  images: [],
  attributes: {},
  mapping: {},
  provider_settings: {},
  price: null,
  state: "draft",
  snapshot: null,
});
export const actionPath = (name: string) =>
  `/api/apps/marketplaces/actions/${name}`;

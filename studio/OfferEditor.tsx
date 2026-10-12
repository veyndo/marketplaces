/** Channel-specific offer overrides and explicit data-only attribute mappings. No copied catalog or provider secrets. */
import { useEffect, useState } from "react";
import { useLocale } from "../../../../frontend/src/shared/i18n/i18n";
import { contentText } from "../../../../frontend/src/shared/i18n/content-language";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import LocalizedField from "../../../../frontend/src/shared/i18n/LocalizedField";
import { ContentLanguage } from "../../../../frontend/src/shared/i18n/ContentLanguage";
import ContentLanguagePicker from "../../../../frontend/src/shared/i18n/ContentLanguagePicker";
import { type Row, policyFields } from "./marketplace-model";
import { useMarketText } from "./marketplace-i18n";
export default function OfferEditor({
  initial,
  mainLocale,
  connections,
  request,
  busy,
  save,
  cancel,
}: {
  initial: Row;
  mainLocale: string;
  connections: Row[];
  request: RequestFn;
  busy: boolean;
  save: (r: Row) => void;
  cancel: () => void;
}) {
  const t = useMarketText(),
    { locale } = useLocale(),
    [row, setRow] = useState(initial),
    [products, setProducts] = useState<Row[]>([]),
    [query, setQuery] = useState(""),
    [attrs, setAttrs] = useState(JSON.stringify(row.attributes ?? {}, null, 2)),
    [mapping, setMapping] = useState(
      JSON.stringify(row.mapping ?? {}, null, 2),
    ),
    [error, setError] = useState(""),
    [scales, setScales] = useState<Record<string, number>>({});
  useEffect(() => {
    void request("/store-api/currencies")
      .then((v) =>
        setScales(
          Object.fromEntries(
            v.catalogue.map((c: { code: string; scale: number }) => [
              c.code,
              c.scale,
            ]),
          ),
        ),
      )
      .catch((e) => setError(e.message));
  }, [request]);
  const patch = (key: string, value: unknown) =>
    setRow({ ...row, [key]: value });
  useEffect(() => {
    let active = true;
    const timer = setTimeout(() => {
      void request(
        `/api/merchant/products?limit=30&search=${encodeURIComponent(query)}`,
      )
        .then((v) => {
          if (active) setProducts(v.elements);
        })
        .catch((e) => {
          if (active) setError(e.message);
        });
    }, 200);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [request, query]);
  return (
    <ContentLanguage
      locales={Array.from(
        new Set([mainLocale, ...connections.map((c) => c.locale)]),
      )}
      mainLocale={mainLocale}
    >
      <form
        className="market-editor"
        onSubmit={(e) => {
          e.preventDefault();
          try {
            save({
              ...row,
              attributes: JSON.parse(attrs),
              mapping: JSON.parse(mapping),
              listing_key: `${row.connection_id}:${row.sku}`,
              state: row.state === "uncertain" ? "uncertain" : "draft",
            });
          } catch (e) {
            setError((e as Error).message);
          }
        }}
      >
        <header>
          <h3>{t("offers")}</h3>
          <ContentLanguagePicker />
        </header>
        <fieldset disabled={busy}>
          <div className="market-grid">
            <label>
              {t("connections")}
              <select
                required
                value={row.connection_id}
                onChange={(e) => patch("connection_id", e.target.value)}
              >
                <option value="">—</option>
                {connections.map((c) => (
                  <option key={c.id} value={c.id}>
                    {contentText(c.name, locale, mainLocale) || c.id}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("product")}
              <input
                aria-label={t("product")}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              <select
                required
                value={row.product_id}
                onChange={(e) => {
                  const p = products.find((p) => p.id === e.target.value);
                  setRow({
                    ...row,
                    product_id: e.target.value,
                    sku: row.sku || p?.productNumber || p?.id || "",
                  });
                }}
              >
                <option value="">—</option>
                {row.product_id &&
                  !products.some((p) => p.id === row.product_id) && (
                    <option value={row.product_id}>{row.product_id}</option>
                  )}
                {products.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("sku")}
              <input
                required
                value={row.sku}
                onChange={(e) => patch("sku", e.target.value)}
              />
            </label>
          </div>
          <p>{t("inherit")}</p>
          <LocalizedField
            label={t("title")}
            value={row.title ?? {}}
            onChange={(v) => patch("title", v)}
          />
          <LocalizedField
            label={t("description")}
            multiline
            value={row.description ?? {}}
            onChange={(v) => patch("description", v)}
          />
          <label>
            {t("images")}
            <textarea
              value={(row.images ?? []).join("\n")}
              onChange={(e) =>
                patch(
                  "images",
                  e.target.value
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean),
                )
              }
            />
          </label>
          <div className="market-grid">
            <label>
              {t("attributes")}
              <textarea
                value={attrs}
                onChange={(e) => setAttrs(e.target.value)}
              />
            </label>
            <label>
              {t("mapping")}
              <textarea
                value={mapping}
                onChange={(e) => setMapping(e.target.value)}
              />
            </label>
          </div>
          <h4>{t("policy")}</h4>
          <div className="market-grid">
            {policyFields[
              connections.find((c) => c.id === row.connection_id)
                ?.provider as keyof typeof policyFields
            ]?.map((k) => (
              <label key={k}>
                {t(k)}
                <input
                  placeholder={
                    connections.find((c) => c.id === row.connection_id)
                      ?.policy?.[k] ?? ""
                  }
                  value={row.provider_settings?.[k] ?? ""}
                  onChange={(e) => {
                    const next = { ...row.provider_settings };
                    if (e.target.value) next[k] = e.target.value;
                    else delete next[k];
                    patch("provider_settings", next);
                  }}
                />
              </label>
            ))}
          </div>
          <label>
            {t("price")}
            <input
              type="number"
              min={1}
              step={1}
              disabled={
                scales[
                  connections.find((c) => c.id === row.connection_id)?.currency
                ] === undefined
              }
              value={row.price?.minor ?? ""}
              onChange={(e) => {
                const c = connections.find((c) => c.id === row.connection_id);
                patch(
                  "price",
                  e.target.value
                    ? {
                        minor: Number(e.target.value),
                        currency: {
                          code: c?.currency ?? "EUR",
                          scale: scales[c?.currency ?? "EUR"],
                        },
                      }
                    : null,
                );
              }}
            />
          </label>
        </fieldset>
        {error && <p role="alert">{error}</p>}
        <footer>
          <button
            type="button"
            className="studio-secondary"
            disabled={busy}
            onClick={cancel}
          >
            {t("cancel")}
          </button>
          <button
            className="studio-primary"
            disabled={busy || !row.product_id || !row.sku}
          >
            {t("save")}
          </button>
        </footer>
      </form>
    </ContentLanguage>
  );
}

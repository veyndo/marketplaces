/** Connection and offer cards with explicit prepare/submit controls and channel-specific preview. */
import { contentText } from "../../../../frontend/src/shared/i18n/content-language";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import {
  actionPath,
  freshConnection,
  freshOffer,
  type Row,
} from "./marketplace-model";
import { useMarketText, type MarketText } from "./marketplace-i18n";
type Editor = { entity: "connections" | "listings"; row: Row };
export default function MarketplaceOffers({
  kind,
  connections,
  offers,
  locale,
  mainLocale,
  manage,
  busy,
  setEditor,
  setConfirm,
  run,
  sync,
  request,
  preview,
  setPreview,
}: {
  kind: string;
  connections: Row[];
  offers: Row[];
  locale: string;
  mainLocale: string;
  manage: boolean;
  busy: boolean;
  setEditor: (e: Editor) => void;
  setConfirm: (r: Row) => void;
  run: (f: () => Promise<unknown>) => Promise<void>;
  sync: (c: Row, k: "orders" | "listing") => Promise<unknown>;
  request: RequestFn;
  preview: any;
  setPreview: (v: any) => void;
}) {
  const t = useMarketText();
  const status = (s: string) => t(s as Parameters<MarketText>[0]);
  return (
    <>
      {kind === "connections" && (
        <>
          <button
            className="studio-primary"
            disabled={!manage || busy}
            onClick={() =>
              setEditor({ entity: "connections", row: freshConnection() })
            }
          >
            {t("newConnection")}
          </button>
          <div className="market-cards">
            {connections.map((c) => (
              <article key={c.id} data-provider={c.provider}>
                <span className="market-provider">
                  {t(c.provider as Parameters<MarketText>[0])}
                </span>
                <h3>{contentText(c.name, locale, mainLocale) || c.id}</h3>
                <p>
                  {c.destination} · {c.locale} · {c.currency}
                </p>
                <span className="market-state">
                  {c.active ? t("active") : t("cancelled")}
                </span>
                <footer>
                  <button
                    className="studio-secondary"
                    disabled={!manage || busy}
                    onClick={() => setEditor({ entity: "connections", row: c })}
                  >
                    {t("edit")}
                  </button>
                  <button
                    className="studio-primary"
                    disabled={!manage || busy || !c.active}
                    onClick={() => void run(() => sync(c, "orders"))}
                  >
                    {t("retrieve")}
                  </button>
                </footer>
              </article>
            ))}
          </div>
          <p>{t("serverHint")}</p>
        </>
      )}
      {kind === "offers" && (
        <>
          <button
            className="studio-primary"
            disabled={!manage || busy || !connections.length}
            onClick={() =>
              setEditor({
                entity: "listings",
                row: freshOffer(connections[0]),
              })
            }
          >
            {t("newOffer")}
          </button>
          <div className="market-cards">
            {offers.map((o) => {
              const connection = connections.find(
                (c) => c.id === o.connection_id,
              );
              return (
                <article key={o.id} data-provider={connection?.provider}>
                  {connection && (
                    <span className="market-provider">
                      {t(connection.provider as Parameters<MarketText>[0])} ·{" "}
                      {contentText(connection.name, locale, mainLocale) ||
                        connection.id}
                    </span>
                  )}
                  <h3>{contentText(o.title, locale, mainLocale) || o.sku}</h3>
                  <p>
                    {o.sku} · {o.product_id}
                  </p>
                  <span className="market-state" data-state={o.state}>
                    {status(o.state)}
                  </span>
                  <footer>
                    <button
                      className="studio-secondary"
                      disabled={!manage || busy}
                      onClick={() => setEditor({ entity: "listings", row: o })}
                    >
                      {t("edit")}
                    </button>
                    <button
                      className="studio-secondary"
                      disabled={!manage || busy || o.state === "uncertain"}
                      onClick={() =>
                        void run(async () =>
                          setPreview(
                            await request(actionPath("prepare"), {
                              listingId: o.id,
                            }),
                          ),
                        )
                      }
                    >
                      {t("prepare")}
                    </button>
                    <button
                      className="studio-primary"
                      disabled={
                        !manage ||
                        busy ||
                        !["ready", "submitted"].includes(o.state)
                      }
                      onClick={() => setConfirm(o)}
                    >
                      {t("publish")}
                    </button>
                  </footer>
                </article>
              );
            })}
          </div>
          {preview && (
            <section className="market-preview">
              <h3>{t("preview")}</h3>
              <h4>{preview.mapped.title}</h4>
              <p>{preview.mapped.description}</p>
              <div className="market-images">
                {preview.mapped.images.map((url: string) => (
                  <img
                    key={url}
                    src={url}
                    alt={preview.mapped.title}
                    loading="lazy"
                  />
                ))}
              </div>
              {preview.missing.length > 0 && (
                <p role="alert">
                  {t("missing")}: {preview.missing.join(", ")}
                </p>
              )}
              <details>
                <summary>{t("mapping")}</summary>
                <pre>{JSON.stringify(preview.mapped, null, 2)}</pre>
              </details>
            </section>
          )}
        </>
      )}
    </>
  );
}

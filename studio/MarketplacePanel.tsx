/** App-owned marketplace workspace uses generated entity actions, native job queue and commerce callbacks. */
import { useEffect, useRef, useState } from "react";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import { useLocale } from "../../../../frontend/src/shared/i18n/i18n";
import { contentText } from "../../../../frontend/src/shared/i18n/content-language";
import Icon from "../../../../frontend/src/shared/ui/Icon";
import ConfirmDialog from "../../../../frontend/src/shared/ui/ConfirmDialog";
import MarketplaceOffers from "./MarketplaceOffers";
import ConnectionEditor from "./ConnectionEditor";
import OfferEditor from "./OfferEditor";
import OrderReview from "./OrderReview";
import { actionPath, fields, type Row, type Job } from "./marketplace-model";
import { useMarketText, type MarketText } from "./marketplace-i18n";
import "./marketplaces.css";
export default function MarketplacePanel({
  request,
  manage,
  mainLocale,
}: {
  request: RequestFn;
  manage: boolean;
  mainLocale: string;
}) {
  const t = useMarketText(),
    { locale } = useLocale(),
    [tab, setTab] = useState<"connections" | "offers" | "inbox" | "jobs">(
      "connections",
    ),
    [connections, setConnections] = useState<Row[]>([]),
    [offers, setOffers] = useState<Row[]>([]),
    [inbox, setInbox] = useState<Row[]>([]),
    [jobs, setJobs] = useState<Job[]>([]),
    [editor, setEditor] = useState<{
      entity: "connections" | "listings";
      row: Row;
    }>(),
    [review, setReview] = useState<Row>(),
    [preview, setPreview] = useState<any>(),
    [confirm, setConfirm] = useState<Row>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const orderLink = (id: string) => {
    const url = new URL(window.location.href);
    url.searchParams.set("studio", "orders");
    url.searchParams.set("order", id);
    url.hash = "merchant";
    return url.href;
  };
  const lock = useRef(false);
  const refresh = async () => {
    const values = await Promise.all([
      request(actionPath("list_connections"), {}),
      request(actionPath("list_listings"), {}),
      request("/api/apps/marketplaces/jobs"),
    ]);
    setConnections(values[0].elements);
    setOffers(values[1].elements);
    setJobs(values[2].jobs);
  };
  const run = async (fn: () => Promise<unknown>) => {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    setError("");
    try {
      await fn();
      await refresh();
    } catch (e) {
      setError((e as Error).message);
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  useEffect(() => {
    let active = true;
    void Promise.all([
      request(actionPath("list_connections"), {}),
      request(actionPath("list_listings"), {}),
      request("/api/apps/marketplaces/jobs"),
    ])
      .then((v) => {
        if (active) {
          setConnections(v[0].elements);
          setOffers(v[1].elements);
          setJobs(v[2].jobs);
        }
      })
      .catch((e) => {
        if (active) setError(e.message);
      });
    return () => {
      active = false;
    };
  }, [request]);
  useEffect(() => {
    if (
      !jobs.some((j) =>
        ["queued", "running", "cancel_requested"].includes(j.status),
      )
    )
      return;
    const timer = setInterval(() => {
      void refresh().catch((e) => setError(e.message));
    }, 3000);
    return () => clearInterval(timer);
  }, [jobs, request]);
  const sync = async (
    c: Row,
    kind: "listing" | "orders",
    listing?: Row,
    cursor?: string,
    after?: string,
  ) =>
    request(
      actionPath("sync"),
      {
        connectionId: c.id,
        kind,
        ...(listing ? { listingId: listing.id } : {}),
        ...(cursor
          ? {
              cursor,
              after,
            }
          : {}),
      },
      "POST",
      { idempotencyKey: crypto.randomUUID() },
    );
  const status = (s: string) => t(s as Parameters<MarketText>[0]) || s;
  return (
    <section className="market-studio">
      <header className="market-hero">
        <div>
          <span className="market-eyebrow">{t("connections")}</span>
          <h2>{t("heading")}</h2>
          <p>{t("intro")}</p>
        </div>
        <Icon name="globe" />
      </header>
      <div className="market-stats">
        <div>
          <strong>{connections.length}</strong>
          {t("connections")}
        </div>
        <div>
          <strong>{offers.length}</strong>
          {t("offers")}
        </div>
        <div>
          <strong>{jobs.filter((j) => j.status === "succeeded").length}</strong>
          {t("succeeded")}
        </div>
      </div>
      <nav aria-label={t("heading")}>
        {(["connections", "offers", "inbox", "jobs"] as const).map((k) => (
          <button
            className={tab === k ? "studio-primary" : "studio-secondary"}
            key={k}
            aria-pressed={tab === k}
            onClick={() => {
              setTab(k);
              setEditor(undefined);
              setReview(undefined);
              if (k === "inbox")
                void run(async () =>
                  setInbox(
                    (await request(actionPath("list_inbox"), {})).elements,
                  ),
                );
            }}
          >
            <Icon
              name={
                k === "connections"
                  ? "globe"
                  : k === "offers"
                    ? "box"
                    : k === "inbox"
                      ? "box"
                      : "pulse"
              }
            />
            {t(k)}
          </button>
        ))}
        <button
          className="studio-secondary"
          disabled={busy}
          onClick={() => void run(refresh)}
        >
          <Icon name="refresh" />
          {t("refresh")}
        </button>
      </nav>
      {error && <p role="alert">{error}</p>}
      {editor?.entity === "connections" ? (
        <ConnectionEditor
          key={editor.row.id}
          initial={editor.row}
          mainLocale={mainLocale}
          request={request}
          busy={busy}
          cancel={() => setEditor(undefined)}
          save={(row) =>
            void run(async () => {
              await request(actionPath("save_connections"), {
                id: row.id,
                revision: row.revision,
                fields: fields(row),
              });
              setEditor(undefined);
            })
          }
        />
      ) : editor?.entity === "listings" ? (
        <OfferEditor
          key={editor.row.id}
          initial={editor.row}
          connections={connections}
          mainLocale={mainLocale}
          request={request}
          busy={busy}
          cancel={() => setEditor(undefined)}
          save={(row) =>
            void run(async () => {
              await request(actionPath("save_listings"), {
                id: row.id,
                revision: row.revision,
                fields: fields(row),
              });
              setEditor(undefined);
            })
          }
        />
      ) : review ? (
        <OrderReview
          row={review}
          connections={connections}
          offers={offers}
          request={request}
          busy={busy}
          cancel={() => setReview(undefined)}
          done={() =>
            void run(async () => {
              setReview(undefined);
              setInbox((await request(actionPath("list_inbox"), {})).elements);
            })
          }
        />
      ) : (
        <>
          <MarketplaceOffers
            kind={tab}
            connections={connections}
            offers={offers}
            locale={locale}
            mainLocale={mainLocale}
            manage={manage}
            busy={busy}
            setEditor={setEditor}
            setConfirm={setConfirm}
            run={run}
            sync={sync}
            request={request}
            preview={preview}
            setPreview={setPreview}
          />
          {tab === "inbox" && (
            <div className="market-cards">
              {inbox.map((row) => (
                <article key={row.id}>
                  <h3>{row.external_id}</h3>
                  <span className="market-state">{status(row.state)}</span>
                  <footer>
                    {row.order_id ? (
                      <a
                        className="studio-secondary"
                        href={orderLink(row.order_id)}
                      >
                        {t("openOrder")}
                      </a>
                    ) : (
                      <button
                        className="studio-primary"
                        disabled={!manage || busy}
                        onClick={() => setReview(row)}
                      >
                        {t("review")}
                      </button>
                    )}
                  </footer>
                </article>
              ))}
            </div>
          )}
          {tab === "jobs" && (
            <div className="market-cards">
              {jobs.map((j) => (
                <article key={j.id}>
                  <h3>{status(j.status)}</h3>
                  <progress value={j.progress} max={100} />
                  <p>{contentText(j.message, locale, "en")}</p>
                  {j.result?.nextCursor && (
                    <button
                      className="studio-secondary"
                      disabled={busy}
                      onClick={() =>
                        void run(async () => {
                          const c = connections.find(
                            (c) => c.id === j.result?.connectionId,
                          );
                          if (c)
                            await sync(
                              c,
                              "orders",
                              undefined,
                              j.result?.nextCursor,
                              j.result?.after,
                            );
                        })
                      }
                    >
                      {t("next")}
                    </button>
                  )}
                  <details>
                    <summary>{t("result")}</summary>
                    <pre>{JSON.stringify(j.result ?? {}, null, 2)}</pre>
                  </details>
                </article>
              ))}
            </div>
          )}
          {!(
            tab === "connections"
              ? connections
              : tab === "offers"
                ? offers
                : tab === "inbox"
                  ? inbox
                  : jobs
          ).length && <p className="market-empty">{t("empty")}</p>}
        </>
      )}
      {confirm && (
        <ConfirmDialog
          title={t("publish")}
          confirmLabel={t("publish")}
          disabled={busy}
          onCancel={() => setConfirm(undefined)}
          onConfirm={() =>
            void run(async () => {
              const c = connections.find((c) => c.id === confirm.connection_id);
              if (c) await sync(c, "listing", confirm);
              setConfirm(undefined);
            })
          }
        >
          <p>{t("confirmPublish")}</p>
        </ConfirmDialog>
      )}
    </section>
  );
}

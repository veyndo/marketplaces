/** Source-derived review uses visible addresses, SKU matches and exact amounts; no invented products or taxes. */
import { useEffect, useState } from "react";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import { type Row, actionPath } from "./marketplace-model";
import { useMarketText } from "./marketplace-i18n";
import OrderAddress from "./OrderAddress";
export default function OrderReview({
  row,
  offers,
  request,
  busy,
  cancel,
  done,
}: {
  row: Row;
  connections: Row[];
  offers: Row[];
  request: RequestFn;
  busy: boolean;
  cancel: () => void;
  done: () => void;
}) {
  const t = useMarketText(),
    [approved, setApproved] = useState(false),
    [saving, setSaving] = useState(false),
    [error, setError] = useState(""),
    [snapshot, setSnapshot] = useState<any>(),
    [unmapped, setUnmapped] = useState<string[]>([]);
  useEffect(() => {
    let active = true;
    void request(actionPath("review_order"), { inboxId: row.id })
      .then((v) => {
        if (active) {
          setSnapshot(v.snapshot);
          setUnmapped(v.unmappedSkus);
        }
      })
      .catch((e) => {
        if (active) setError(e.message);
      });
    return () => {
      active = false;
    };
  }, [request, row.id]);
  const amount = (value: any, change: (v: any) => void) => (
    <div className="market-grid">
      {(["grossMinor", "taxMinor", "taxRateBasisPoints"] as const).map((k) => (
        <label key={k}>
          {t(k)}
          <input
            type="number"
            required
            min={0}
            step={1}
            value={value[k] ?? ""}
            onChange={(e) =>
              change({
                ...value,
                [k]: e.target.value === "" ? null : Number(e.target.value),
              })
            }
          />
        </label>
      ))}
    </div>
  );
  return (
    <form
      className="market-editor"
      onSubmit={async (e) => {
        e.preventDefault();
        if (saving || !snapshot) return;
        setSaving(true);
        setError("");
        try {
          await request(actionPath("import_order"), {
            inboxId: row.id,
            snapshot,
            approve: approved,
          });
          done();
        } catch (e) {
          setError((e as Error).message);
        } finally {
          setSaving(false);
        }
      }}
    >
      <h3>
        {t("review")} · {row.external_id}
      </h3>
      <p>{t("reviewHint")}</p>
      {unmapped.length > 0 && (
        <p role="alert">
          {t("unmapped")}: {unmapped.join(", ")}
        </p>
      )}
      {snapshot && (
        <fieldset disabled={busy || saving}>
          <label>
            {t("email")}
            <input
              type="email"
              required
              value={snapshot.email}
              onChange={(e) =>
                setSnapshot({ ...snapshot, email: e.target.value })
              }
            />
          </label>
          <OrderAddress
            title={t("billingAddress")}
            value={snapshot.billingAddress}
            change={(v) => setSnapshot({ ...snapshot, billingAddress: v })}
          />
          <OrderAddress
            title={t("shippingAddress")}
            value={snapshot.shippingAddress}
            change={(v) => setSnapshot({ ...snapshot, shippingAddress: v })}
          />
          {snapshot.lines.map((l: any, i: number) => (
            <section key={i}>
              <div className="market-grid">
                <label>
                  {t("product")}
                  <select
                    required
                    value={l.productId}
                    onChange={(e) =>
                      setSnapshot({
                        ...snapshot,
                        lines: snapshot.lines.map((v: any, n: number) =>
                          n === i ? { ...v, productId: e.target.value } : v,
                        ),
                      })
                    }
                  >
                    <option value="">—</option>
                    {offers
                      .filter((o) => o.connection_id === row.connection_id)
                      .map((o) => (
                        <option key={o.id} value={o.product_id}>
                          {o.sku} · {o.product_id}
                        </option>
                      ))}
                  </select>
                </label>
                <label>
                  {t("quantity")}
                  <input
                    required
                    type="number"
                    min={1}
                    max={10000}
                    step={1}
                    value={l.quantity ?? ""}
                    onChange={(e) =>
                      setSnapshot({
                        ...snapshot,
                        lines: snapshot.lines.map((v: any, n: number) =>
                          n === i
                            ? { ...v, quantity: Number(e.target.value) }
                            : v,
                        ),
                      })
                    }
                  />
                </label>
              </div>
              {amount(l.amount, (v) =>
                setSnapshot({
                  ...snapshot,
                  lines: snapshot.lines.map((l: any, n: number) =>
                    n === i ? { ...l, amount: v } : l,
                  ),
                }),
              )}
            </section>
          ))}
          <h4>{t("shipping")}</h4>
          {amount(snapshot.shipping, (v) =>
            setSnapshot({ ...snapshot, shipping: v }),
          )}
          <label>
            {t("totalMinor")} · {snapshot.currency}
            <input
              required
              type="number"
              min={1}
              step={1}
              value={snapshot.totalMinor ?? ""}
              onChange={(e) =>
                setSnapshot({ ...snapshot, totalMinor: Number(e.target.value) })
              }
            />
          </label>
          <label className="market-check">
            <input
              type="checkbox"
              checked={snapshot.paid}
              onChange={(e) =>
                setSnapshot({ ...snapshot, paid: e.target.checked })
              }
            />
            {t("paid")}
          </label>
          <details>
            <summary>{t("source")}</summary>
            <pre>{JSON.stringify(row.source, null, 2)}</pre>
          </details>
          <label className="market-check">
            <input
              type="checkbox"
              checked={approved}
              onChange={(e) => setApproved(e.target.checked)}
            />
            {t("approved")}
          </label>
        </fieldset>
      )}
      {error && <p role="alert">{error}</p>}
      <footer>
        <button
          className="studio-secondary"
          type="button"
          disabled={saving}
          onClick={cancel}
        >
          {t("cancel")}
        </button>
        <button
          className="studio-primary"
          disabled={
            !snapshot ||
            !approved ||
            !snapshot.paid ||
            !snapshot.lines.length ||
            busy ||
            saving
          }
        >
          {t("import")}
        </button>
      </footer>
    </form>
  );
}

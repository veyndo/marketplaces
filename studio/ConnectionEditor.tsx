/** Guided seller connection editor reuses native channel and record revisions. Secrets remain server-only. */
import { useEffect, useState } from "react";
import { useLocale } from "../../../../frontend/src/shared/i18n/i18n";
import { contentText } from "../../../../frontend/src/shared/i18n/content-language";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import LocalizedField from "../../../../frontend/src/shared/i18n/LocalizedField";
import { ContentLanguage } from "../../../../frontend/src/shared/i18n/ContentLanguage";
import ContentLanguagePicker from "../../../../frontend/src/shared/i18n/ContentLanguagePicker";
import { type Row, policyFields } from "./marketplace-model";
import { useMarketText } from "./marketplace-i18n";
export default function ConnectionEditor({
  initial,
  mainLocale,
  request,
  busy,
  save,
  cancel,
}: {
  initial: Row;
  mainLocale: string;
  request: RequestFn;
  busy: boolean;
  save: (r: Row) => void;
  cancel: () => void;
}) {
  const t = useMarketText(),
    { locale } = useLocale(),
    [row, setRow] = useState(initial),
    [channels, setChannels] = useState<Row[]>([]);
  useEffect(() => {
    let active = true;
    void request("/api/automation")
      .then((v) => {
        if (active)
          setChannels(
            v.channels.filter((c: Row) => c.data.kind === "marketplace"),
          );
      })
      .catch(() => {});
    return () => {
      active = false;
    };
  }, [request]);
  const patch = (key: string, value: unknown) =>
    setRow({ ...row, [key]: value });
  return (
    <ContentLanguage
      locales={Array.from(
        new Set([mainLocale, ...channels.flatMap((c) => c.data.locales)]),
      )}
      mainLocale={mainLocale}
    >
      <form
        className="market-editor"
        onSubmit={(e) => {
          e.preventDefault();
          save(row);
        }}
      >
        <header>
          <h3>{t("newConnection")}</h3>
          <ContentLanguagePicker />
        </header>
        <fieldset disabled={busy}>
          <LocalizedField
            label={t("name")}
            value={row.name}
            onChange={(v) => patch("name", v)}
            required
          />
          <div className="market-grid">
            <label>
              {t("channel")}
              <select
                required
                value={row.channel_id}
                onChange={(e) => patch("channel_id", e.target.value)}
              >
                <option value="">—</option>
                {channels.map((c) => (
                  <option value={c.id} key={c.id}>
                    {contentText(c.data.name, locale, mainLocale) || c.id}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("provider")}
              <select
                value={row.provider}
                onChange={(e) =>
                  setRow({
                    ...row,
                    provider: e.target.value,
                    account: e.target.value,
                    policy: {},
                  })
                }
              >
                {Object.keys(policyFields).map((p) => (
                  <option key={p} value={p}>
                    {t(p as keyof typeof policyFields)}
                  </option>
                ))}
              </select>
            </label>
            {(["account", "destination", "locale", "currency"] as const).map(
              (k) => (
                <label key={k}>
                  {t(k)}
                  <input
                    required
                    value={row[k]}
                    maxLength={100}
                    onChange={(e) => patch(k, e.target.value)}
                  />
                </label>
              ),
            )}
          </div>
          <p>{t("serverHint")}</p>
          <h4>{t("policy")}</h4>
          <div className="market-grid">
            {policyFields[row.provider as keyof typeof policyFields]?.map(
              (k) => (
                <label key={k}>
                  {t(k)}
                  <input
                    value={row.policy?.[k] ?? ""}
                    onChange={(e) =>
                      patch("policy", { ...row.policy, [k]: e.target.value })
                    }
                  />
                </label>
              ),
            )}
          </div>
          <label className="market-check">
            <input
              type="checkbox"
              checked={row.active}
              onChange={(e) => patch("active", e.target.checked)}
            />
            {t("active")}
          </label>
        </fieldset>
        <footer>
          <button
            type="button"
            className="studio-secondary"
            disabled={busy}
            onClick={cancel}
          >
            {t("cancel")}
          </button>
          <button className="studio-primary" disabled={busy || !row.channel_id}>
            {t("save")}
          </button>
        </footer>
      </form>
    </ContentLanguage>
  );
}

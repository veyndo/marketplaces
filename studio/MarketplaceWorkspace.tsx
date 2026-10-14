/** Channel-scoped entry to the existing app-owned seller UI; installation uses native reviewed consent. */
import { useEffect, useState } from "react";
import type { RequestFn } from "../../../../frontend/src/shared/api/types";
import AppConsent from "../../../../frontend/src/admin/apps/AppConsent";
import Icon from "../../../../frontend/src/shared/ui/Icon";
import { useChannelText } from "../../../../frontend/src/admin/channels/channel-i18n";
import MarketplacePanel from "./MarketplacePanel";
import { useMarketplaceSetupText } from "./marketplace-setup-i18n";
import "./marketplaces.css";
export default function MarketplaceWorkspace({
  request,
  channel,
  name,
  mainLocale,
  rights,
  onBack,
}: {
  request: RequestFn;
  channel: string;
  name: string;
  mainLocale: string;
  rights: string[];
  onBack: () => void;
}) {
  const t = useMarketplaceSetupText(),
    c = useChannelText();
  const [app, setApp] = useState<{ active: boolean; revision: number }>(),
    [loading, setLoading] = useState(true),
    [error, setError] = useState(""),
    [install, setInstall] = useState(false),
    [busy, setBusy] = useState(false),
    [refresh, setRefresh] = useState(0);
  const manage = rights.includes("apps.manage"),
    read = rights.includes("apps.read");
  useEffect(() => {
    let active = true;
    setLoading(true);
    setError("");
    setApp(undefined);
    if (!read) {
      setLoading(false);
      return;
    }
    void request("/api/apps")
      .then((v) => {
        if (active)
          setApp(
            v.packages.find((p: { id: string }) => p.id === "marketplaces"),
          );
      })
      .catch((e) => {
        if (active) setError(e.message);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [request, refresh, read]);
  const changed = () => {
    dispatchEvent(new Event("commerce.apps.changed"));
    setRefresh((v) => v + 1);
  };
  return (
    <div className="studio-page marketplace-workspace">
      <header className="page-intro">
        <button className="studio-secondary" onClick={onBack}>
          ← {c("back")}
        </button>
        <span className="kicker">{name}</span>
        <h1>{t("studio")}</h1>
        <p>{t("intro")}</p>
      </header>
      {error && <p role="alert">{error}</p>}
      {loading ? (
        <p role="status">{t("loading")}</p>
      ) : !read ? (
        <p role="alert">{t("rights")}</p>
      ) : app?.active ? (
        <MarketplacePanel
          request={request}
          manage={manage}
          mainLocale={mainLocale}
          channel={channel}
        />
      ) : (
        <section className="studio-card market-launch-card">
          <Icon name="layers" size={32} />
          <h2>{t("connect")}</h2>
          <p>{t("setup")}</p>
          <ol>
            <li>{t("step1")}</li>
            <li>{t("step2")}</li>
            <li>{t("step3")}</li>
          </ol>
          {!manage && <p>{t("rights")}</p>}
          <button
            className="studio-primary"
            disabled={!manage || busy || !!error}
            onClick={async () => {
              if (!app) {
                setInstall(true);
                return;
              }
              setBusy(true);
              setError("");
              try {
                await request(
                  "/api/apps/marketplaces",
                  { active: true, revision: app.revision },
                  "PUT",
                );
                changed();
              } catch (e) {
                setError((e as Error).message);
              } finally {
                setBusy(false);
              }
            }}
          >
            {t(app ? "activate" : "install")}
          </button>
        </section>
      )}
      {install && (
        <AppConsent
          app="marketplaces"
          request={request}
          onCancel={() => setInstall(false)}
          onInstall={async (consent) => {
            await request("/api/apps", {
              builtIn: "marketplaces",
              approve: true,
              digest: consent.digest,
              permissions: consent.permissions,
            });
            setInstall(false);
            changed();
          }}
        />
      )}
    </div>
  );
}

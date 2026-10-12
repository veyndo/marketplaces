/** Marketplace review addresses remain source snapshots; no separate customer directory. */
import { useMarketText } from "./marketplace-i18n";
export default function OrderAddress({
  title,
  value,
  change,
}: {
  title: string;
  value: Record<string, string>;
  change: (v: Record<string, string>) => void;
}) {
  const t = useMarketText();
  return (
    <section>
      <h4>{title}</h4>
      <div className="market-grid">
        {(["name", "street", "postalCode", "city", "country"] as const).map(
          (k) => (
            <label key={k}>
              {t(k)}
              <input
                required
                value={value[k] ?? ""}
                maxLength={k === "country" ? 2 : 200}
                onChange={(e) =>
                  change({
                    ...value,
                    [k]:
                      k === "country"
                        ? e.target.value.toUpperCase()
                        : e.target.value,
                  })
                }
              />
            </label>
          ),
        )}
      </div>
    </section>
  );
}

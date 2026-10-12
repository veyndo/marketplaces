//! Explainable automatic field suggestions and per-channel overrides; mapping is reviewed data, never executable code.
use super::*;
#[cfg(test)]
pub fn text(v: &Value, locale: &str, main: &str) -> String {
    [
        locale,
        locale.split('-').next().unwrap_or(locale),
        main,
        main.split('-').next().unwrap_or(main),
    ]
    .iter()
    .find_map(|l| v[*l].as_str().filter(|s| !s.trim().is_empty()))
    .unwrap_or("")
    .to_owned()
}
fn source<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    if path.len() > 200 || path.split('.').count() > 8 {
        return None;
    }
    let mut v = v;
    for p in path.split('.') {
        v = v.get(p)?;
    }
    Some(v)
}
pub fn mapped(product: &Value, listing: &Value, connection: &Value, money: Value) -> Result<Value> {
    let locale = connection["locale"].as_str().ok_or(error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Connection content language required",
    ))?;
    let main = product["mainLocale"].as_str().unwrap_or("en-GB");
    let translations = &product["translations"];
    let native_name = translations[locale]["name"]
        .as_str()
        .or_else(|| translations[locale.split('-').next().unwrap_or(locale)]["name"].as_str())
        .filter(|s| !s.trim().is_empty())
        .or_else(|| translations[main]["name"].as_str())
        .or_else(|| translations[main.split('-').next().unwrap_or(main)]["name"].as_str())
        .unwrap_or("");
    let native_desc = translations[locale]["description"]
        .as_str()
        .or_else(|| {
            translations[locale.split('-').next().unwrap_or(locale)]["description"].as_str()
        })
        .filter(|s| !s.trim().is_empty())
        .or_else(|| translations[main]["description"].as_str())
        .or_else(|| translations[main.split('-').next().unwrap_or(main)]["description"].as_str())
        .unwrap_or("");
    let override_text = |k: &str, fallback: &str| {
        let keys = [
            locale,
            locale.split('-').next().unwrap_or(locale),
            main,
            main.split('-').next().unwrap_or(main),
        ];
        keys.iter()
            .find_map(|key| {
                listing[k][*key]
                    .as_str()
                    .filter(|s| k == "description" || !s.trim().is_empty())
            })
            .unwrap_or(fallback)
            .to_owned()
    };
    let images = listing["images"]
        .as_array()
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| {
            product["commerce"]["media"]
                .as_array()
                .map(|vs| {
                    vs.iter()
                        .filter_map(|v| {
                            v["url"]
                                .as_str()
                                .or_else(|| v["src"].as_str())
                                .map(|s| json!(s))
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
    let scale = money["currency"]["scale"].clone();
    let price = if listing["price"].is_object() {
        listing["price"].clone()
    } else {
        money
    };
    if price["currency"]["scale"] != scale
        || price["currency"]["code"] != connection["currency"]
        || price["minor"]
            .as_i64()
            .is_none_or(|p| p <= 0 || p > 1_000_000_000_000)
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Price currency must match the channel; exact positive amount required",
        ));
    }
    for image in &images {
        let valid = image.as_str().is_some_and(|s| {
            reqwest::Url::parse(s).is_ok_and(|u| {
                u.scheme() == "https"
                    && u.host_str().is_some()
                    && u.username().is_empty()
                    && u.password().is_none()
            })
        });
        if !valid {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Marketplace images require public HTTPS URLs; upload local assets first",
            ));
        }
    }
    if images.len() > 12 {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Maximum 12 marketplace images",
        ));
    }
    let mut attrs = listing["attributes"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    if let Some(mapping) = listing["mapping"].as_object() {
        if mapping.len() > 40 {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Maximum 40 mapped attributes",
            ));
        }
        for (target, path) in mapping {
            let v = path
                .as_str()
                .and_then(|path| source(product, path))
                .ok_or(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Mapped source field unavailable; review field mapping",
                ))?;
            if !attrs.contains_key(target) {
                attrs.insert(target.clone(), v.clone());
            }
        }
    }
    Ok(
        json!({"title":override_text("title",native_name),"description":override_text("description",native_desc),"images":images,"sku":listing["sku"],"attributes":attrs,"money":price,"stock":product["commerce"]["stock"],"locale":locale,"sourceRevision":product["revision"],"suggestions":{"title":"translations → channel language → shop main language","description":"translations → channel language → shop main language","images":"commerce.media","stock":"commerce.stock","price":"native channel currency pricing","attributes":"explicit mapping → source field; overrides take precedence"}}),
    )
}
pub fn offer_connection(connection: &Value, listing: &Value) -> Value {
    let mut c = connection.clone();
    let mut policy = c["policy"].as_object().cloned().unwrap_or_default();
    if let Some(overrides) = listing["provider_settings"].as_object() {
        policy.extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    c["policy"] = json!(policy);
    c
}
pub async fn prepare(a: &App, t: &str, v: &Value) -> Result<Value> {
    let id = v["listingId"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "Listing ID required"))?;
    let row = a.one(t, "listings", id).await?;
    if row["state"] == "uncertain" {
        return Err(error(
            StatusCode::CONFLICT,
            "Reconcile the previous transmission in the seller portal before explicitly resetting this listing",
        ));
    }
    let c = a
        .one(
            t,
            "connections",
            row["connection_id"].as_str().unwrap_or(""),
        )
        .await?;
    connection(a, t, &c).await?;
    let source = a
        .callback(
            t,
            "channel_product",
            json!({"id":row["product_id"],"channelId":c["channel_id"],"currency":c["currency"]}),
        )
        .await?;
    let mapped = mapped(&source["product"], &row, &c, source["money"].clone())?;
    let missing = providers::missing(&offer_connection(&c, &row), &mapped);
    let snapshot = json!({"mapped":mapped,"missing":missing,"publishable":missing.is_empty()});
    let mut fields = fields(&row);
    fields["snapshot"] = snapshot.clone();
    fields["state"] = json!(if missing.is_empty() {
        "ready"
    } else {
        "needs_review"
    });
    a.save(t, "listings", &row, fields).await?;
    Ok(snapshot)
}
pub fn fields(row: &Value) -> Value {
    let mut row = row.as_object().cloned().unwrap_or_default();
    row.remove("id");
    row.remove("revision");
    Value::Object(row)
}
pub async fn connection(a: &App, t: &str, c: &Value) -> Result<()> {
    if c["active"] != true {
        return Err(error(
            StatusCode::CONFLICT,
            "Synchronization is paused for this connection",
        ));
    }
    let tenant = a
        .config
        .tenants
        .get(t)
        .ok_or(error(StatusCode::FORBIDDEN, "Unknown tenant"))?;
    let account = c["account"]
        .as_str()
        .and_then(|id| tenant.accounts.get(id))
        .ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Server credential alias is not configured",
        ))?;
    if account.provider != c["provider"].as_str().unwrap_or("") {
        return Err(error(StatusCode::FORBIDDEN, "Credential provider mismatch"));
    }
    let channel = a
        .callback(t, "channel", json!({"channelId":c["channel_id"]}))
        .await?;
    if !channel["data"]["locales"]
        .as_array()
        .is_some_and(|ls| ls.contains(&c["locale"]))
        || !channel["currencies"]["enabled"]
            .as_array()
            .is_some_and(|ls| ls.contains(&c["currency"]))
    {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Connection language or currency is disabled in sales channel",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn localized_overrides_inherit_and_mapping_is_data_only() {
        assert_eq!(
            text(&json!({"en":"Main","de":null}), "de-DE", "en-GB"),
            "Main"
        );
        assert!(source(&json!({}), "a.b.c.d.e.f.g.h.i").is_none());
        let p = json!({"mainLocale":"en-GB","translations":{"en":{"name":"Main","description":"Details"},"de":{"name":"Deutsch"}},"commerce":{"stock":7,"media":[]}});
        let c = json!({"locale":"de-DE","currency":"EUR"});
        let price = json!({"minor":1099,"currency":{"code":"EUR","scale":2}});
        let r = mapped(
            &p,
            &json!({"sku":"SKU","title":{"de":"Custom"},"mapping":{"count":"commerce.stock"}}),
            &c,
            price.clone(),
        )
        .unwrap();
        assert_eq!(r["title"], "Custom");
        assert_eq!(r["attributes"]["count"], 7);
        assert_eq!(r["description"], "Details");
        assert!(
            mapped(
                &p,
                &json!({"price":{"minor":10,"currency":{"code":"USD","scale":2}}}),
                &c,
                price
            )
            .is_err()
        );
    }
}

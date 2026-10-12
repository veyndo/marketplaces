//! Distinct official listing payloads, provider prerequisites and order pages; more providers extend this app-level seam.
use super::*;
use reqwest::Method;
pub fn missing(c: &Value, p: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for k in ["title", "sku"] {
        if p[k].as_str().is_none_or(|s| s.trim().is_empty()) {
            out.push(k.into());
        }
    }
    if p["images"].as_array().is_none_or(Vec::is_empty) {
        out.push("public_images".into());
    }
    let policy = &c["policy"];
    let keys: &[&str] = match c["provider"].as_str() {
        Some("amazon") => &["productType"],
        Some("ebay") => &[
            "categoryId",
            "merchantLocationKey",
            "paymentPolicyId",
            "returnPolicyId",
            "fulfillmentPolicyId",
        ],
        Some("kaufland") => &["ean", "vatIndicator", "shippingGroupId"],
        _ => &["supported_provider"],
    };
    for k in keys {
        if policy[*k].as_str().is_none_or(|s| s.is_empty()) {
            out.push((*k).into());
        }
    }
    if p["stock"].as_i64().is_none_or(|n| n < 0) {
        out.push("available_stock".into());
    }
    out
}
pub fn decimal(m: &Value) -> Result<String> {
    let n = m["minor"].as_i64().filter(|n| *n > 0).ok_or(error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Exact positive price required",
    ))?;
    let scale = m["currency"]["scale"]
        .as_u64()
        .filter(|n| *n <= 3)
        .ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Explicit currency scale required",
        ))?;
    let factor = 10_i64.pow(scale as u32);
    Ok(if scale == 0 {
        n.to_string()
    } else {
        format!(
            "{}.{:0width$}",
            n / factor,
            n % factor,
            width = scale as usize
        )
    })
}
pub async fn listing(a: &App, t: &str, c: &Value, p: &Value, prior: &Value) -> Result<Value> {
    match c["provider"].as_str() {
        Some("amazon") => amazon::listing(a, t, c, p, prior).await,
        Some("ebay") => ebay::listing(a, t, c, p, prior).await,
        Some("kaufland") => kaufland::listing(a, t, c, p, prior).await,
        _ => Err(error(StatusCode::BAD_REQUEST, "Unsupported provider")),
    }
}

pub async fn orders(a: &App, t: &str, c: &Value, after: &str, cursor: &str) -> Result<Value> {
    let dest = c["destination"].as_str().unwrap_or("");
    match c["provider"].as_str() {
        Some("amazon") => {
            a.provider(
                t,
                c,
                Method::GET,
                &["orders", "2026-01-01", "orders"],
                &[
                    ("marketplaceIds", dest),
                    ("lastUpdatedAfter", after),
                    ("maxResultsPerPage", "10"),
                    ("paginationToken", cursor),
                    (
                        "includedData",
                        "BUYER,RECIPIENT,PROCEEDS,PAYMENT,TAX,FULFILLMENT",
                    ),
                ],
                None,
            )
            .await
        }
        Some("ebay") => {
            cursor
                .parse::<u64>()
                .or_else(|e| if cursor.is_empty() { Ok(0) } else { Err(e) })
                .map_err(|_| error(StatusCode::BAD_REQUEST, "Invalid page offset"))?;
            a.provider(
                t,
                c,
                Method::GET,
                &["sell", "fulfillment", "v1", "order"],
                &[
                    ("filter", &format!("lastmodifieddate:[{after}..]")),
                    ("limit", "10"),
                    ("offset", if cursor.is_empty() { "0" } else { cursor }),
                ],
                None,
            )
            .await
        }
        Some("kaufland") => {
            if !cursor.is_empty() && cursor.parse::<u64>().is_err() {
                return Err(error(StatusCode::BAD_REQUEST, "Invalid page offset"));
            }
            let mut page = a
                .provider(
                    t,
                    c,
                    Method::GET,
                    &["v2", "orders"],
                    &[
                        ("storefront", dest),
                        ("ts_units_updated_from_iso", after),
                        ("limit", "10"),
                        ("offset", if cursor.is_empty() { "0" } else { cursor }),
                    ],
                    None,
                )
                .await?;
            let orders = page["data"].as_array().cloned().ok_or(error(
                StatusCode::BAD_GATEWAY,
                "Kaufland order collection missing",
            ))?;
            if orders.len() > 10 {
                return Err(error(
                    StatusCode::BAD_GATEWAY,
                    "Kaufland exceeded requested page size",
                ));
            }
            let mut detailed = Vec::new();
            for order in orders {
                let id = order["id_order"].as_str().ok_or(error(
                    StatusCode::BAD_GATEWAY,
                    "Kaufland order identity missing",
                ))?;
                let detail = a
                    .provider(t, c, Method::GET, &["v2", "orders", id], &[], None)
                    .await?;
                detailed.push(detail["data"].clone());
            }
            page["data"] = json!(detailed);
            Ok(page)
        }
        _ => Err(error(StatusCode::BAD_REQUEST, "Unsupported provider")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_currency_and_missing_policy() {
        assert_eq!(
            decimal(&json!({"minor":1234,"currency":{"scale":3}})).unwrap(),
            "1.234"
        );
        assert_eq!(
            decimal(&json!({"minor":1200,"currency":{"scale":0}})).unwrap(),
            "1200"
        );
        assert!(missing(&json!({"provider":"ebay"}), &json!({})).contains(&"categoryId".into()));
    }
}

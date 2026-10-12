//! Official ebay listing adapter, isolated inside the marketplace app.
use super::*;
use reqwest::Method;
pub async fn listing(a: &App, t: &str, c: &Value, p: &Value, prior: &Value) -> Result<Value> {
    let sku = p["sku"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "SKU missing"))?;
    let dest = c["destination"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "Marketplace ID missing"))?;
    let policy = &c["policy"];
    let price = providers::decimal(&p["money"])?;

    let body = json!({"availability":{"shipToLocationAvailability":{"quantity":p["stock"]}},"condition":"NEW","product":{"title":p["title"],"description":p["description"],"imageUrls":p["images"],"aspects":p["attributes"]}});
    a.provider(
        t,
        c,
        Method::PUT,
        &["sell", "inventory", "v1", "inventory_item", sku],
        &[],
        Some(&body),
    )
    .await?;
    // Reconcile offers by stable seller SKU before any non-idempotent create/publish.
    let found = a
        .provider(
            t,
            c,
            Method::GET,
            &["sell", "inventory", "v1", "offer"],
            &[("sku", sku), ("marketplace_id", dest)],
            None,
        )
        .await?;
    let offers = found["offers"].as_array().cloned().unwrap_or_default();
    if offers.len() > 1 {
        return Err(error(
            StatusCode::CONFLICT,
            "Multiple eBay offers share this SKU; reconcile manually",
        ));
    }
    let existing = offers
        .first()
        .and_then(|v| v["offerId"].as_str())
        .or_else(|| prior["externalId"].as_str());
    let offer = json!({"sku":sku,"marketplaceId":dest,"format":"FIXED_PRICE","categoryId":policy["categoryId"],"merchantLocationKey":policy["merchantLocationKey"],"availableQuantity":p["stock"],"listingDescription":p["description"],"pricingSummary":{"price":{"value":price,"currency":p["money"]["currency"]["code"]}},"listingPolicies":{"paymentPolicyId":policy["paymentPolicyId"],"returnPolicyId":policy["returnPolicyId"],"fulfillmentPolicyId":policy["fulfillmentPolicyId"]}});
    let created = if let Some(id) = existing {
        a.provider(
            t,
            c,
            Method::PUT,
            &["sell", "inventory", "v1", "offer", id],
            &[],
            Some(&offer),
        )
        .await?;
        json!({"offerId":id})
    } else {
        a.provider(
            t,
            c,
            Method::POST,
            &["sell", "inventory", "v1", "offer"],
            &[],
            Some(&offer),
        )
        .await?
    };
    let id = created["offerId"].as_str().ok_or(error(
        StatusCode::BAD_GATEWAY,
        "eBay offer result is unknown; reconcile by SKU",
    ))?;
    let live = offers.first().is_some_and(|o| o["status"] == "PUBLISHED");
    let result = if live {
        json!({"alreadyPublished":true})
    } else {
        a.provider(
            t,
            c,
            Method::POST,
            &["sell", "inventory", "v1", "offer", id, "publish"],
            &[],
            Some(&json!({})),
        )
        .await?
    };
    Ok(json!({"provider":result,"externalId":id,"state":"submitted"}))
}

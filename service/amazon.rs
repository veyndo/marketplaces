//! Official amazon listing adapter, isolated inside the marketplace app.
use super::*;
use reqwest::Method;
pub async fn listing(a: &App, t: &str, c: &Value, p: &Value, _prior: &Value) -> Result<Value> {
    let sku = p["sku"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "SKU missing"))?;
    let dest = c["destination"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "Marketplace ID missing"))?;
    let policy = &c["policy"];
    let price = providers::decimal(&p["money"])?;

    let seller = a.account(t, c)?.seller_id.as_deref().ok_or(error(
        StatusCode::UNPROCESSABLE_ENTITY,
        "Amazon seller ID missing",
    ))?;
    let mut attrs = p["attributes"].as_object().cloned().unwrap_or_default();
    for (key, value) in [
        ("item_name", p["title"].clone()),
        ("product_description", p["description"].clone()),
    ] {
        attrs.insert(
            key.into(),
            json!([{"value":value,"language_tag":p["locale"],"marketplace_id":dest}]),
        );
    }
    attrs.insert(
        "main_product_image_locator".into(),
        json!([{"media_location":p["images"][0],"marketplace_id":dest}]),
    );
    for (i, image) in p["images"].as_array().unwrap().iter().skip(1).enumerate() {
        attrs.insert(
            format!("other_product_image_locator_{}", i + 1),
            json!([{"media_location":image,"marketplace_id":dest}]),
        );
    }
    attrs.insert(
        "fulfillment_availability".into(),
        json!([{"fulfillment_channel_code":"DEFAULT","quantity":p["stock"]}]),
    );
    attrs.insert("purchasable_offer".into(),json!([{"marketplace_id":dest,"currency":p["money"]["currency"]["code"],"our_price":[{"schedule":[{"value_with_tax":price.parse::<f64>().map_err(|_|error(StatusCode::BAD_REQUEST,"Invalid price"))?}]}]}]));
    let body =
        json!({"productType":policy["productType"],"requirements":"LISTING","attributes":attrs});
    let result = a
        .provider(
            t,
            c,
            Method::PUT,
            &["listings", "2021-08-01", "items", seller, sku],
            &[
                ("marketplaceIds", dest),
                ("issueLocale", p["locale"].as_str().unwrap()),
            ],
            Some(&body),
        )
        .await?;
    if result["status"] != "ACCEPTED" {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Amazon listing was not accepted; inspect category requirements and seller issues",
        ));
    }
    Ok(json!({"provider":result,"externalId":sku,"state":"submitted"}))
}

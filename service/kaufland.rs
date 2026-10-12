//! Official kaufland listing adapter, isolated inside the marketplace app.
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
    providers::decimal(&p["money"])?;
    let currency = match dest {
        "de" | "sk" | "at" | "fr" | "it" | "es" | "nl" => "EUR",
        "cz" => "CZK",
        "pl" => "PLN",
        _ => {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Unsupported Kaufland storefront",
            ));
        }
    };
    if p["money"]["currency"]["code"] != currency || p["money"]["currency"]["scale"] != 2 {
        return Err(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Kaufland price must use the storefront currency and two decimal places",
        ));
    }
    let ean = policy["ean"]
        .as_str()
        .filter(|s| matches!(s.len(), 13 | 14) && s.bytes().all(|b| b.is_ascii_digit()))
        .ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Kaufland EAN requires 13 or 14 digits",
        ))?;
    let vat = policy["vatIndicator"]
        .as_str()
        .filter(|s| {
            [
                "standard_rate",
                "reduced_rate_1",
                "reduced_rate_2",
                "super_reduced_rate",
                "zero_rate",
            ]
            .contains(s)
        })
        .ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Choose an official Kaufland VAT indicator, e.g. standard_rate",
        ))?;
    let shipping = policy["shippingGroupId"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Kaufland shipping group requires a positive numeric ID",
        ))?;
    let handling = match policy
        .get("handlingTime")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        Some(v) => v.parse::<u64>().ok().filter(|n| *n <= 100).ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Kaufland handling time must be between 0 and 100 days",
        ))?,
        None => 2,
    };

    let mut product = p["attributes"].as_object().cloned().unwrap_or_default();

    product.insert("title".into(), json!([p["title"]]));
    product.insert("description".into(), json!([p["description"]]));
    product.insert("picture".into(), p["images"].clone());
    a.provider(
        t,
        c,
        Method::PUT,
        &["v2", "product-data"],
        &[("locale", p["locale"].as_str().unwrap())],
        Some(&json!({"ean":[ean],"attributes":product})),
    )
    .await?;
    let units = a
        .provider(
            t,
            c,
            Method::GET,
            &["v2", "units"],
            &[("id_offer", sku), ("storefront", dest), ("limit", "2")],
            None,
        )
        .await?;
    let vs = units["data"].as_array().ok_or(error(
        StatusCode::BAD_GATEWAY,
        "Kaufland unit lookup response is invalid; no unit was created",
    ))?;
    if vs.len() > 1 {
        return Err(error(
            StatusCode::CONFLICT,
            "Multiple Kaufland units share this offer; reconcile manually",
        ));
    }
    let body = json!({"listing_price":p["money"]["minor"],"amount":p["stock"],"vat_indicator":vat,"id_shipping_group":shipping,"handling_time":handling});
    let result = if let Some(id) = vs.first().and_then(|v| v["id_unit"].as_u64()) {
        a.provider(
            t,
            c,
            Method::PATCH,
            &["v2", "units", &id.to_string()],
            &[("storefront", dest)],
            Some(&body),
        )
        .await?
    } else {
        a.provider(
            t,
            c,
            Method::POST,
            &["v2", "units"],
            &[("storefront", dest)],
            Some(&{
                let mut create = body.clone();
                create["ean"] = json!(ean);
                create["condition"] = json!("NEW");
                create["id_offer"] = json!(sku);
                create
            }),
        )
        .await?
    };
    Ok(json!({"provider":result,"externalId":sku,"state":"submitted"}))
}

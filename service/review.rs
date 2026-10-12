//! Source-derived SKU, quantity and address suggestions; absent financial facts stay null for explicit review.
use super::*;
pub async fn draft(a: &App, t: &str, v: &Value) -> Result<Value> {
    let row = a
        .one(
            t,
            "inbox",
            v["inboxId"]
                .as_str()
                .ok_or(error(StatusCode::BAD_REQUEST, "Inbox ID required"))?,
        )
        .await?;
    let c = a
        .one(
            t,
            "connections",
            row["connection_id"].as_str().unwrap_or(""),
        )
        .await?;
    mapping::connection(a, t, &c).await?;
    if row["review"].is_object() {
        return Ok(json!({"snapshot":row["review"],"unmappedSkus":[]}));
    }
    let source = &row["source"];
    let listing = a
        .rows(t, "listings", json!({"connection_id":c["id"]}))
        .await?;
    let (lines, unmapped) = lines(source, &listing);
    let ship = &source["fulfillmentStartInstructions"][0]["shippingStep"]["shipTo"]; // eBay Fulfillment contract
    let addr = if ship.is_object() {
        &ship["contactAddress"]
    } else if source["shipping_address"].is_object() {
        &source["shipping_address"]
    } else {
        &source["recipient"]["deliveryAddress"]
    };
    let shipping = address(addr, ship["fullName"].as_str());
    let billing = if source["billing_address"].is_object() {
        address(&source["billing_address"], None)
    } else {
        shipping.clone() // Visible review lets the merchant confirm or correct unavailable billing facts.
    };
    let email = source["buyer"]["buyerRegistrationAddress"]["email"]
        .as_str()
        .or_else(|| source["buyer"]["email"].as_str())
        .or_else(|| source["buyer"]["emailAddress"].as_str())
        .unwrap_or("");
    // No inference that pending/unknown provider payments are captured; reviewer must confirm the source.
    let paid = source["orderPaymentStatus"] == "PAID";
    Ok(
        json!({"snapshot":{"salesChannelId":c["channel_id"],"externalId":row["external_id"],"currency":c["currency"],"locale":c["locale"],"email":email,"billingAddress":billing,"shippingAddress":shipping,"lines":lines,"shipping":{"grossMinor":null,"taxMinor":null,"taxRateBasisPoints":null},"totalMinor":null,"paid":paid},"unmappedSkus":unmapped}),
    )
}
fn address(addr: &Value, full_name: Option<&str>) -> Value {
    let name = full_name
        .or_else(|| addr["name"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{} {}",
                addr["first_name"].as_str().unwrap_or(""),
                addr["last_name"].as_str().unwrap_or("")
            )
            .trim()
            .to_owned()
        });
    let street = addr["addressLine1"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| {
            format!(
                "{} {}",
                addr["street"].as_str().unwrap_or(""),
                addr["house_number"].as_str().unwrap_or("")
            )
            .trim()
            .to_owned()
        });
    json!({"name":name,"street":street,"postalCode":addr["postalCode"].as_str().or_else(||addr["postcode"].as_str()).unwrap_or(""),"city":addr["city"].as_str().unwrap_or(""),"country":addr["countryCode"].as_str().or_else(||addr["country"].as_str()).unwrap_or("")})
}

fn lines(source: &Value, listings: &[Value]) -> (Vec<Value>, Vec<Value>) {
    let items = source["orderItems"]
        .as_array()
        .or_else(|| source["lineItems"].as_array())
        .or_else(|| source["order_units"].as_array());
    let mut lines: Vec<Value> = Vec::new();
    let mut unmapped = Vec::new();
    for item in items.into_iter().flatten() {
        let sku = item["sku"]
            .as_str()
            .or_else(|| item["product"]["sellerSku"].as_str())
            .or_else(|| item["id_offer"].as_str())
            .unwrap_or("");
        let matches = listings
            .iter()
            .filter(|l| l["sku"] == sku)
            .collect::<Vec<_>>();
        let id = if matches.len() == 1 {
            matches[0]["product_id"].clone()
        } else {
            unmapped.push(json!(sku));
            json!("")
        };
        let qty = item["quantity"]
            .as_u64()
            .or_else(|| item["quantityOrdered"].as_u64())
            .or_else(|| {
                (item["id_order_unit"].is_number() || item["id_order_unit"].is_string())
                    .then_some(1)
            });
        // Kaufland represents each purchased unit separately. Merge only uniquely resolved
        // native products; missing quantities remain unknown, never silently become zero.
        if id.as_str().is_some_and(|id| !id.is_empty())
            && let Some(existing) = lines.iter_mut().find(|line| line["productId"] == id)
        {
            existing["quantity"] = existing["quantity"]
                .as_u64()
                .and_then(|old| qty.and_then(|next| old.checked_add(next)))
                .map_or(Value::Null, |sum| json!(sum));
            continue;
        }
        lines.push(json!({"productId":id,"quantity":qty,"amount":{"grossMinor":null,"taxMinor":null,"taxRateBasisPoints":null}}));
    }
    (lines, unmapped)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combines_resolved_units_without_guessing_financials() {
        let (l, u) = lines(
            &json!({"order_units":[{"id_order_unit":1,"id_offer":"SKU"},{"id_order_unit":2,"id_offer":"SKU"}]}),
            &[json!({"sku":"SKU","product_id":"p"})],
        );
        assert_eq!(l.len(), 1);
        assert_eq!(l[0]["quantity"], 2);
        assert!(l[0]["amount"]["grossMinor"].is_null());
        assert!(u.is_empty());
        let (l, _) = lines(
            &json!({"lineItems":[{"sku":"SKU","quantity":1},{"sku":"SKU"}]}),
            &[json!({"sku":"SKU","product_id":"p"})],
        );
        assert!(l[0]["quantity"].is_null());
    }
    #[test]
    fn preserves_source_address_name_and_house_number() {
        let a = address(
            &json!({"first_name":"Jane","last_name":"Doe","street":"Lane","house_number":"73","postcode":"53117","city":"Bonn","country":"DE"}),
            None,
        );
        assert_eq!(a["name"], "Jane Doe");
        assert_eq!(a["street"], "Lane 73");
        assert_eq!(a["postalCode"], "53117");
    }
    #[test]
    fn never_guesses_a_product_or_tax() {
        let (l, u) = lines(
            &json!({"lineItems":[{"sku":"KNOWN","quantity":2},{"sku":"OTHER","quantity":1}]}),
            &[json!({"sku":"KNOWN","product_id":"p"})],
        );
        assert_eq!(l[0]["productId"], "p");
        assert_eq!(l[0]["quantity"], 2);
        assert!(l[0]["amount"]["taxMinor"].is_null());
        assert_eq!(l[1]["productId"], "");
        assert_eq!(u, vec![json!("OTHER")]);
    }
}

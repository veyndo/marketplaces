//! Provider order inbox preserves source identity; explicit review delegates purchase effects exactly once to the native importer.
use super::*;
pub async fn receive(a: &App, t: &str, c: &Value, page: &Value) -> Result<Value> {
    let orders = page["orders"]
        .as_array()
        .or_else(|| page["data"].as_array())
        .ok_or(error(
            StatusCode::BAD_GATEWAY,
            "Provider order page is missing orders",
        ))?;
    if orders.len() > 10 {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Provider exceeded the requested page size",
        ));
    }
    let mut ids = vec![];
    let mut changed = vec![];
    for source in orders {
        let ext = source["orderId"]
            .as_str()
            .or_else(|| source["id_order"].as_str())
            .filter(|s| !s.is_empty() && s.len() <= 100)
            .ok_or(error(
                StatusCode::BAD_GATEWAY,
                "Provider order identity missing",
            ))?;
        if source.to_string().len() > 8000 {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Source order exceeds review record limit; narrow items in seller portal",
            ));
        }
        let key = hash(&format!("{}:{ext}", c["id"].as_str().unwrap()));
        let existing = a.rows(t, "inbox", json!({"source_key":key})).await?;
        if let Some(row) = existing.first() {
            if row["source"] != *source {
                changed.push(ext.to_owned());
            }
            ids.push(row["id"].clone());
            continue;
        }
        let fields = json!({"connection_id":c["id"],"external_id":ext,"source_key":key,"state":"needs_review","source":source,"order_id":null,"review":null});
        match a
            .save(t, "inbox", &json!({"id":key,"revision":0}), fields)
            .await
        {
            Ok(_) => ids.push(json!(key)),
            Err(e) => return Err(e),
        }
    }
    // Never call a partial provider page a complete sync; caller can resume with the exact advertised cursor.
    Ok(
        json!({"received":ids.len(),"inboxIds":ids,"changedSourceOrders":changed,"nextCursor":next_cursor(c,page)?,"reviewRequired":true}),
    )
}
pub async fn import(a: &App, t: &str, v: &Value) -> Result<Value> {
    if v["approve"] != true {
        return Err(error(
            StatusCode::CONFLICT,
            "Confirm the source payment, complete tax breakdown and product mapping before importing",
        ));
    }
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
    let snapshot = &v["snapshot"];
    if snapshot["externalId"] != row["external_id"]
        || snapshot["salesChannelId"] != c["channel_id"]
        || snapshot["currency"] != c["currency"]
        || snapshot["locale"] != c["locale"]
    {
        return Err(error(
            StatusCode::FORBIDDEN,
            "Reviewed order must match the current inbox, channel, currency and language",
        ));
    }
    // Tenant-owned mappings must admit every line; seller SKUs never grant access to arbitrary product IDs.
    let listings = a
        .rows(t, "listings", json!({"connection_id":c["id"]}))
        .await?;
    for line in snapshot["lines"]
        .as_array()
        .ok_or(error(StatusCode::BAD_REQUEST, "Reviewed lines required"))?
    {
        if !listings
            .iter()
            .any(|l| l["product_id"] == line["productId"])
        {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Order contains an unmapped product; create and review its marketplace listing first",
            ));
        }
    }
    if row["state"] == "imported" && row["review"] != *snapshot {
        return Err(error(
            StatusCode::CONFLICT,
            "Imported order snapshot is immutable",
        ));
    }
    let result = a.callback(t, "order_import", snapshot.clone()).await?;
    if row["state"] != "imported" {
        let mut fields = mapping::fields(&row);
        fields["order_id"] = result["order"]["id"].clone();
        fields["review"] = snapshot.clone();
        fields["state"] = json!("imported");
        a.save(t, "inbox", &row, fields).await?;
    }
    Ok(json!({"orderId":result["order"]["id"],"replayed":result["replayed"],"nativeOrder":true}))
}

fn next_cursor(c: &Value, page: &Value) -> Result<Value> {
    if c["provider"] == "amazon" {
        return Ok(page["pagination"]["nextToken"].clone());
    }
    let offset = page["offset"]
        .as_u64()
        .or_else(|| page["pagination"]["offset"].as_u64())
        .unwrap_or(0);
    let limit = page["limit"]
        .as_u64()
        .or_else(|| page["pagination"]["limit"].as_u64())
        .unwrap_or(10);
    let total = page["total"]
        .as_u64()
        .or_else(|| page["pagination"]["total"].as_u64())
        .unwrap_or(0);
    if limit == 0 {
        return Err(error(
            StatusCode::BAD_GATEWAY,
            "Invalid provider pagination",
        ));
    }
    Ok(if offset.saturating_add(limit) < total {
        json!(offset + limit).to_string().into()
    } else {
        Value::Null
    })
}

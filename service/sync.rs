//! Sync jobs persist ambiguous-write fences and source inbox identity through native revisioned app records, never a second queue.
use super::*;
pub async fn run(a: &App, t: &str, input: &Value) -> Result<Value> {
    let id = input["connectionId"]
        .as_str()
        .ok_or(error(StatusCode::BAD_REQUEST, "Connection ID required"))?;
    let c = a.one(t, "connections", id).await?;
    mapping::connection(a, t, &c).await?;
    match input["kind"].as_str() {
        Some("listing") => {
            let lid = input["listingId"].as_str().ok_or(error(
                StatusCode::BAD_REQUEST,
                "Choose a listing before synchronization",
            ))?;
            let row = a.one(t, "listings", lid).await?;
            if row["connection_id"] != id
                || !["ready", "submitted"].iter().any(|s| row["state"] == *s)
            {
                return Err(error(
                    StatusCode::CONFLICT,
                    "Listing must be reviewed and ready; unknown outcomes require reconciliation first",
                ));
            }
            let duplicates = a
                .rows(t, "listings", json!({"connection_id":id,"sku":row["sku"]}))
                .await?;
            if duplicates.len() != 1 {
                return Err(error(
                    StatusCode::CONFLICT,
                    "Marketplace SKU must be unique in this connection",
                ));
            }
            let source=a.callback(t,"channel_product",json!({"id":row["product_id"],"channelId":c["channel_id"],"currency":c["currency"]})).await?;
            let p = mapping::mapped(&source["product"], &row, &c, source["money"].clone())?;
            let effective = mapping::offer_connection(&c, &row);
            if !providers::missing(&effective, &p).is_empty() {
                return Err(error(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Required fields or policies missing; prepare and review this listing",
                ));
            }
            let mut fields = mapping::fields(&row);
            fields["state"] = json!("uncertain");
            let fenced = a.save(t, "listings", &row, fields.clone()).await?;
            let mut next = row.clone();
            next["revision"] = fenced["revision"].clone();
            // Recheck channel/consent immediately before external effects. CAS prevents two jobs sending the same listing concurrently.
            mapping::connection(a, t, &c).await?;
            let result = providers::listing(a, t, &effective, &p, &row["snapshot"]).await?;
            fields["state"] = json!("submitted");
            fields["snapshot"] = json!({"mapped":p,"externalId":result["externalId"],"provider":result["provider"],"submittedAt":chrono::Utc::now().to_rfc3339()});
            a.save(t, "listings", &next, fields).await?;
            Ok(result)
        }
        Some("orders") => {
            let since = input["after"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339());
            chrono::DateTime::parse_from_rfc3339(&since).map_err(|_| {
                error(
                    StatusCode::BAD_REQUEST,
                    "Order sync requires an ISO update timestamp",
                )
            })?;
            let page =
                providers::orders(a, t, &c, &since, input["cursor"].as_str().unwrap_or("")).await?;
            let mut result = intake::receive(a, t, &c, &page).await?;
            result["connectionId"] = json!(id);
            result["after"] = json!(since);
            Ok(result)
        }
        _ => Err(error(
            StatusCode::BAD_REQUEST,
            "Unknown synchronization kind",
        )),
    }
}

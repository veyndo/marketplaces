//! Authenticated app actions and indexed native record access.
use super::*;
impl App {
    pub(super) fn tenant<'a>(&self, h: &'a HeaderMap) -> Result<&'a str> {
        let expected = format!("Bearer {}", self.config.gateway_token);
        let got = h
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let key = ring::hmac::Key::new(
            ring::hmac::HMAC_SHA256,
            self.config.gateway_token.as_bytes(),
        );
        if ring::hmac::verify(
            &key,
            got.as_bytes(),
            ring::hmac::sign(&key, expected.as_bytes()).as_ref(),
        )
        .is_err()
        {
            return Err(error(
                StatusCode::UNAUTHORIZED,
                "Marketplace gateway authentication required",
            ));
        }
        h.get("x-tenant")
            .and_then(|v| v.to_str().ok())
            .filter(|t| self.config.tenants.contains_key(*t))
            .ok_or(error(
                StatusCode::FORBIDDEN,
                "Marketplace tenant is not configured",
            ))
    }
    pub(super) async fn rows(&self, t: &str, entity: &str, filter: Value) -> Result<Vec<Value>> {
        let v = self
            .record(
                t,
                &format!("list_{entity}"),
                json!({"filter":filter,"limit":100}),
            )
            .await?;
        if v["hasMore"] == true {
            return Err(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "More than 100 matches; narrow this operation",
            ));
        }
        Ok(v["elements"].as_array().cloned().unwrap_or_default())
    }
    pub(super) async fn one(&self, t: &str, entity: &str, id: &str) -> Result<Value> {
        // ID lookup uses indexed equality via the generated entity reader (id is a built-in column).
        let v = self
            .callback(
                t,
                "record_action",
                json!({"action":format!("list_{entity}"),"input":{"id":id}}),
            )
            .await?;
        v["elements"]
            .as_array()
            .and_then(|vs| vs.iter().find(|v| v["id"] == id))
            .cloned()
            .ok_or(error(StatusCode::NOT_FOUND, "App record unavailable"))
    }
    pub(super) async fn save(&self, t: &str, e: &str, row: &Value, fields: Value) -> Result<Value> {
        self.record(
            t,
            &format!("save_{e}"),
            json!({"id":row["id"],"revision":row["revision"],"fields":fields}),
        )
        .await
    }
}
pub(super) async fn action(
    State(a): State<App>,
    h: HeaderMap,
    Path(name): Path<String>,
    Json(v): Json<Value>,
) -> Result<Json<Value>> {
    let t = a.tenant(&h)?.to_owned();
    let _permit = a.capacity.clone().try_acquire_owned().map_err(|_| {
        error(
            StatusCode::TOO_MANY_REQUESTS,
            "Marketplace service is busy; retry after current jobs finish",
        )
    })?;
    Ok(Json(match name.as_str() {
        "prepare" => mapping::prepare(&a, &t, &v).await?,
        "review_order" => review::draft(&a, &t, &v).await?,
        "import_order" => intake::import(&a, &t, &v).await?,
        _ => return Err(error(StatusCode::NOT_FOUND, "Unknown marketplace action")),
    }))
}

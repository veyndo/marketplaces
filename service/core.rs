//! Scoped canonical callbacks retain installed digest, consent, actor rights and native app storage ownership.
use super::*;
impl App {
    pub async fn callback(&self, t: &str, op: &str, v: Value) -> Result<Value> {
        let c = self
            .config
            .tenants
            .get(t)
            .ok_or(error(StatusCode::FORBIDDEN, "Unknown marketplace tenant"))?;
        let response = self
            .http
            .post(format!(
                "{}/api/apps/{}/core/{op}",
                c.core_url.trim_end_matches('/'),
                "marketplaces"
            ))
            .bearer_auth(&c.callback_token)
            .header("x-tenant", t)
            .json(&v)
            .send()
            .await
            .map_err(|_| {
                error(
                    StatusCode::BAD_GATEWAY,
                    "Core callback unavailable; retry preserves the order identity",
                )
            })?;
        let code = response.status();
        if response.content_length().is_some_and(|n| n > 65536) {
            return Err(error(
                StatusCode::BAD_GATEWAY,
                "Core callback response is too large",
            ));
        }
        let mut bytes = Vec::new();
        let mut response = response;
        while let Some(chunk) = response.chunk().await.map_err(|_| {
            error(
                StatusCode::BAD_GATEWAY,
                "Core callback response interrupted",
            )
        })? {
            if bytes.len() + chunk.len() > 65536 {
                return Err(error(
                    StatusCode::BAD_GATEWAY,
                    "Core callback response limit",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let result: Value = serde_json::from_slice(&bytes)
            .map_err(|_| error(StatusCode::BAD_GATEWAY, "Invalid core callback response"))?;
        if !code.is_success() {
            let code = if code.is_client_error() {
                StatusCode::from_u16(code.as_u16()).unwrap()
            } else {
                StatusCode::BAD_GATEWAY
            };
            return Err(error(code,result["errors"][0]["detail"].as_str().or_else(|| result["error"].as_str()).unwrap_or("Callback denied: check package digest, current consent, membership and key expiry")));
        }
        Ok(result)
    }
    pub async fn record(&self, t: &str, action: &str, input: Value) -> Result<Value> {
        self.callback(t, "record_action", json!({"action":action,"input":input}))
            .await
    }
}

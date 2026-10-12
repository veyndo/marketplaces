//! Fixed official origins, bounded provider responses, scoped OAuth and HMAC; no request-supplied egress or blind retries.
use super::*;
use reqwest::{Method, Url};
impl App {
    pub fn account(&self, t: &str, c: &Value) -> Result<&config::Account> {
        self.config
            .tenants
            .get(t)
            .and_then(|v| c["account"].as_str().and_then(|id| v.accounts.get(id)))
            .filter(|a| c["provider"] == a.provider)
            .ok_or(error(
                StatusCode::FORBIDDEN,
                "Marketplace account is not configured for this tenant",
            ))
    }
    pub fn origin(&self, provider: &str, region: Option<&str>, oauth: bool) -> Result<Url> {
        if let Some(fixture) = &self.config.fixture {
            return Url::parse(fixture)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid fixture origin"));
        }
        let s = match (provider, oauth) {
            ("amazon", true) => "https://api.amazon.com/",
            ("amazon", false) => match region.unwrap_or("eu") {
                "eu" => "https://sellingpartnerapi-eu.amazon.com/",
                "na" => "https://sellingpartnerapi-na.amazon.com/",
                "fe" => "https://sellingpartnerapi-fe.amazon.com/",
                _ => return Err(error(StatusCode::BAD_REQUEST, "Invalid Amazon region")),
            },
            ("ebay", _) => {
                if region == Some("sandbox") {
                    "https://api.sandbox.ebay.com/"
                } else {
                    "https://api.ebay.com/"
                }
            }
            ("kaufland", false) => "https://sellerapi.kaufland.com/",
            _ => return Err(error(StatusCode::BAD_REQUEST, "Unsupported provider")),
        };
        Ok(Url::parse(s).unwrap())
    }
    pub async fn bounded(&self, response: reqwest::Response) -> Result<Value> {
        let code = response.status();
        let mut r = response;
        let mut bytes = Vec::new();
        while let Some(chunk) = r.chunk().await.map_err(|_| {
            error(
                StatusCode::BAD_GATEWAY,
                "Provider response interrupted; outcome may be unknown",
            )
        })? {
            if bytes.len() + chunk.len() > 65536 {
                return Err(error(
                    StatusCode::BAD_GATEWAY,
                    "Provider response exceeds 64 KiB; narrow the page",
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        if !code.is_success() {
            return Err(error(
                if code.as_u16() == 429 {
                    StatusCode::TOO_MANY_REQUESTS
                } else {
                    StatusCode::BAD_GATEWAY
                },
                match code.as_u16() {
                    429 => {
                        "Provider rate limit reached. Wait before an explicit retry; no write was automatically repeated."
                    }
                    401 | 403 => {
                        "Provider access denied. Check seller consent, token expiry, API scopes and account configuration."
                    }
                    400 | 422 => {
                        "Provider rejected required product fields, policies or category. Inspect the listing in the seller portal."
                    }
                    _ => {
                        "Provider unavailable or ambiguous result. Reconcile in the seller portal before retrying a write."
                    }
                },
            ));
        }
        if bytes.is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_slice(&bytes).map_err(|_| {
            error(
                StatusCode::BAD_GATEWAY,
                "Invalid provider JSON; reconcile before retrying",
            )
        })
    }
    async fn token(&self, a: &config::Account) -> Result<String> {
        let provider = &a.provider;
        let mut u = self.origin(provider, a.region.as_deref(), true)?;
        u.set_path(if provider == "amazon" {
            "/auth/o2/token"
        } else {
            "/identity/v1/oauth2/token"
        });
        let client = a.client_id.as_deref().ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "OAuth client ID is missing",
        ))?;
        let secret = a.client_secret.as_deref().ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "OAuth client secret is missing",
        ))?;
        let refresh = a.refresh_token.as_deref().ok_or(error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Seller OAuth refresh token is missing",
        ))?;
        let mut r = self.http.post(u);
        let fields = if provider == "amazon" {
            vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh),
                ("client_id", client),
                ("client_secret", secret),
            ]
        } else {
            r = r.basic_auth(client, Some(secret));
            vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh),
                (
                    "scope",
                    "https://api.ebay.com/oauth/api_scope/sell.inventory https://api.ebay.com/oauth/api_scope/sell.fulfillment",
                ),
            ]
        };
        let v = self
            .bounded(
                r.form(&fields)
                    .send()
                    .await
                    .map_err(|_| error(StatusCode::BAD_GATEWAY, "OAuth service unavailable"))?,
            )
            .await?;
        v["access_token"].as_str().map(str::to_owned).ok_or(error(
            StatusCode::BAD_GATEWAY,
            "Provider did not return an access token",
        ))
    }
    pub async fn provider(
        &self,
        t: &str,
        c: &Value,
        method: Method,
        segments: &[&str],
        query: &[(&str, &str)],
        body: Option<&Value>,
    ) -> Result<Value> {
        let a = self.account(t, c)?;
        let mut u = self.origin(&a.provider, a.region.as_deref(), false)?;
        u.path_segments_mut()
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid provider origin"))?
            .clear()
            .extend(segments);
        if !query.is_empty() {
            u.query_pairs_mut().extend_pairs(
                query
                    .iter()
                    .copied()
                    .filter(|(k, v)| *k != "paginationToken" || !v.is_empty()),
            );
        }
        let mut r = self.http.request(method.clone(), u.clone());
        if a.provider == "kaufland" {
            use hmac::{Hmac, Mac};
            let stamp = chrono::Utc::now().timestamp().to_string();
            let raw = body.map(Value::to_string).unwrap_or_default();
            let key = a.key.as_deref().ok_or(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Kaufland client key missing",
            ))?;
            let secret = a.secret.as_deref().ok_or(error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Kaufland secret key missing",
            ))?;
            let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
            mac.update(format!("{}\n{u}\n{raw}\n{stamp}", method.as_str()).as_bytes());
            let sign = mac
                .finalize()
                .into_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            r = r
                .header("Shop-Client-Key", key)
                .header("Shop-Timestamp", stamp)
                .header("Shop-Signature", sign)
                .header("User-Agent", "Vendune/MarketplaceStudio");
            if body.is_some() {
                r = r.header("Content-Type", "application/json").body(raw);
            }
        } else {
            let token = self.token(a).await?;
            r = if a.provider == "amazon" {
                r.header("x-amz-access-token", token)
            } else {
                r.bearer_auth(token)
                    .header("Content-Language", c["locale"].as_str().unwrap_or("en-GB"))
            };
            if let Some(b) = body {
                r = r.json(b);
            }
        }
        let response = r.send().await.map_err(|_| {
            error(
                StatusCode::BAD_GATEWAY,
                "Provider request interrupted; reconcile outcome before retrying",
            )
        })?;
        // eBay getOffers documents error 25713 for no offers; only this exact read can yield an empty collection.
        if response.status().as_u16() == 404
            && a.provider == "ebay"
            && method == Method::GET
            && segments == ["sell", "inventory", "v1", "offer"]
        {
            let mut response = response;
            let mut bytes = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| error(StatusCode::BAD_GATEWAY, "Interrupted offer lookup"))?
            {
                if bytes.len() + chunk.len() > 65536 {
                    return Err(error(
                        StatusCode::BAD_GATEWAY,
                        "Offer lookup exceeds response limit",
                    ));
                }
                bytes.extend_from_slice(&chunk);
            }
            let body: Value = serde_json::from_slice(&bytes)
                .map_err(|_| error(StatusCode::BAD_GATEWAY, "Invalid offer lookup"))?;
            if body["errors"]
                .as_array()
                .is_some_and(|es| es.iter().all(|e| e["errorId"] == 25713) && !es.is_empty())
            {
                return Ok(json!({"offers":[]}));
            }
            return Err(error(
                StatusCode::BAD_GATEWAY,
                "eBay offer lookup failed; reconcile seller access",
            ));
        }
        self.bounded(response).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_identifiers_cannot_change_the_origin() {
        let mut u = Url::parse("https://api.ebay.com/").unwrap();
        u.path_segments_mut()
            .unwrap()
            .clear()
            .extend(["sell", "SKU/../../private?x=1"]);
        assert_eq!(u.host_str(), Some("api.ebay.com"));
        assert!(u.path().contains("%2F"));
        assert!(u.query().is_none());
    }
}

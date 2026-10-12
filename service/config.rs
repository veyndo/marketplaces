//! Operator-only destinations and per-tenant credentials; marketplace records can select aliases, never supply secrets or URLs.
use std::collections::HashMap;
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub provider: String,
    pub region: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
    pub seller_id: Option<String>,
    pub key: Option<String>,
    pub secret: Option<String>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Tenant {
    pub core_url: String,
    pub callback_token: String,
    pub accounts: HashMap<String, Account>,
}
pub struct Config {
    pub gateway_token: String,
    pub tenants: HashMap<String, Tenant>,
    pub fixture: Option<String>,
}
impl Config {
    pub fn load() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let gateway_token = std::env::var("APP_TOKEN")?;
        let tenants: HashMap<String, Tenant> =
            serde_json::from_str(&std::env::var("APP_MARKETPLACE_TENANTS")?)?;
        if gateway_token.len() < 24 || tenants.is_empty() || tenants.len() > 1000 {
            return Err("Invalid marketplace configuration".into());
        }
        for (id, t) in &tenants {
            let u = reqwest::Url::parse(&t.core_url)?;
            let local = matches!(u.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
            if id.is_empty()
                || id.len() > 100
                || t.callback_token.len() < 24
                || t.accounts.len() > 40
                || (u.scheme() != "https" && !(local && u.scheme() == "http"))
                || u.path() != "/"
                || !u.username().is_empty()
                || u.password().is_some()
                || u.query().is_some()
                || u.fragment().is_some()
            {
                return Err("Invalid fixed core URL".into());
            }
            for a in t.accounts.values() {
                if !["amazon", "ebay", "kaufland"].contains(&a.provider.as_str())
                    || a.region
                        .as_deref()
                        .is_some_and(|v| !["eu", "na", "fe", "sandbox"].contains(&v))
                {
                    return Err("Invalid provider".into());
                }
            }
        }
        let fixture = std::env::var("MARKETPLACE_TEST_ORIGIN").ok();
        if let Some(origin) = &fixture {
            let u = reqwest::Url::parse(origin)?;
            if std::env::var("MARKETPLACE_ALLOW_FIXTURE").as_deref() != Ok("1")
                || u.scheme() != "http"
                || u.host_str() != Some("127.0.0.1")
                || u.path() != "/"
                || !u.username().is_empty()
                || u.password().is_some()
                || u.query().is_some()
                || u.fragment().is_some()
            {
                return Err("Fixture origin must be explicitly enabled loopback".into());
            }
        }
        Ok(Self {
            gateway_token,
            tenants,
            fixture,
        })
    }
}

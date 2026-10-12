//! Independent Marketplace Studio service; provider protocols stay in this app, native commerce stays in Core.
mod amazon;
mod config;
mod core;
mod ebay;
mod intake;
mod jobs;
mod kaufland;
mod mapping;
mod providers;
mod review;
mod runtime;
mod sync;
mod transport;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;
type Error = (StatusCode, Json<Value>);
type Result<T> = std::result::Result<T, Error>;
fn error(code: StatusCode, s: &str) -> Error {
    (code, Json(json!({"error":s})))
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}
#[derive(Clone)]
struct App {
    config: Arc<config::Config>,
    http: reqwest::Client,
    capacity: Arc<tokio::sync::Semaphore>,
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let a = App {
        config: Arc::new(config::Config::load()?),
        http: reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(8))
            .build()?,
        capacity: Arc::new(tokio::sync::Semaphore::new(8)),
    };
    let r = Router::new()
        .route(
            "/health",
            get(|| async { Json(json!({"ok":true,"service":"marketplaces"})) }),
        )
        .route("/actions/{action}", post(runtime::action))
        .route("/events", post(jobs::events))
        .layer(DefaultBodyLimit::max(65536))
        .with_state(a);
    let bind = std::env::var("APP_BIND_ADDR").unwrap_or("0.0.0.0:8799".into());
    axum::serve(tokio::net::TcpListener::bind(bind).await?, r).await?;
    Ok(())
}

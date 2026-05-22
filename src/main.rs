mod bilibili;
mod cache;
mod mirror_cdn;

use axum::{
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::Instant;

fn redirect_302(url: &str) -> Response {
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, url)
        .body(axum::body::Body::empty())
        .unwrap()
}

static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

#[derive(Deserialize)]
struct RedirectQuery {
    url: Option<String>,
}

async fn redirect_handler(Query(params): Query<RedirectQuery>) -> impl IntoResponse {
    let target_url = match params.url {
        Some(url) => url,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "Missing \"url\" parameter",
                    "usage": "GET /?url=<bilibili video or live URL>"
                })),
            )
                .into_response();
        }
    };

    let cache_key = format!("raw:{}", target_url);

    if let Some(cached) = cache::CACHE.get(&cache_key) {
        return redirect_302(&cached);
    }

    match bilibili::resolve_raw_url(&target_url).await {
        Ok(cdn_url) => {
            cache::CACHE.insert(cache_key, cdn_url.clone());
            redirect_302(&cdn_url)
        }
        Err(err) => {
            tracing::error!("[ERROR] Failed to resolve: {} {}", target_url, err);
            (
                StatusCode::BAD_GATEWAY,
                axum::Json(serde_json::json!({
                    "error": "Failed to resolve video URL",
                    "detail": err
                })),
            )
                .into_response()
        }
    }
}

async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "uptime": START_TIME.elapsed().as_secs()
    }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = Router::new()
        .route("/", get(redirect_handler))
        .route("/health", get(health_handler));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();

    tracing::info!("BiliAnalysis Server running on http://localhost:{}", port);
    axum::serve(listener, app).await.unwrap();
}

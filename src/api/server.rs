//! Axum server setup and routing.

use axum::{
    middleware,
    routing::get,
    Router,
};
use axum::http::HeaderValue;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::{docs::ApiDoc, handlers, middleware as api_middleware};
use crate::api::models::{PriceStreamMessage, ReservesInfo};
use crate::app_state::AppState;

/// Run the Axum API server.
pub async fn run_server(
    state: AppState,
    port: u16,
    rate_limit_rpm: u32,
    cors_origins: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Ensuring default pool exists in database");
    state.repository.ensure_default_pool().await?;

    let limiter = api_middleware::rate_limit::create_rate_limiter(rate_limit_rpm);

    let api_routes = Router::new()
        .route("/health", get(handlers::health::health_check))
        .route("/pools", get(handlers::pools::list_pools))
        .route("/price/current/:pool", get(handlers::price::get_current_price))
        .route("/price/history/:pool", get(handlers::price::get_price_history))
        .route("/stats/:pool", get(handlers::stats::get_stats))
        .route("/events/:pool", get(handlers::events::get_recent_events))
        .route("/stream/:pool", get(handlers::stream::websocket_handler));

    let cors = build_cors_layer(cors_origins);

    let middleware_stack = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(middleware::from_fn(api_middleware::logging::log_requests))
        .layer(middleware::from_fn(move |req, next| {
            api_middleware::rate_limit::rate_limit(limiter.clone(), req, next)
        }));

    let static_files = ServeDir::new("public")
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new("public/index.html"));

    let app = Router::new()
        .nest_service("/", static_files)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .nest("/api/v1", api_routes)
        .layer(middleware_stack)
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!(addr = %addr, "Starting API server");

    tokio::spawn(async move {
        poll_and_broadcast_prices(state).await;
    });

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_cors_layer(origins: Vec<String>) -> CorsLayer {
    if origins.is_empty() || origins.iter().any(|o| o == "*") {
        CorsLayer::new().allow_origin(Any)
    } else {
        let mut layer = CorsLayer::new();
        for origin in origins {
            if let Ok(header) = origin.parse::<HeaderValue>() {
                layer = layer.clone().allow_origin(header);
            }
        }
        layer
    }
}

async fn poll_and_broadcast_prices(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    let mut last_seen: HashMap<i64, i64> = HashMap::new();

    loop {
        interval.tick().await;

        let pools = match state.repository.get_all_pools().await {
            Ok(pools) => pools,
            Err(_) => continue,
        };

        for pool in pools {
            let name = pool.name.unwrap_or_else(|| pool.address.clone());
            let latest = match state.repository.get_latest_price(pool.id).await {
                Ok(Some(price)) => price,
                Ok(None) => continue,
                Err(_) => continue,
            };

            let last_block = last_seen.get(&pool.id).copied().unwrap_or_default();
            if latest.block_number <= last_block {
                continue;
            }

            last_seen.insert(pool.id, latest.block_number);

            let msg = PriceStreamMessage {
                event_type: "price_update".to_string(),
                pool: name,
                price: latest.price,
                block_number: latest.block_number as u64,
                timestamp: chrono::DateTime::from_timestamp(latest.block_timestamp, 0)
                    .unwrap_or_else(chrono::Utc::now),
                reserves: ReservesInfo {
                    weth: latest.reserve0_human,
                    usdt: latest.reserve1_human,
                },
            };

            state.broadcast_price_update(msg);
        }
    }
}

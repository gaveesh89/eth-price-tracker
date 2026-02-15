//! OpenAPI documentation for the REST API.

use utoipa::OpenApi;

use crate::api::handlers;

/// OpenAPI documentation for the REST API.
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::health::health_check,
        handlers::pools::list_pools,
        handlers::price::get_current_price,
        handlers::price::get_price_history,
        handlers::stats::get_stats,
        handlers::events::get_recent_events,
        handlers::stream::websocket_handler,
    ),
    components(schemas(
        crate::api::models::HealthResponse,
        crate::api::models::PoolInfo,
        crate::api::models::CurrentPriceResponse,
        crate::api::models::PricePoint,
        crate::api::models::PaginatedResponse<crate::api::models::PricePoint>,
        crate::api::models::StatsResponse,
        crate::api::models::ErrorResponse,
        crate::api::models::RecentEventResponse,
    )),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Pools", description = "Pool management"),
        (name = "Price", description = "Price data endpoints"),
        (name = "Statistics", description = "Statistical data"),
        (name = "Events", description = "Event listing"),
        (name = "Streaming", description = "WebSocket streaming"),
    ),
    info(
        title = "ETH Price Tracker API",
        version = "1.0.0",
        description = "Production-grade Ethereum price indexer API",
    )
)]
pub struct ApiDoc;

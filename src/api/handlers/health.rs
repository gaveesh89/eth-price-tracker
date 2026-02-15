//! Health check endpoint.

use axum::{extract::State, Json};
use std::time::SystemTime;
use tracing::instrument;

use crate::api::middleware::error::ApiError;
use crate::api::models::{HealthResponse, HealthStatus};
use crate::app_state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "Health"
)]
/// Returns service health information.
#[instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    let uptime = SystemTime::now()
        .duration_since(state.start_time)
        .unwrap_or_default()
        .as_secs();

    let indexed_block = state
        .repository
        .get_state(1)
        .await?
        .map(|s| s.last_indexed_block as u64)
        .unwrap_or(0);

    let db_status = match state.repository.health_check().await {
        Ok(()) => HealthStatus::Healthy,
        Err(_) => HealthStatus::Unhealthy,
    };

    let ws_status = if state.ws_connected.load(std::sync::atomic::Ordering::Relaxed) {
        HealthStatus::Healthy
    } else {
        HealthStatus::Degraded
    };

    let status = match (&db_status, &ws_status) {
        (HealthStatus::Healthy, HealthStatus::Healthy) => HealthStatus::Healthy,
        (HealthStatus::Healthy, _) => HealthStatus::Degraded,
        (_, HealthStatus::Healthy) => HealthStatus::Degraded,
        _ => HealthStatus::Unhealthy,
    };

    Ok(Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        indexed_block,
        database_status: format!("{:?}", db_status).to_lowercase(),
        websocket_status: format!("{:?}", ws_status).to_lowercase(),
    }))
}

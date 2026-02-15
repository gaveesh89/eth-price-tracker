//! Event listing endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::instrument;

use crate::api::middleware::error::ApiError;
use crate::api::models::{RecentEventResponse, SyncEventInfo};
use crate::app_state::AppState;

/// Query parameters for recent events.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    50
}

#[utoipa::path(
    get,
    path = "/api/v1/events/{pool}",
    params(
        ("pool" = String, Path, description = "Pool name")
    ),
    responses(
        (status = 200, description = "Recent events", body = RecentEventResponse)
    ),
    tag = "Events"
)]
/// Returns recent sync events for a pool.
#[instrument(skip(state), fields(pool = %pool_name))]
pub async fn get_recent_events(
    State(state): State<AppState>,
    Path(pool_name): Path<String>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<RecentEventResponse>, ApiError> {
    let pool_name_normalized = pool_name.replace('-', "/");

    if query.limit == 0 || query.limit > 1000 {
        return Err(ApiError::BadRequest("limit must be between 1 and 1000".to_string()));
    }

    let pool = state
        .repository
        .get_pool_by_name(&pool_name_normalized)
        .await?
        .ok_or_else(|| ApiError::NotFound("Pool not found".to_string()))?;

    let events = state
        .repository
        .get_recent_events(pool.id, query.limit as i64)
        .await?;

    let items = events
        .into_iter()
        .map(|e| SyncEventInfo {
            block_number: e.block_number as u64,
            timestamp: DateTime::from_timestamp(e.block_timestamp, 0).unwrap_or_else(Utc::now),
            tx_hash: e.tx_hash,
            reserve0: e.reserve0,
            reserve1: e.reserve1,
        })
        .collect();

    Ok(Json(RecentEventResponse {
        pool: pool_name_normalized,
        events: items,
    }))
}

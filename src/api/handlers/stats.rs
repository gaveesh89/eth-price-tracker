//! Statistics endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use tracing::instrument;

use crate::api::middleware::error::ApiError;
use crate::api::models::{StatsPeriod, StatsResponse};
use crate::app_state::AppState;

/// Query parameters for statistics.
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    #[serde(default = "default_period")]
    period: String,
}

fn default_period() -> String {
    "24h".to_string()
}

#[utoipa::path(
    get,
    path = "/api/v1/stats/{pool}",
    params(
        ("pool" = String, Path, description = "Pool name")
    ),
    responses(
        (status = 200, description = "Statistics", body = StatsResponse)
    ),
    tag = "Statistics"
)]
/// Returns statistics for a pool over a time period.
#[instrument(skip(state), fields(pool = %pool_name))]
pub async fn get_stats(
    State(state): State<AppState>,
    Path(pool_name): Path<String>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<StatsResponse>, ApiError> {
    let pool_name_normalized = pool_name.replace('-', "/");

    let pool = state
        .repository
        .get_pool_by_name(&pool_name_normalized)
        .await?
        .ok_or_else(|| ApiError::NotFound("Pool not found".to_string()))?;

    let (period_enum, from_timestamp) = match query.period.as_str() {
        "1h" => (StatsPeriod::Hour1, Utc::now() - Duration::hours(1)),
        "24h" => (StatsPeriod::Hour24, Utc::now() - Duration::hours(24)),
        "7d" => (StatsPeriod::Day7, Utc::now() - Duration::days(7)),
        "30d" => (StatsPeriod::Day30, Utc::now() - Duration::days(30)),
        "all" => (StatsPeriod::All, DateTime::from_timestamp(0, 0).unwrap()),
        _ => {
            return Err(ApiError::BadRequest(
                "Invalid period. Use: 1h, 24h, 7d, 30d, or all".to_string(),
            ))
        }
    };

    let stats_data = state
        .repository
        .get_stats_for_period(pool.id, from_timestamp.timestamp())
        .await?;

    let current = state
        .repository
        .get_latest_price(pool.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("No price data".to_string()))?;

    let change_percent = if stats_data.first_price.unwrap_or(0.0) > 0.0 {
        let first = stats_data.first_price.unwrap_or(0.0);
        ((current.price - first) / first) * 100.0
    } else {
        0.0
    };

    let response = StatsResponse {
        pool: pool_name_normalized,
        period: period_enum,
        current_price: current.price,
        high: stats_data.max_price,
        low: stats_data.min_price,
        average: stats_data.avg_price,
        change_percent,
        volume_events: stats_data.total_events as u64,
        first_timestamp: DateTime::from_timestamp(stats_data.first_timestamp, 0)
            .unwrap_or_else(Utc::now),
        last_timestamp: DateTime::from_timestamp(stats_data.last_timestamp, 0)
            .unwrap_or_else(Utc::now),
    };

    Ok(Json(response))
}

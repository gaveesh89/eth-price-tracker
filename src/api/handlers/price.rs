//! Price endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use tracing::{info, instrument};

use crate::api::middleware::error::ApiError;
use crate::api::models::{
    CurrentPriceResponse, HistoryQuery, PaginatedResponse, PaginationInfo, PricePoint, ReservesInfo,
};
use crate::app_state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/price/current/{pool}",
    params(
        ("pool" = String, Path, description = "Pool name (e.g., WETH-USDT)")
    ),
    responses(
        (status = 200, description = "Current price", body = CurrentPriceResponse),
        (status = 404, description = "Pool not found")
    ),
    tag = "Price"
)]
/// Returns the latest confirmed price for a pool.
#[instrument(skip(state), fields(pool = %pool_name))]
pub async fn get_current_price(
    State(state): State<AppState>,
    Path(pool_name): Path<String>,
) -> Result<Json<CurrentPriceResponse>, ApiError> {
    info!("Fetching current price");

    let pool_name_normalized = pool_name.replace('-', "/");

    let pool = state
        .repository
        .get_pool_by_name(&pool_name_normalized)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Pool {} not found", pool_name_normalized)))?;

    let price_point = state
        .repository
        .get_latest_price(pool.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("No price data available".to_string()))?;

    let change_24h = state.repository.get_24h_price_change(pool.id).await.ok();

    let timestamp =
        DateTime::from_timestamp(price_point.block_timestamp, 0).unwrap_or_else(Utc::now);

    let response = CurrentPriceResponse {
        pool: pool_name_normalized,
        price: price_point.price,
        block_number: price_point.block_number as u64,
        timestamp,
        tx_hash: price_point.tx_hash,
        reserves: ReservesInfo {
            weth: price_point.reserve0_human,
            usdt: price_point.reserve1_human,
        },
        change_24h,
    };

    info!(
        price = response.price,
        block = response.block_number,
        "Current price fetched"
    );

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/api/v1/price/history/{pool}",
    params(
        ("pool" = String, Path, description = "Pool name"),
        HistoryQuery
    ),
    responses(
        (status = 200, description = "Historical prices", body = PaginatedResponse<PricePoint>)
    ),
    tag = "Price"
)]
/// Returns paginated historical prices for a pool.
#[instrument(skip(state), fields(pool = %pool_name))]
pub async fn get_price_history(
    State(state): State<AppState>,
    Path(pool_name): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<PaginatedResponse<PricePoint>>, ApiError> {
    let pool_name_normalized = pool_name.replace('-', "/");

    if query.page < 1 {
        return Err(ApiError::BadRequest("page must be >= 1".to_string()));
    }
    if query.page_size > 1000 {
        return Err(ApiError::BadRequest(
            "page_size must be <= 1000".to_string(),
        ));
    }

    let pool = state
        .repository
        .get_pool_by_name(&pool_name_normalized)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Pool {} not found", pool_name_normalized)))?;

    let from_ts = parse_timestamp(&query.from)?;
    let to_ts = parse_timestamp(&query.to)?;

    let offset = (query.page - 1) * query.page_size;

    let (prices, total_count) = state
        .repository
        .get_price_history_paginated(
            pool.id,
            from_ts,
            to_ts,
            query.page_size as i64,
            offset as i64,
        )
        .await?;

    let data = prices
        .into_iter()
        .map(|p| PricePoint {
            block_number: p.block_number as u64,
            timestamp: DateTime::from_timestamp(p.block_timestamp, 0).unwrap_or_else(Utc::now),
            price: p.price,
            tx_hash: p.tx_hash,
            reserves: ReservesInfo {
                weth: p.reserve0_human,
                usdt: p.reserve1_human,
            },
        })
        .collect::<Vec<_>>();

    let has_next_page = (offset + query.page_size) < total_count as u32;

    let response = PaginatedResponse {
        data,
        pagination: PaginationInfo {
            page: query.page,
            page_size: query.page_size,
            total_count,
            has_next_page,
        },
    };

    info!(
        count = response.data.len(),
        total = total_count,
        page = query.page,
        "Historical prices fetched"
    );

    Ok(Json(response))
}

fn parse_timestamp(ts: &Option<String>) -> Result<Option<i64>, ApiError> {
    match ts {
        None => Ok(None),
        Some(s) => {
            if let Ok(unix_ts) = s.parse::<i64>() {
                return Ok(Some(unix_ts));
            }

            if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
                return Ok(Some(dt.timestamp()));
            }

            Err(ApiError::BadRequest(format!(
                "Invalid timestamp format: {}. Use ISO 8601 or UNIX timestamp",
                s
            )))
        }
    }
}

//! Pool listing endpoints.

use axum::{extract::State, Json};
use tracing::instrument;

use crate::api::middleware::error::ApiError;
use crate::api::models::{PoolInfo, TokenInfo};
use crate::app_state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/pools",
    responses(
        (status = 200, description = "List of tracked pools", body = Vec<PoolInfo>)
    ),
    tag = "Pools"
)]
/// Returns the list of tracked pools.
#[instrument(skip(state))]
pub async fn list_pools(State(state): State<AppState>) -> Result<Json<Vec<PoolInfo>>, ApiError> {
    let pools = state.repository.get_all_pools().await?;

    let pool_infos = pools
        .into_iter()
        .map(|p| {
            let name = p.name.unwrap_or_else(|| p.address.clone());
            PoolInfo {
                name,
                address: p.address,
                token0: TokenInfo {
                    symbol: p.token0_symbol.unwrap_or_else(|| "TOKEN0".to_string()),
                    address: p.token0_address,
                    decimals: p.token0_decimals as u8,
                },
                token1: TokenInfo {
                    symbol: p.token1_symbol.unwrap_or_else(|| "TOKEN1".to_string()),
                    address: p.token1_address,
                    decimals: p.token1_decimals as u8,
                },
                last_indexed_block: p.last_indexed_block as u64,
                total_events: p.total_events as u64,
            }
        })
        .collect();

    Ok(Json(pool_infos))
}

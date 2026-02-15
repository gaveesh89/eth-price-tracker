//! API request and response models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// API response for current price.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CurrentPriceResponse {
    /// Pool identifier (e.g., "WETH/USDT")
    pub pool: String,
    /// Current ETH/USDT price
    pub price: f64,
    /// Block number where this price was recorded
    pub block_number: u64,
    /// Block timestamp (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// Transaction hash
    pub tx_hash: String,
    /// Reserve amounts
    pub reserves: ReservesInfo,
    /// 24-hour price change percentage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub change_24h: Option<f64>,
}

/// Reserve amounts for a pool.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReservesInfo {
    /// WETH reserve (human-readable)
    pub weth: f64,
    /// USDT reserve (human-readable)
    pub usdt: f64,
}

/// Historical price point.
/// Historical price point.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PricePoint {
    /// Block number where this price was recorded
    pub block_number: u64,
    /// Block timestamp (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// ETH/USDT price
    pub price: f64,
    /// Transaction hash
    pub tx_hash: String,
    /// Reserve amounts
    pub reserves: ReservesInfo,
}

/// Paginated response wrapper.
/// Paginated response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    /// Response data
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationInfo,
}

/// Pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// Current page number
    pub page: u32,
    /// Items per page
    pub page_size: u32,
    /// Total number of items
    pub total_count: u64,
    /// Whether there is another page
    pub has_next_page: bool,
}

/// Query parameters for historical prices.
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct HistoryQuery {
    /// Start timestamp (ISO 8601) or UNIX timestamp
    #[serde(default)]
    pub from: Option<String>,
    /// End timestamp (ISO 8601) or UNIX timestamp
    #[serde(default)]
    pub to: Option<String>,
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page (max 1000)
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    100
}

/// Pool information.
/// Pool metadata for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PoolInfo {
    /// Pool name (e.g., "WETH/USDT")
    pub name: String,
    /// Pool contract address
    pub address: String,
    /// Token0 metadata
    pub token0: TokenInfo,
    /// Token1 metadata
    pub token1: TokenInfo,
    /// Last indexed block number
    pub last_indexed_block: u64,
    /// Total events processed
    pub total_events: u64,
}

/// Token metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenInfo {
    /// Token symbol
    pub symbol: String,
    /// Token contract address
    pub address: String,
    /// Token decimals
    pub decimals: u8,
}

/// Statistics response.
/// Statistics response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsResponse {
    /// Pool name
    pub pool: String,
    /// Requested stats period
    pub period: StatsPeriod,
    /// Current price
    pub current_price: f64,
    /// Highest price in period
    pub high: f64,
    /// Lowest price in period
    pub low: f64,
    /// Average price in period
    pub average: f64,
    /// Percentage change from first to last
    pub change_percent: f64,
    /// Number of events in period
    pub volume_events: u64,
    /// Timestamp of first event in period
    pub first_timestamp: DateTime<Utc>,
    /// Timestamp of last event in period
    pub last_timestamp: DateTime<Utc>,
}

/// Supported statistics periods.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum StatsPeriod {
    /// Last 1 hour
    Hour1,
    /// Last 24 hours
    Hour24,
    /// Last 7 days
    Day7,
    /// Last 30 days
    Day30,
    /// All available data
    All,
}

/// Health check response.
/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Overall health status
    pub status: HealthStatus,
    /// Application version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last indexed block number
    pub indexed_block: u64,
    /// Database status
    pub database_status: String,
    /// WebSocket status
    pub websocket_status: String,
}

/// Health status states.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All services healthy
    Healthy,
    /// Partial degradation
    Degraded,
    /// Unhealthy state
    Unhealthy,
}

/// Error response.
/// Error response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Error type
    pub error: String,
    /// Human-readable message
    pub message: String,
    /// Optional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Recent event response.
/// Recent events response.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecentEventResponse {
    /// Pool name
    pub pool: String,
    /// List of recent events
    pub events: Vec<SyncEventInfo>,
}

/// Sync event data.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SyncEventInfo {
    /// Block number where event occurred
    pub block_number: u64,
    /// Block timestamp
    pub timestamp: DateTime<Utc>,
    /// Transaction hash
    pub tx_hash: String,
    /// Reserve0 raw value
    pub reserve0: String,
    /// Reserve1 raw value
    pub reserve1: String,
}

/// WebSocket message for price stream.
/// WebSocket price update message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceStreamMessage {
    /// Event type (e.g., "price_update", "connected")
    pub event_type: String,
    /// Pool name
    pub pool: String,
    /// Price value
    pub price: f64,
    /// Block number
    pub block_number: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Reserve amounts
    pub reserves: ReservesInfo,
}

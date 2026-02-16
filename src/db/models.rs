//! Database models that map to SQL tables.
//!
//! These structures represent rows in the database and provide
//! conversions from blockchain types to database representations.

use alloy::primitives::{Address, FixedBytes, U256};
use serde::{Deserialize, Serialize};

/// Represents a Uniswap V2 pool in the database.
///
/// Maps to the `pools` table. Stores metadata about the pool
/// including token information.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PoolRecord {
    /// Database-assigned unique identifier
    pub id: i64,
    /// Pool contract address (hex string with 0x prefix)
    pub address: String,
    /// Optional human-readable name (e.g., "USDC-WETH")
    pub name: Option<String>,
    /// Token0 contract address (hex string with 0x prefix)
    pub token0_address: String,
    /// Token0 symbol (e.g., "USDC")
    pub token0_symbol: Option<String>,
    /// Token0 decimal places (e.g., 6 for USDC)
    pub token0_decimals: i32,
    /// Token1 contract address (hex string with 0x prefix)
    pub token1_address: String,
    /// Token1 symbol (e.g., "WETH")
    pub token1_symbol: Option<String>,
    /// Token1 decimal places (e.g., 18 for WETH)
    pub token1_decimals: i32,
    /// Unix timestamp when record was created
    pub created_at: i64,
}

impl PoolRecord {
    /// Creates a new pool record from blockchain data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use alloy::primitives::Address;
    /// use eth_uniswap_alloy::db::models::PoolRecord;
    ///
    /// let pool = PoolRecord::new(
    ///     "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse().unwrap(),
    ///     Some("USDC-WETH".to_string()),
    ///     "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
    ///     Some("USDC".to_string()),
    ///     6,
    ///     "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
    ///     Some("WETH".to_string()),
    ///     18,
    /// );
    /// ```
    pub fn new(
        address: Address,
        name: Option<String>,
        token0_address: Address,
        token0_symbol: Option<String>,
        token0_decimals: u8,
        token1_address: Address,
        token1_symbol: Option<String>,
        token1_decimals: u8,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            address: format!("{:?}", address),
            name,
            token0_address: format!("{:?}", token0_address),
            token0_symbol,
            token0_decimals: token0_decimals as i32,
            token1_address: format!("{:?}", token1_address),
            token1_symbol,
            token1_decimals: token1_decimals as i32,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Represents a raw sync event from the blockchain.
///
/// Maps to the `sync_events` table. Stores the original event data
/// for audit purposes and reorg recovery.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncEventRecord {
    /// Database-assigned unique identifier
    pub id: i64,
    /// Foreign key to pools table
    pub pool_id: i64,
    /// Block number where event occurred
    pub block_number: i64,
    /// Block hash (hex string with 0x prefix)
    pub block_hash: String,
    /// Unix timestamp of the block
    pub block_timestamp: i64,
    /// Transaction hash (hex string with 0x prefix)
    pub tx_hash: String,
    /// Log index within the transaction
    pub log_index: i32,
    /// Reserve amount of token0 (stored as TEXT for U256 precision)
    pub reserve0: String,
    /// Reserve amount of token1 (stored as TEXT for U256 precision)
    pub reserve1: String,
    /// Whether this event is from a finalized block
    pub is_confirmed: bool,
    /// Unix timestamp when record was created
    pub created_at: i64,
}

impl SyncEventRecord {
    /// Creates a new sync event record from blockchain data.
    ///
    /// # Arguments
    ///
    /// * `pool_id` - Database ID of the pool
    /// * `block_number` - Block number where event occurred
    /// * `block_hash` - Hash of the block
    /// * `block_timestamp` - Unix timestamp of the block
    /// * `tx_hash` - Transaction hash
    /// * `log_index` - Index of the log in the transaction
    /// * `reserve0` - Reserve amount of token0 (as U256)
    /// * `reserve1` - Reserve amount of token1 (as U256)
    /// * `is_confirmed` - Whether the block is considered final
    ///
    /// # Example
    ///
    /// ```no_run
    /// use alloy::primitives::{U256, FixedBytes};
    /// use eth_uniswap_alloy::db::models::SyncEventRecord;
    ///
    /// let event = SyncEventRecord::new(
    ///     1,
    ///     19000000,
    ///     FixedBytes::from([0u8; 32]),
    ///     1706745600,
    ///     FixedBytes::from([0u8; 32]),
    ///     0,
    ///     U256::from(1000000000u64),
    ///     U256::from(500000000000000000u64),
    ///     false,
    /// );
    /// ```
    pub fn new(
        pool_id: i64,
        block_number: u64,
        block_hash: FixedBytes<32>,
        block_timestamp: u64,
        tx_hash: FixedBytes<32>,
        log_index: u32,
        reserve0: U256,
        reserve1: U256,
        is_confirmed: bool,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            pool_id,
            block_number: block_number as i64,
            block_hash: format!("{:?}", block_hash),
            block_timestamp: block_timestamp as i64,
            tx_hash: format!("{:?}", tx_hash),
            log_index: log_index as i32,
            reserve0: reserve0.to_string(),
            reserve1: reserve1.to_string(),
            is_confirmed,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Converts reserve0 TEXT back to U256.
    pub fn reserve0_u256(&self) -> Result<U256, crate::error::TrackerError> {
        U256::from_str_radix(&self.reserve0, 10).map_err(|e| {
            crate::error::TrackerError::DecodingError {
                message: format!("Failed to parse reserve0: {}", self.reserve0),
                source: Some(Box::new(e)),
            }
        })
    }

    /// Converts reserve1 TEXT back to U256.
    pub fn reserve1_u256(&self) -> Result<U256, crate::error::TrackerError> {
        U256::from_str_radix(&self.reserve1, 10).map_err(|e| {
            crate::error::TrackerError::DecodingError {
                message: format!("Failed to parse reserve1: {}", self.reserve1),
                source: Some(Box::new(e)),
            }
        })
    }
}

/// Represents a computed price point.
///
/// Maps to the `price_points` table. Stores human-readable
/// prices and reserves for fast queries.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PricePointRecord {
    /// Database-assigned unique identifier
    pub id: i64,
    /// Foreign key to pools table
    pub pool_id: i64,
    /// Block number where price was observed
    pub block_number: i64,
    /// Unix timestamp of the block
    pub block_timestamp: i64,
    /// Transaction hash (hex string with 0x prefix)
    pub tx_hash: String,
    /// Computed price (token1 per token0)
    pub price: f64,
    /// Raw reserve of token0 (TEXT for U256 precision)
    pub reserve0_raw: String,
    /// Raw reserve of token1 (TEXT for U256 precision)
    pub reserve1_raw: String,
    /// Decimal-adjusted reserve of token0 (for display)
    pub reserve0_human: f64,
    /// Decimal-adjusted reserve of token1 (for display)
    pub reserve1_human: f64,
    /// Whether this price is from a finalized block
    pub is_confirmed: bool,
    /// Unix timestamp when record was created
    pub created_at: i64,
}

/// Lightweight price point row for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PricePointRow {
    /// Block number where price was recorded
    pub block_number: i64,
    /// Block timestamp (unix seconds)
    pub block_timestamp: i64,
    /// Transaction hash
    pub tx_hash: String,
    /// Price value
    pub price: f64,
    /// Human-readable reserve0
    pub reserve0_human: f64,
    /// Human-readable reserve1
    pub reserve1_human: f64,
}

/// Aggregated stats row for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StatsRow {
    /// Total events in the period
    pub total_events: i64,
    /// Minimum price
    pub min_price: f64,
    /// Maximum price
    pub max_price: f64,
    /// Average price
    pub avg_price: f64,
    /// First timestamp in period
    pub first_timestamp: i64,
    /// Last timestamp in period
    pub last_timestamp: i64,
    /// First price in period
    pub first_price: Option<f64>,
}

/// Pool row with indexer metadata.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PoolRow {
    /// Pool database ID
    pub id: i64,
    /// Optional pool name
    pub name: Option<String>,
    /// Pool address
    pub address: String,
    /// Token0 symbol
    pub token0_symbol: Option<String>,
    /// Token0 address
    pub token0_address: String,
    /// Token0 decimals
    pub token0_decimals: i64,
    /// Token1 symbol
    pub token1_symbol: Option<String>,
    /// Token1 address
    pub token1_address: String,
    /// Token1 decimals
    pub token1_decimals: i64,
    /// Last indexed block
    pub last_indexed_block: i64,
    /// Total events processed
    pub total_events: i64,
}

/// Lightweight sync event row for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncEventRow {
    /// Block number where event occurred
    pub block_number: i64,
    /// Block timestamp (unix seconds)
    pub block_timestamp: i64,
    /// Transaction hash
    pub tx_hash: String,
    /// Reserve0 raw value
    pub reserve0: String,
    /// Reserve1 raw value
    pub reserve1: String,
}

impl PricePointRecord {
    /// Creates a new price point record from blockchain data and computed values.
    ///
    /// # Arguments
    ///
    /// * `pool_id` - Database ID of the pool
    /// * `block_number` - Block number
    /// * `block_timestamp` - Unix timestamp
    /// * `tx_hash` - Transaction hash
    /// * `price` - Computed price (token1 per token0)
    /// * `reserve0` - Raw reserve of token0 (U256)
    /// * `reserve1` - Raw reserve of token1 (U256)
    /// * `reserve0_human` - Human-readable reserve0 (decimal-adjusted)
    /// * `reserve1_human` - Human-readable reserve1 (decimal-adjusted)
    /// * `is_confirmed` - Whether block is finalized
    ///
    /// # Example
    ///
    /// ```no_run
    /// use alloy::primitives::{U256, FixedBytes};
    /// use eth_uniswap_alloy::db::models::PricePointRecord;
    ///
    /// let price_point = PricePointRecord::new(
    ///     1,
    ///     19000000,
    ///     1706745600,
    ///     FixedBytes::from([0u8; 32]),
    ///     3500.0,
    ///     U256::from(1000000000u64),
    ///     U256::from(500000000000000000u64),
    ///     1000.0,
    ///     0.5,
    ///     false,
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pool_id: i64,
        block_number: u64,
        block_timestamp: u64,
        tx_hash: FixedBytes<32>,
        price: f64,
        reserve0: U256,
        reserve1: U256,
        reserve0_human: f64,
        reserve1_human: f64,
        is_confirmed: bool,
    ) -> Self {
        Self {
            id: 0, // Will be set by database
            pool_id,
            block_number: block_number as i64,
            block_timestamp: block_timestamp as i64,
            tx_hash: format!("{:?}", tx_hash),
            price,
            reserve0_raw: reserve0.to_string(),
            reserve1_raw: reserve1.to_string(),
            reserve0_human,
            reserve1_human,
            is_confirmed,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Represents the indexer's persistent state.
///
/// Maps to the `indexer_state` table. Replaces the old state.json file.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IndexerState {
    /// Foreign key to pools table (also serves as PRIMARY KEY)
    pub pool_id: i64,
    /// Last block number successfully indexed
    pub last_indexed_block: i64,
    /// Hash of the last indexed block (for reorg detection)
    pub last_block_hash: String,
    /// Number of chain reorganizations detected
    pub reorg_count: i64,
    /// Total sync events processed (lifetime counter)
    pub total_events_processed: i64,
    /// Unix timestamp of last state update
    pub last_updated_at: i64,
}

impl IndexerState {
    /// Creates a new indexer state.
    pub fn new(
        pool_id: i64,
        last_indexed_block: u64,
        last_block_hash: FixedBytes<32>,
        reorg_count: u64,
        total_events_processed: u64,
    ) -> Self {
        Self {
            pool_id,
            last_indexed_block: last_indexed_block as i64,
            last_block_hash: format!("{:?}", last_block_hash),
            reorg_count: reorg_count as i64,
            total_events_processed: total_events_processed as i64,
            last_updated_at: chrono::Utc::now().timestamp(),
        }
    }

    /// Parses the block hash back to FixedBytes<32>.
    pub fn block_hash(&self) -> Result<FixedBytes<32>, crate::error::TrackerError> {
        self.last_block_hash
            .parse()
            .map_err(|e| crate::error::TrackerError::DecodingError {
                message: format!("Failed to parse block hash: {}", self.last_block_hash),
                source: Some(Box::new(e)),
            })
    }
}

/// Statistics for a pool's price history.
///
/// Used for aggregated queries (min/max/avg prices over a time range).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PriceStats {
    /// Minimum price observed in the range
    pub min_price: f64,
    /// Maximum price observed in the range
    pub max_price: f64,
    /// Average price over the range
    pub avg_price: f64,
    /// Total number of price points in the range
    pub total_points: i64,
}

impl PriceStats {
    /// Creates a new price stats record.
    pub fn new(min_price: f64, max_price: f64, avg_price: f64, total_points: i64) -> Self {
        Self {
            min_price,
            max_price,
            avg_price,
            total_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_event_reserve_conversion() {
        let reserve0 = U256::from(1000000000u64);
        let reserve1 = U256::from(500000000000000000u64);

        let event = SyncEventRecord::new(
            1,
            19000000,
            FixedBytes::from([0u8; 32]),
            1706745600,
            FixedBytes::from([0u8; 32]),
            0,
            reserve0,
            reserve1,
            false,
        );

        assert_eq!(event.reserve0_u256().unwrap(), reserve0);
        assert_eq!(event.reserve1_u256().unwrap(), reserve1);
    }

    #[test]
    fn test_pool_record_creation() {
        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let token0_addr: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap();
        let token1_addr: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse()
            .unwrap();

        let pool = PoolRecord::new(
            pool_addr,
            Some("USDC-WETH".to_string()),
            token0_addr,
            Some("USDC".to_string()),
            6,
            token1_addr,
            Some("WETH".to_string()),
            18,
        );

        assert_eq!(pool.token0_decimals, 6);
        assert_eq!(pool.token1_decimals, 18);
        assert_eq!(pool.name, Some("USDC-WETH".to_string()));
    }

    #[test]
    fn test_indexer_state_block_hash_parsing() {
        let block_hash = FixedBytes::from([1u8; 32]);
        let state = IndexerState::new(1, 19000000, block_hash, 0, 100);

        let parsed_hash = state.block_hash().unwrap();
        assert_eq!(parsed_hash, block_hash);
    }
}

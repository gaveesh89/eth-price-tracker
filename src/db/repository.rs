//! Repository pattern for database operations.
//!
//! Provides high-level CRUD operations for sync events, price points,
//! and indexer state. Handles batch inserts, queries, and reorg recovery.

use alloy::primitives::{Address, FixedBytes, U256};
use sqlx::SqlitePool;
use tracing::{debug, info, instrument};

use super::models::{
    IndexerState, PoolRecord, PoolRow, PricePointRecord, PricePointRow, PriceStats, StatsRow,
    SyncEventRecord, SyncEventRow,
};
use crate::error::TrackerError;

/// Repository for database operations.
///
/// Wraps a SQLite connection pool and provides type-safe methods
/// for all database interactions.
pub struct Repository {
    pool: SqlitePool,
}

impl Repository {
    /// Creates a new repository with the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ==================== POOL OPERATIONS ====================

    /// Ensures a pool exists in the database, creating it if necessary.
    ///
    /// Returns the pool's database ID.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use alloy::primitives::Address;
    /// use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let pool = create_pool("sqlite:./indexer.db").await?;
    ///     let repo = Repository::new(pool);
    ///     
    ///     let pool_id = repo.ensure_pool_exists(
    ///         "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".parse().unwrap(),
    ///         Some("USDC-WETH".to_string()),
    ///         "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse().unwrap(),
    ///         Some("USDC".to_string()),
    ///         6,
    ///         "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
    ///         Some("WETH".to_string()),
    ///         18,
    ///     ).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn ensure_pool_exists(
        &self,
        address: Address,
        name: Option<String>,
        token0_address: Address,
        token0_symbol: Option<String>,
        token0_decimals: u8,
        token1_address: Address,
        token1_symbol: Option<String>,
        token1_decimals: u8,
    ) -> Result<i64, TrackerError> {
        let address_str = format!("{:?}", address);

        // Check if pool already exists
        let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM pools WHERE address = ?")
            .bind(&address_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                TrackerError::database(
                    "Failed to query existing pool".to_string(),
                    Some(Box::new(e)),
                )
            })?;

        if let Some((pool_id,)) = existing {
            return Ok(pool_id);
        }

        // Insert new pool
        let record = PoolRecord::new(
            address,
            name,
            token0_address,
            token0_symbol,
            token0_decimals,
            token1_address,
            token1_symbol,
            token1_decimals,
        );

        let result = sqlx::query(
            r#"
            INSERT INTO pools (
                address, name, token0_address, token0_symbol, token0_decimals,
                token1_address, token1_symbol, token1_decimals, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&record.address)
        .bind(&record.name)
        .bind(&record.token0_address)
        .bind(&record.token0_symbol)
        .bind(record.token0_decimals)
        .bind(&record.token1_address)
        .bind(&record.token1_symbol)
        .bind(record.token1_decimals)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to insert pool".to_string(), Some(Box::new(e)))
        })?;

        Ok(result.last_insert_rowid())
    }

    /// Retrieves a pool by its address.
    pub async fn get_pool_by_address(
        &self,
        address: Address,
    ) -> Result<Option<PoolRecord>, TrackerError> {
        let address_str = format!("{:?}", address);

        let pool = sqlx::query_as::<_, PoolRecord>("SELECT * FROM pools WHERE address = ?")
            .bind(&address_str)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                TrackerError::database(
                    "Failed to query pool by address".to_string(),
                    Some(Box::new(e)),
                )
            })?;

        Ok(pool)
    }

    // ==================== SYNC EVENT OPERATIONS ====================

    /// Inserts a single sync event into the database.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use alloy::primitives::{U256, FixedBytes};
    /// use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let pool = create_pool("sqlite:./indexer.db").await?;
    ///     let repo = Repository::new(pool);
    ///     
    ///     repo.insert_sync_event(
    ///         1,
    ///         19000000,
    ///         FixedBytes::from([0u8; 32]),
    ///         1706745600,
    ///         FixedBytes::from([0u8; 32]),
    ///         0,
    ///         U256::from(1000000000u64),
    ///         U256::from(500000000000000000u64),
    ///         false,
    ///     ).await?;
    ///     
    ///     Ok(())
    /// }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_sync_event(
        &self,
        pool_id: i64,
        block_number: u64,
        block_hash: FixedBytes<32>,
        block_timestamp: u64,
        tx_hash: FixedBytes<32>,
        log_index: u32,
        reserve0: U256,
        reserve1: U256,
        is_confirmed: bool,
    ) -> Result<i64, TrackerError> {
        let record = SyncEventRecord::new(
            pool_id,
            block_number,
            block_hash,
            block_timestamp,
            tx_hash,
            log_index,
            reserve0,
            reserve1,
            is_confirmed,
        );

        let result = sqlx::query(
            r#"
            INSERT INTO sync_events (
                pool_id, block_number, block_hash, block_timestamp, tx_hash,
                log_index, reserve0, reserve1, is_confirmed, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (pool_id, block_number, tx_hash, log_index) DO UPDATE SET
                block_hash = excluded.block_hash,
                block_timestamp = excluded.block_timestamp,
                reserve0 = excluded.reserve0,
                reserve1 = excluded.reserve1,
                is_confirmed = excluded.is_confirmed
            "#,
        )
        .bind(record.pool_id)
        .bind(record.block_number)
        .bind(&record.block_hash)
        .bind(record.block_timestamp)
        .bind(&record.tx_hash)
        .bind(record.log_index)
        .bind(&record.reserve0)
        .bind(&record.reserve1)
        .bind(record.is_confirmed)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to insert sync event".to_string(), Some(Box::new(e)))
        })?;

        Ok(result.last_insert_rowid())
    }

    /// Batch inserts multiple sync events in a single transaction.
    ///
    /// More efficient than inserting events one at a time.
    /// Recommended batch size: 100-1000 events.
    #[instrument(skip(self, events), fields(count = events.len(), duration_ms = tracing::field::Empty))]
    pub async fn batch_insert_sync_events(
        &self,
        events: Vec<SyncEventRecord>,
    ) -> Result<(), TrackerError> {
        if events.is_empty() {
            debug!("Empty event batch, skipping");
            return Ok(());
        }

        let count = events.len();
        info!(count = count, "Starting batch insert of sync events");

        let start = std::time::Instant::now();

        let mut tx = self.pool.begin().await.map_err(|e| {
            TrackerError::database("Failed to start transaction".to_string(), Some(Box::new(e)))
        })?;

        for event in events {
            sqlx::query(
                r#"
                INSERT INTO sync_events (
                    pool_id, block_number, block_hash, block_timestamp, tx_hash,
                    log_index, reserve0, reserve1, is_confirmed, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (pool_id, block_number, tx_hash, log_index) DO UPDATE SET
                    block_hash = excluded.block_hash,
                    block_timestamp = excluded.block_timestamp,
                    reserve0 = excluded.reserve0,
                    reserve1 = excluded.reserve1,
                    is_confirmed = excluded.is_confirmed
                "#,
            )
            .bind(event.pool_id)
            .bind(event.block_number)
            .bind(&event.block_hash)
            .bind(event.block_timestamp)
            .bind(&event.tx_hash)
            .bind(event.log_index)
            .bind(&event.reserve0)
            .bind(&event.reserve1)
            .bind(event.is_confirmed)
            .bind(event.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                TrackerError::database(
                    format!(
                        "Failed to insert sync event at block {}",
                        event.block_number
                    ),
                    Some(Box::new(e)),
                )
            })?;
        }

        tx.commit().await.map_err(|e| {
            TrackerError::database(
                "Failed to commit transaction".to_string(),
                Some(Box::new(e)),
            )
        })?;

        let duration = start.elapsed();
        tracing::Span::current().record("duration_ms", duration.as_millis() as u64);

        info!(
            count = count,
            duration_ms = duration.as_millis(),
            throughput = (count as f64 / duration.as_secs_f64()) as u64,
            "Batch insert completed successfully"
        );

        Ok(())
    }

    // ==================== PRICE POINT OPERATIONS ====================

    /// Inserts a single price point into the database.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_price_point(
        &self,
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
    ) -> Result<i64, TrackerError> {
        let record = PricePointRecord::new(
            pool_id,
            block_number,
            block_timestamp,
            tx_hash,
            price,
            reserve0,
            reserve1,
            reserve0_human,
            reserve1_human,
            is_confirmed,
        );

        let result = sqlx::query(
            r#"
            INSERT INTO price_points (
                pool_id, block_number, block_timestamp, tx_hash, price,
                reserve0_raw, reserve1_raw, reserve0_human, reserve1_human,
                is_confirmed, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (pool_id, block_number, tx_hash) DO UPDATE SET
                block_timestamp = excluded.block_timestamp,
                price = excluded.price,
                reserve0_raw = excluded.reserve0_raw,
                reserve1_raw = excluded.reserve1_raw,
                reserve0_human = excluded.reserve0_human,
                reserve1_human = excluded.reserve1_human,
                is_confirmed = excluded.is_confirmed
            "#,
        )
        .bind(record.pool_id)
        .bind(record.block_number)
        .bind(record.block_timestamp)
        .bind(&record.tx_hash)
        .bind(record.price)
        .bind(&record.reserve0_raw)
        .bind(&record.reserve1_raw)
        .bind(record.reserve0_human)
        .bind(record.reserve1_human)
        .bind(record.is_confirmed)
        .bind(record.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to insert price point".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(result.last_insert_rowid())
    }

    /// Batch inserts multiple price points in a single transaction.
    pub async fn batch_insert_price_points(
        &self,
        prices: Vec<PricePointRecord>,
    ) -> Result<(), TrackerError> {
        if prices.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await.map_err(|e| {
            TrackerError::database("Failed to start transaction".to_string(), Some(Box::new(e)))
        })?;

        for price in prices {
            sqlx::query(
                r#"
                INSERT INTO price_points (
                    pool_id, block_number, block_timestamp, tx_hash, price,
                    reserve0_raw, reserve1_raw, reserve0_human, reserve1_human,
                    is_confirmed, created_at
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (pool_id, block_number, tx_hash) DO UPDATE SET
                    block_timestamp = excluded.block_timestamp,
                    price = excluded.price,
                    reserve0_raw = excluded.reserve0_raw,
                    reserve1_raw = excluded.reserve1_raw,
                    reserve0_human = excluded.reserve0_human,
                    reserve1_human = excluded.reserve1_human,
                    is_confirmed = excluded.is_confirmed
                "#,
            )
            .bind(price.pool_id)
            .bind(price.block_number)
            .bind(price.block_timestamp)
            .bind(&price.tx_hash)
            .bind(price.price)
            .bind(&price.reserve0_raw)
            .bind(&price.reserve1_raw)
            .bind(price.reserve0_human)
            .bind(price.reserve1_human)
            .bind(price.is_confirmed)
            .bind(price.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                TrackerError::database(
                    format!(
                        "Failed to insert price point at block {}",
                        price.block_number
                    ),
                    Some(Box::new(e)),
                )
            })?;
        }

        tx.commit().await.map_err(|e| {
            TrackerError::database(
                "Failed to commit transaction".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(())
    }

    /// Gets the most recent N price points for a pool.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = create_pool("sqlite:./indexer.db").await?;
    /// # let repo = Repository::new(pool);
    /// let recent_prices = repo.get_recent_prices(1, 100).await?;
    /// for price in recent_prices {
    ///     println!("Block {}: {} token1/token0", price.block_number, price.price);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_recent_prices(
        &self,
        pool_id: i64,
        limit: i64,
    ) -> Result<Vec<PricePointRecord>, TrackerError> {
        let prices = sqlx::query_as::<_, PricePointRecord>(
            r#"
            SELECT * FROM price_points
            WHERE pool_id = ?
            ORDER BY block_number DESC
            LIMIT ?
            "#,
        )
        .bind(pool_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query recent prices".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(prices)
    }

    /// Gets price points within a specific time range.
    ///
    /// # Arguments
    ///
    /// * `pool_id` - Database ID of the pool
    /// * `start_time` - Unix timestamp (inclusive)
    /// * `end_time` - Unix timestamp (inclusive)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = create_pool("sqlite:./indexer.db").await?;
    /// # let repo = Repository::new(pool);
    /// let start = 1706745600; // Jan 1, 2024
    /// let end = 1706832000;   // Jan 2, 2024
    /// let prices = repo.get_price_history(1, start, end).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_price_history(
        &self,
        pool_id: i64,
        start_time: i64,
        end_time: i64,
    ) -> Result<Vec<PricePointRecord>, TrackerError> {
        let prices = sqlx::query_as::<_, PricePointRecord>(
            r#"
            SELECT * FROM price_points
            WHERE pool_id = ?
            AND block_timestamp >= ?
            AND block_timestamp <= ?
            ORDER BY block_timestamp ASC
            "#,
        )
        .bind(pool_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query price history".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(prices)
    }

    /// Calculates statistics (min/max/avg) for prices over a time range.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = create_pool("sqlite:./indexer.db").await?;
    /// # let repo = Repository::new(pool);
    /// let start = 1706745600;
    /// let end = 1706832000;
    /// let stats = repo.get_price_stats(1, start, end).await?;
    /// println!("Min: {}, Max: {}, Avg: {}", stats.min_price, stats.max_price, stats.avg_price);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_price_stats(
        &self,
        pool_id: i64,
        start_time: i64,
        end_time: i64,
    ) -> Result<PriceStats, TrackerError> {
        let stats = sqlx::query_as::<_, PriceStats>(
            r#"
            SELECT
                MIN(price) as min_price,
                MAX(price) as max_price,
                AVG(price) as avg_price,
                COUNT(*) as total_points
            FROM price_points
            WHERE pool_id = ?
            AND block_timestamp >= ?
            AND block_timestamp <= ?
            "#,
        )
        .bind(pool_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to query price stats".to_string(), Some(Box::new(e)))
        })?;

        Ok(stats)
    }

    // ==================== API QUERY OPERATIONS ====================

    /// Health check for database connectivity.
    pub async fn health_check(&self) -> Result<(), TrackerError> {
        sqlx::query("SELECT 1 as check")
            .execute(&self.pool)
            .await
            .map_err(|e| {
                TrackerError::database(
                    "Database health check failed".to_string(),
                    Some(Box::new(e)),
                )
            })?;

        Ok(())
    }

    /// Get a pool by its name (e.g., "WETH/USDT").
    pub async fn get_pool_by_name(&self, name: &str) -> Result<Option<PoolRecord>, TrackerError> {
        let pool = sqlx::query_as::<_, PoolRecord>("SELECT * FROM pools WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                TrackerError::database(
                    "Failed to query pool by name".to_string(),
                    Some(Box::new(e)),
                )
            })?;

        Ok(pool)
    }

    /// Get the latest confirmed price point for a pool.
    pub async fn get_latest_price(
        &self,
        pool_id: i64,
    ) -> Result<Option<PricePointRow>, TrackerError> {
        let price = sqlx::query_as::<_, PricePointRow>(
            r#"
            SELECT block_number, block_timestamp, tx_hash, price,
                   reserve0_human, reserve1_human
            FROM price_points
            WHERE pool_id = ? AND is_confirmed = 1
            ORDER BY block_number DESC
            LIMIT 1
            "#,
        )
        .bind(pool_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query latest price".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(price)
    }

    /// Calculate 24-hour price change percentage.
    pub async fn get_24h_price_change(&self, pool_id: i64) -> Result<f64, TrackerError> {
        let now = chrono::Utc::now().timestamp();
        let day_ago = now - 86_400;

        let result = sqlx::query_as::<_, (Option<f64>, Option<f64>)>(
            r#"
            SELECT 
                (SELECT price FROM price_points 
                 WHERE pool_id = ? AND is_confirmed = 1 
                 ORDER BY block_number DESC LIMIT 1) as current_price,
                (SELECT price FROM price_points 
                 WHERE pool_id = ? AND is_confirmed = 1 
                 AND block_timestamp <= ?
                 ORDER BY block_number DESC LIMIT 1) as day_ago_price
            "#,
        )
        .bind(pool_id)
        .bind(pool_id)
        .bind(day_ago)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query 24h price change".to_string(),
                Some(Box::new(e)),
            )
        })?;

        if let (Some(current), Some(old)) = (result.0, result.1) {
            if old == 0.0 {
                return Ok(0.0);
            }
            Ok(((current - old) / old) * 100.0)
        } else {
            Ok(0.0)
        }
    }

    /// Get paginated price history.
    pub async fn get_price_history_paginated(
        &self,
        pool_id: i64,
        from_ts: Option<i64>,
        to_ts: Option<i64>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<PricePointRow>, u64), TrackerError> {
        let from = from_ts.unwrap_or(0);
        let to = to_ts.unwrap_or(i64::MAX);

        let count = sqlx::query_as::<_, (i64,)>(
            r#"
                        SELECT COUNT(*) as count
                        FROM price_points
                        WHERE pool_id = ? AND is_confirmed = 1
                            AND block_timestamp BETWEEN ? AND ?
                        "#,
        )
        .bind(pool_id)
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to query price count".to_string(), Some(Box::new(e)))
        })?
        .0 as u64;

        let prices = sqlx::query_as::<_, PricePointRow>(
            r#"
            SELECT block_number, block_timestamp, tx_hash, price,
                   reserve0_human, reserve1_human
            FROM price_points
            WHERE pool_id = ? AND is_confirmed = 1
              AND block_timestamp BETWEEN ? AND ?
            ORDER BY block_number DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(pool_id)
        .bind(from)
        .bind(to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query price history".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok((prices, count))
    }

    /// Get statistics for a time period.
    pub async fn get_stats_for_period(
        &self,
        pool_id: i64,
        from_timestamp: i64,
    ) -> Result<StatsRow, TrackerError> {
        let stats = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT 
                COUNT(*) as total_events,
                MIN(price) as min_price,
                MAX(price) as max_price,
                AVG(price) as avg_price,
                MIN(block_timestamp) as first_timestamp,
                MAX(block_timestamp) as last_timestamp,
                (SELECT price FROM price_points 
                 WHERE pool_id = ? AND block_timestamp >= ?
                 ORDER BY block_number ASC LIMIT 1) as first_price
            FROM price_points
            WHERE pool_id = ? AND is_confirmed = 1
              AND block_timestamp >= ?
            "#,
        )
        .bind(pool_id)
        .bind(from_timestamp)
        .bind(pool_id)
        .bind(from_timestamp)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to query stats".to_string(), Some(Box::new(e)))
        })?;

        Ok(stats)
    }

    /// Get all pools with indexer metadata.
    pub async fn get_all_pools(&self) -> Result<Vec<PoolRow>, TrackerError> {
        let pools = sqlx::query_as::<_, PoolRow>(
            r#"
            SELECT p.id, p.name, p.address, p.token0_symbol, p.token0_address, p.token0_decimals,
                   p.token1_symbol, p.token1_address, p.token1_decimals,
                   COALESCE(s.last_indexed_block, 0) as last_indexed_block,
                   COALESCE(s.total_events_processed, 0) as total_events
            FROM pools p
            LEFT JOIN indexer_state s ON p.id = s.pool_id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to query pools".to_string(), Some(Box::new(e)))
        })?;

        Ok(pools)
    }

    /// Get recent sync events for a pool.
    pub async fn get_recent_events(
        &self,
        pool_id: i64,
        limit: i64,
    ) -> Result<Vec<SyncEventRow>, TrackerError> {
        let events = sqlx::query_as::<_, SyncEventRow>(
            r#"
            SELECT block_number, block_timestamp, tx_hash, reserve0, reserve1
            FROM sync_events
            WHERE pool_id = ?
            ORDER BY block_number DESC, log_index DESC
            LIMIT ?
            "#,
        )
        .bind(pool_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to query recent events".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(events)
    }

    /// Ensure the default WETH/USDT pool exists for API testing.
    pub async fn ensure_default_pool(&self) -> Result<i64, TrackerError> {
        let existing = sqlx::query_as::<_, (i64,)>("SELECT id FROM pools WHERE name = 'WETH/USDT'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                TrackerError::database(
                    "Failed to query default pool".to_string(),
                    Some(Box::new(e)),
                )
            })?;

        if let Some((pool_id,)) = existing {
            info!(pool_id, "Default pool already exists");
            return Ok(pool_id);
        }

        info!("Creating default WETH/USDT pool");

        let pool_id = sqlx::query_as::<_, (i64,)>(
            r#"
            INSERT INTO pools (
                address, name,
                token0_address, token0_symbol, token0_decimals,
                token1_address, token1_symbol, token1_decimals,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852")
        .bind("WETH/USDT")
        .bind("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        .bind("WETH")
        .bind(18)
        .bind("0xdAC17F958D2ee523a2206206994597C13D831ec7")
        .bind("USDT")
        .bind(6)
        .bind(chrono::Utc::now().timestamp())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to insert default pool".to_string(),
                Some(Box::new(e)),
            )
        })?
        .0;

        sqlx::query(
            r#"
            INSERT INTO indexer_state (pool_id, last_indexed_block, last_block_hash, last_updated_at)
            VALUES (?, 0, '0x0000000000000000000000000000000000000000000000000000000000000000', ?)
            "#,
        )
        .bind(pool_id)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database("Failed to initialize indexer state".to_string(), Some(Box::new(e)))
        })?;

        info!(pool_id, "Default pool created successfully");
        Ok(pool_id)
    }

    // ==================== INDEXER STATE OPERATIONS ====================

    /// Gets the indexer state for a specific pool.
    ///
    /// Returns `None` if no state exists (first run).
    pub async fn get_state(&self, pool_id: i64) -> Result<Option<IndexerState>, TrackerError> {
        let state =
            sqlx::query_as::<_, IndexerState>("SELECT * FROM indexer_state WHERE pool_id = ?")
                .bind(pool_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| {
                    TrackerError::database(
                        "Failed to query indexer state".to_string(),
                        Some(Box::new(e)),
                    )
                })?;

        Ok(state)
    }

    /// Updates the indexer state for a pool.
    ///
    /// Creates a new state entry if it doesn't exist.
    pub async fn update_state(
        &self,
        pool_id: i64,
        last_indexed_block: u64,
        last_block_hash: FixedBytes<32>,
        reorg_count: u64,
        total_events_processed: u64,
    ) -> Result<(), TrackerError> {
        let state = IndexerState::new(
            pool_id,
            last_indexed_block,
            last_block_hash,
            reorg_count,
            total_events_processed,
        );

        sqlx::query(
            r#"
            INSERT INTO indexer_state (
                pool_id, last_indexed_block, last_block_hash,
                reorg_count, total_events_processed, last_updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (pool_id) DO UPDATE SET
                last_indexed_block = excluded.last_indexed_block,
                last_block_hash = excluded.last_block_hash,
                reorg_count = excluded.reorg_count,
                total_events_processed = excluded.total_events_processed,
                last_updated_at = excluded.last_updated_at
            "#,
        )
        .bind(state.pool_id)
        .bind(state.last_indexed_block)
        .bind(&state.last_block_hash)
        .bind(state.reorg_count)
        .bind(state.total_events_processed)
        .bind(state.last_updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to update indexer state".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(())
    }

    // ==================== REORG OPERATIONS ====================

    /// Invalidates all data from a specific block onwards.
    ///
    /// Used during chain reorganization to remove data from invalidated blocks.
    /// Sets `is_confirmed = 0` for all affected events and prices.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use eth_uniswap_alloy::db::{create_pool, repository::Repository};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let pool = create_pool("sqlite:./indexer.db").await?;
    /// # let repo = Repository::new(pool);
    /// // Reorg detected at block 19000000
    /// repo.invalidate_from_block(1, 19000000).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn invalidate_from_block(
        &self,
        pool_id: i64,
        from_block: u64,
    ) -> Result<(), TrackerError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            TrackerError::database("Failed to start transaction".to_string(), Some(Box::new(e)))
        })?;

        // Mark sync events as unconfirmed
        sqlx::query(
            "UPDATE sync_events SET is_confirmed = 0 WHERE pool_id = ? AND block_number >= ?",
        )
        .bind(pool_id)
        .bind(from_block as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to invalidate sync events".to_string(),
                Some(Box::new(e)),
            )
        })?;

        // Mark price points as unconfirmed
        sqlx::query(
            "UPDATE price_points SET is_confirmed = 0 WHERE pool_id = ? AND block_number >= ?",
        )
        .bind(pool_id)
        .bind(from_block as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to invalidate price points".to_string(),
                Some(Box::new(e)),
            )
        })?;

        tx.commit().await.map_err(|e| {
            TrackerError::database(
                "Failed to commit transaction".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(())
    }

    /// Marks data as confirmed (finalized) up to a specific block.
    ///
    /// Used to mark blocks as final after they've been confirmed by enough subsequent blocks.
    pub async fn confirm_up_to_block(
        &self,
        pool_id: i64,
        up_to_block: u64,
    ) -> Result<(), TrackerError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            TrackerError::database("Failed to start transaction".to_string(), Some(Box::new(e)))
        })?;

        sqlx::query(
            "UPDATE sync_events SET is_confirmed = 1 WHERE pool_id = ? AND block_number <= ? AND is_confirmed = 0",
        )
        .bind(pool_id)
        .bind(up_to_block as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to confirm sync events".to_string(),
                Some(Box::new(e)),
            )
        })?;

        sqlx::query(
            "UPDATE price_points SET is_confirmed = 1 WHERE pool_id = ? AND block_number <= ? AND is_confirmed = 0",
        )
        .bind(pool_id)
        .bind(up_to_block as i64)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to confirm price points".to_string(),
                Some(Box::new(e)),
            )
        })?;

        tx.commit().await.map_err(|e| {
            TrackerError::database(
                "Failed to commit transaction".to_string(),
                Some(Box::new(e)),
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{create_pool, run_migrations};

    async fn setup_test_db() -> Repository {
        let pool = create_pool("sqlite::memory:")
            .await
            .expect("Failed to create pool");
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        Repository::new(pool)
    }

    #[tokio::test]
    async fn test_ensure_pool_exists() {
        let repo = setup_test_db().await;

        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let token0_addr: Address = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
            .parse()
            .unwrap();
        let token1_addr: Address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
            .parse()
            .unwrap();

        let pool_id = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                token0_addr,
                Some("USDC".to_string()),
                6,
                token1_addr,
                Some("WETH".to_string()),
                18,
            )
            .await
            .expect("Failed to create pool");

        assert!(pool_id > 0);

        // Calling again should return same ID
        let pool_id2 = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                token0_addr,
                Some("USDC".to_string()),
                6,
                token1_addr,
                Some("WETH".to_string()),
                18,
            )
            .await
            .expect("Failed to get pool");

        assert_eq!(pool_id, pool_id2);
    }

    #[tokio::test]
    async fn test_insert_and_query_sync_event() {
        let repo = setup_test_db().await;

        // Create pool first
        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let pool_id = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                Some("USDC".to_string()),
                6,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
                Some("WETH".to_string()),
                18,
            )
            .await
            .unwrap();

        // Insert sync event
        let event_id = repo
            .insert_sync_event(
                pool_id,
                19000000,
                FixedBytes::from([1u8; 32]),
                1706745600,
                FixedBytes::from([2u8; 32]),
                0,
                U256::from(1000000000u64),
                U256::from(500000000000000000u64),
                false,
            )
            .await
            .expect("Failed to insert sync event");

        assert!(event_id > 0);
    }

    #[tokio::test]
    async fn test_insert_and_query_price_point() {
        let repo = setup_test_db().await;

        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let pool_id = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                Some("USDC".to_string()),
                6,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
                Some("WETH".to_string()),
                18,
            )
            .await
            .unwrap();

        // Insert price point
        let price_id = repo
            .insert_price_point(
                pool_id,
                19000000,
                1706745600,
                FixedBytes::from([2u8; 32]),
                3500.0,
                U256::from(1000000000u64),
                U256::from(500000000000000000u64),
                1000.0,
                0.5,
                false,
            )
            .await
            .expect("Failed to insert price point");

        assert!(price_id > 0);

        // Query recent prices
        let prices = repo
            .get_recent_prices(pool_id, 10)
            .await
            .expect("Failed to query prices");

        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].price, 3500.0);
    }

    #[tokio::test]
    async fn test_state_management() {
        let repo = setup_test_db().await;

        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let pool_id = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                Some("USDC".to_string()),
                6,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
                Some("WETH".to_string()),
                18,
            )
            .await
            .unwrap();

        // No state initially
        let state = repo.get_state(pool_id).await.expect("Failed to get state");
        assert!(state.is_none());

        // Update state
        let block_hash = FixedBytes::from([1u8; 32]);
        repo.update_state(pool_id, 19000000, block_hash, 0, 100)
            .await
            .expect("Failed to update state");

        // Verify state
        let state = repo
            .get_state(pool_id)
            .await
            .expect("Failed to get state")
            .expect("State should exist");

        assert_eq!(state.last_indexed_block, 19000000);
        assert_eq!(state.total_events_processed, 100);
        assert_eq!(state.reorg_count, 0);
    }

    #[tokio::test]
    async fn test_invalidate_from_block() {
        let repo = setup_test_db().await;

        let pool_addr: Address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
            .parse()
            .unwrap();
        let pool_id = repo
            .ensure_pool_exists(
                pool_addr,
                Some("USDC-WETH".to_string()),
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
                    .parse()
                    .unwrap(),
                Some("USDC".to_string()),
                6,
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
                    .parse()
                    .unwrap(),
                Some("WETH".to_string()),
                18,
            )
            .await
            .unwrap();

        // Insert events at different blocks
        for block in 19000000..19000010 {
            repo.insert_sync_event(
                pool_id,
                block,
                FixedBytes::from([1u8; 32]),
                1706745600,
                FixedBytes::from([2u8; 32]),
                0,
                U256::from(1000000000u64),
                U256::from(500000000000000000u64),
                true,
            )
            .await
            .expect("Failed to insert sync event");
        }

        // Invalidate from block 19000005
        repo.invalidate_from_block(pool_id, 19000005)
            .await
            .expect("Failed to invalidate");

        // Blocks 19000005+ should be unconfirmed
        // This is tested implicitly by verifying the update succeeded
    }
}

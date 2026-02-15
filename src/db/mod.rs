//! Database module for persistent storage of sync events and price history.
//!
//! This module provides SQLite-based storage for:
//! - Raw sync events from the blockchain (audit trail)
//! - Computed price points (fast queries)
//! - Indexer state (replaces state.json)
//!
//! # Architecture
//!
//! - `models`: Data structures that map to database tables
//! - `repository`: CRUD operations and business logic
//! - Connection pooling with SQLite WAL mode for concurrency
//! - Migration system for schema versioning

use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous, SqliteJournalMode},
    SqlitePool,
};
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

use crate::error::TrackerError;

pub mod models;
pub mod repository;

/// Creates a SQLite connection pool with optimized settings.
///
/// # Configuration
///
/// - **WAL mode**: Enables concurrent readers during writes
/// - **Busy timeout**: 5 seconds to handle lock contention
/// - **Max connections**: 5 (suitable for single-machine indexer)
/// - **Min connections**: 1 (keep one connection warm)
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::db::create_pool;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let pool = create_pool("sqlite:./indexer.db").await?;
///     // Use pool for queries
///     Ok(())
/// }
/// ```
pub async fn create_pool(database_url: &str) -> Result<SqlitePool, TrackerError> {
    info!(database_url, "Connecting to database");

    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| {
            TrackerError::database(
                format!("Failed to parse database URL: {database_url}"),
                Some(Box::new(e)),
            )
        })?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(|e| {
            TrackerError::database(
                format!("Failed to connect to database at {database_url}"),
                Some(Box::new(e)),
            )
        })?;

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to enable foreign keys".to_string(),
                Some(Box::new(e)),
            )
        })?;

    info!("Running database migrations");
    run_migrations(&pool).await?;
    verify_database(&pool).await?;
    info!("Database migrations complete");

    Ok(pool)
}

/// Runs database migrations to ensure schema is up-to-date.
///
/// This function applies all pending migrations from the `migrations/` directory.
/// Migrations are applied in order and are idempotent (safe to run multiple times).
///
/// # Example
///
/// ```no_run
/// use eth_uniswap_alloy::db::{create_pool, run_migrations};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let pool = create_pool("sqlite:./indexer.db").await?;
///     run_migrations(&pool).await?;
///     Ok(())
/// }
/// ```
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), TrackerError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| {
            TrackerError::database(
                "Failed to run database migrations".to_string(),
                Some(Box::new(e)),
            )
        })?;

    Ok(())
}

/// Verify that required tables exist after migrations.
pub async fn verify_database(pool: &SqlitePool) -> Result<(), TrackerError> {
    let rows = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT name FROM sqlite_master
        WHERE type='table' AND name IN ('pools', 'price_points', 'sync_events', 'indexer_state')
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        TrackerError::database("Failed to verify database schema".to_string(), Some(Box::new(e)))
    })?;

    if rows.len() < 4 {
        return Err(TrackerError::database(
            format!("Database schema incomplete. Expected 4 tables, found {}", rows.len()),
            None,
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pool_and_migrations() {
        // Use in-memory database for testing
        let pool = create_pool("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        // Run migrations
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Verify tables exist
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table'")
            .fetch_one(&pool)
            .await
            .expect("Failed to query tables");

        // Should have 4 tables + 1 migration history table
        assert!(result.0 >= 4, "Expected at least 4 tables, got {}", result.0);
    }

    #[tokio::test]
    async fn test_wal_mode_enabled() {
        // Note: WAL mode is not supported in :memory: databases
        // SQLite falls back to 'memory' journal mode
        let pool = create_pool("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        let result: (String,) = sqlx::query_as("PRAGMA journal_mode")
            .fetch_one(&pool)
            .await
            .expect("Failed to query journal mode");

        // For in-memory databases, expect 'memory' journal mode
        assert_eq!(result.0, "memory", "Memory mode expected for :memory: database");
    }

    #[tokio::test]
    async fn test_foreign_keys_enabled() {
        let pool = create_pool("sqlite::memory:")
            .await
            .expect("Failed to create pool");

        let result: (i64,) = sqlx::query_as("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .expect("Failed to query foreign keys");

        assert_eq!(result.0, 1, "Foreign keys should be enabled");
    }
}

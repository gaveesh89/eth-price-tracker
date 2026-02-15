-- Initial database schema for Ethereum event indexer
-- Version: 001
-- Description: Creates tables for pools, sync events, price points, and indexer state

-- =============================================================================
-- POOLS TABLE
-- =============================================================================
-- Stores Uniswap V2 pool information
-- Support for multiple pools (future extensibility)
CREATE TABLE pools (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    address TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    token0_address TEXT NOT NULL,
    token0_symbol TEXT NOT NULL,
    token0_decimals INTEGER NOT NULL,
    token1_address TEXT NOT NULL,
    token1_symbol TEXT NOT NULL,
    token1_decimals INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- =============================================================================
-- SYNC EVENTS TABLE
-- =============================================================================
-- Raw blockchain event data for complete audit trail
-- Stores every Sync event with full context
CREATE TABLE sync_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pool_id INTEGER NOT NULL,
    block_number INTEGER NOT NULL,
    block_hash TEXT NOT NULL,
    block_timestamp INTEGER NOT NULL,
    tx_hash TEXT NOT NULL,
    log_index INTEGER NOT NULL,
    reserve0 TEXT NOT NULL,  -- TEXT to preserve U256 precision (SQLite INTEGER is only 64-bit)
    reserve1 TEXT NOT NULL,  -- TEXT to preserve U256 precision
    is_confirmed BOOLEAN NOT NULL DEFAULT 0,  -- 0 = unconfirmed, 1 = finalized (for reorg safety)
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (pool_id) REFERENCES pools(id) ON DELETE CASCADE,
    UNIQUE(pool_id, block_number, tx_hash, log_index)  -- Prevent duplicates during reorg reprocessing
);

-- =============================================================================
-- PRICE POINTS TABLE
-- =============================================================================
-- Computed price data for fast queries and analytics
-- Denormalized for query performance
CREATE TABLE price_points (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pool_id INTEGER NOT NULL,
    block_number INTEGER NOT NULL,
    block_timestamp INTEGER NOT NULL,
    tx_hash TEXT NOT NULL,
    price REAL NOT NULL,  -- ETH/USDT price as float for easy querying
    reserve0_raw TEXT NOT NULL,  -- U256 as string for precision
    reserve1_raw TEXT NOT NULL,  -- U256 as string for precision
    reserve0_human REAL NOT NULL,  -- Human-readable reserve (with decimals applied)
    reserve1_human REAL NOT NULL,  -- Human-readable reserve (with decimals applied)
    is_confirmed BOOLEAN NOT NULL DEFAULT 0,  -- 0 = unconfirmed, 1 = finalized
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (pool_id) REFERENCES pools(id) ON DELETE CASCADE,
    UNIQUE(pool_id, block_number, tx_hash)  -- One price per transaction (may have multiple Sync events)
);

-- =============================================================================
-- INDEXER STATE TABLE
-- =============================================================================
-- Replaces state.json - tracks indexing progress per pool
-- One row per pool
CREATE TABLE indexer_state (
    pool_id INTEGER PRIMARY KEY,
    last_indexed_block INTEGER NOT NULL,
    last_block_hash TEXT NOT NULL,
    reorg_count INTEGER NOT NULL DEFAULT 0,
    total_events_processed INTEGER NOT NULL DEFAULT 0,
    last_updated_at INTEGER NOT NULL,
    FOREIGN KEY (pool_id) REFERENCES pools(id) ON DELETE CASCADE
);

-- =============================================================================
-- PERFORMANCE INDEXES
-- =============================================================================
-- Critical for query speed - without these, queries will be slow on large datasets

-- Sync events indexes
CREATE INDEX idx_sync_events_block ON sync_events(block_number);
CREATE INDEX idx_sync_events_timestamp ON sync_events(block_timestamp);
CREATE INDEX idx_sync_events_pool ON sync_events(pool_id);
CREATE INDEX idx_sync_events_confirmed ON sync_events(is_confirmed);
CREATE INDEX idx_sync_events_pool_block ON sync_events(pool_id, block_number);  -- Composite for reorg queries

-- Price points indexes
CREATE INDEX idx_price_points_block ON price_points(block_number);
CREATE INDEX idx_price_points_timestamp ON price_points(block_timestamp);
CREATE INDEX idx_price_points_pool ON price_points(pool_id);
CREATE INDEX idx_price_points_confirmed ON price_points(is_confirmed);
CREATE INDEX idx_price_points_pool_block ON price_points(pool_id, block_number);  -- Composite for reorg queries
CREATE INDEX idx_price_points_pool_timestamp ON price_points(pool_id, block_timestamp);  -- For time-range queries

-- =============================================================================
-- SCHEMA NOTES
-- =============================================================================
-- 
-- 1. is_confirmed field:
--    - Set to 0 when first indexed
--    - Set to 1 only after block is behind finalized checkpoint (64+ confirmations)
--    - Production queries should filter on is_confirmed = 1
--    - Unconfirmed data may be deleted during reorg
--
-- 2. TEXT for U256 values:
--    - SQLite INTEGER is only 64-bit (max ~9.2 quintillion)
--    - Ethereum U256 can be up to 115 quattuorvigintillion
--    - Store as TEXT to preserve full precision
--    - Convert to/from U256 in Rust code
--
-- 3. Unique constraints:
--    - Prevent duplicate inserts during reorg reprocessing
--    - Use ON CONFLICT DO NOTHING in INSERT statements
--    - Ensures idempotent operations
--
-- 4. ON DELETE CASCADE:
--    - If a pool is deleted, all related data is automatically removed
--    - Maintains referential integrity
--    - Simplifies cleanup operations
--
-- 5. Timestamps:
--    - created_at: When row was inserted into DB (for audit)
--    - block_timestamp: Unix timestamp from blockchain
--    - last_updated_at: Last time state was updated
--
-- 6. Indexes:
--    - Single-column indexes for simple lookups
--    - Composite indexes for complex queries (pool_id + block_number)
--    - Index on is_confirmed for filtering finalized data
--
-- 7. Future migrations:
--    - Add 002_add_analytics_tables.sql for aggregated stats
--    - Add 003_add_token_metadata.sql for extended token info
--    - Schema is designed for extensibility
--
-- =============================================================================

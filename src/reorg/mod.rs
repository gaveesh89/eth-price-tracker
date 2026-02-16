//! Chain reorganization detection and handling.
//!
//! This module provides robust reorg detection for Ethereum indexing by:
//! - Tracking block hashes to detect chain reorganizations
//! - Binary search to efficiently find fork points
//! - State invalidation and re-indexing from fork points
//! - Metrics tracking for reorg occurrences
//!
//! ## How It Works
//!
//! 1. **Block Hash Chain**: Store each indexed block's hash alongside its number
//! 2. **Parent Hash Verification**: On each new block, verify its parent hash matches our last known hash
//! 3. **Fork Point Detection**: If mismatch detected, binary search to find exact fork point
//! 4. **Rewind and Reprocess**: Invalidate data from fork point forward and re-index
//!
//! ## Reorg Frequency on Ethereum
//!
//! - 1-block reorgs: Several times per day
//! - 2-3 block reorgs: Multiple times per week
//! - 4-7 block reorgs: Weekly to monthly
//! - Finalized blocks (2 epochs = ~12.8 min): Considered permanent
//!
//! ## Example
//!
//! ```rust,ignore
//! use eth_uniswap_alloy::reorg::{ReorgDetector, BlockRecord};
//! use alloy::providers::Provider;
//!
//! # async fn example<P: Provider>(provider: &P) -> eyre::Result<()> {
//! let mut detector = ReorgDetector::new();
//!
//! // Track a new block
//! let block = provider.get_block_by_number(19_000_000u64.into(), false).await?.unwrap();
//! let record = BlockRecord::from_block(&block);
//! detector.add_block(record);
//!
//! // Later, check for reorg
//! if let Some(fork_point) = detector.detect_reorg(provider, 19_000_005).await? {
//!     println!("Reorg detected! Fork point at block {}", fork_point);
//!     // Invalidate data from fork_point forward
//! }
//! # Ok(())
//! # }
//! ```

pub mod detector;

pub use detector::{BlockRecord, ReorgDetector};

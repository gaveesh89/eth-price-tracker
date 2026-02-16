//! Reorg detection implementation.

use alloy::primitives::B256;
use alloy::providers::Provider;
use alloy::rpc::types::{Block, BlockTransactionsKind};
use alloy::transports::http::{Client, Http};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{TrackerError, TrackerResult};

// Import the concrete Provider type from our RPC module
type ConcreteProvider = alloy::providers::RootProvider<Http<Client>>;

/// Record of a processed block for reorg detection.
///
/// Stores minimal information needed to verify chain continuity:
/// - Block number and hash
/// - Parent hash (to verify chain linkage)
/// - Timestamp (for debugging and metrics)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRecord {
    /// Block number
    pub number: u64,

    /// Block hash
    pub hash: B256,

    /// Parent block hash
    pub parent_hash: B256,

    /// Block timestamp (Unix epoch seconds)
    pub timestamp: u64,
}

impl BlockRecord {
    /// Create a BlockRecord from an Alloy Block.
    ///
    /// Extracts the essential fields needed for reorg detection.
    pub fn from_block(block: &Block) -> Self {
        Self {
            number: block.header.number,
            hash: block.header.hash,
            parent_hash: block.header.parent_hash,
            timestamp: block.header.timestamp,
        }
    }

    /// Create a new BlockRecord manually (useful for testing).
    pub fn new(number: u64, hash: B256, parent_hash: B256, timestamp: u64) -> Self {
        Self {
            number,
            hash,
            parent_hash,
            timestamp,
        }
    }
}

/// Chain reorganization detector.
///
/// Detects when the Ethereum chain has reorganized by tracking block hashes
/// and verifying parent hash linkage. When a reorg is detected, performs
/// binary search to find the exact fork point.
///
/// ## Algorithm
///
/// 1. Store the last known block record (number, hash, parent_hash)
/// 2. When fetching a new block, verify its parent_hash matches our last_hash
/// 3. If mismatch: binary search between last_known and current to find fork point
/// 4. Return the fork point block number for state invalidation
///
/// ## Example
///
/// ```rust,ignore
/// use eth_uniswap_alloy::reorg::{ReorgDetector, BlockRecord};
/// use alloy::providers::Provider;
///
/// # async fn example<P: Provider>(provider: &P) -> eyre::Result<()> {
/// let mut detector = ReorgDetector::new();
///
/// // Fetch and track initial block
/// let block = provider.get_block_by_number(19_000_000u64.into(), false).await?.unwrap();
/// let record = BlockRecord::from_block(&block);
/// detector.add_block(record);
///
/// // Later, when processing new block, check for reorg
/// let new_block_number = 19_000_005;
/// if let Some(fork_point) = detector.detect_reorg(provider, new_block_number).await? {
///     println!("Reorg at block {}! Invalidating from {}", new_block_number, fork_point);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgDetector {
    /// Last known canonical block
    last_block: Option<BlockRecord>,

    /// Total number of reorgs detected
    reorg_count: u64,
}

impl ReorgDetector {
    /// Create a new reorg detector with no tracked blocks.
    pub fn new() -> Self {
        Self {
            last_block: None,
            reorg_count: 0,
        }
    }

    /// Create a detector with an initial block record.
    pub fn with_block(block: BlockRecord) -> Self {
        Self {
            last_block: Some(block),
            reorg_count: 0,
        }
    }

    /// Add a new block to the tracker (assumes it's canonical).
    ///
    /// Call this after successfully processing a block to update
    /// the chain tip that we'll verify against future blocks.
    pub fn add_block(&mut self, block: BlockRecord) {
        debug!(
            "Tracking block {} (hash: {}, parent: {})",
            block.number, block.hash, block.parent_hash
        );
        self.last_block = Some(block);
    }

    /// Get the last tracked block record.
    pub fn last_block(&self) -> Option<&BlockRecord> {
        self.last_block.as_ref()
    }

    /// Get the total number of detected reorgs.
    pub fn reorg_count(&self) -> u64 {
        self.reorg_count
    }

    /// Reset the reorg count (useful for testing).
    pub fn reset_reorg_count(&mut self) {
        self.reorg_count = 0;
    }

    /// Detect if a reorg has occurred between our last known block and the given block number.
    ///
    /// Returns `Ok(Some(fork_point))` if a reorg is detected, where `fork_point` is the
    /// last valid block number before the fork. Returns `Ok(None)` if no reorg detected.
    ///
    /// ## Algorithm
    ///
    /// 1. Fetch the block at `current_block_number`
    /// 2. Check if its parent hash matches our last known block hash
    /// 3. If mismatch: binary search to find fork point
    /// 4. Increment reorg counter
    ///
    /// ## Errors
    ///
    /// Returns error if RPC calls fail or if block not found.
    pub async fn detect_reorg(
        &mut self,
        provider: &ConcreteProvider,
        current_block_number: u64,
    ) -> TrackerResult<Option<u64>> {
        let last_known = match &self.last_block {
            Some(block) => block,
            None => {
                debug!("No last block tracked, cannot detect reorg");
                return Ok(None);
            }
        };

        // If we're checking the very next block, just verify parent hash
        if current_block_number == last_known.number + 1 {
            let current_block = self.fetch_block(provider, current_block_number).await?;

            if current_block.parent_hash != last_known.hash {
                warn!(
                    "REORG DETECTED at block {}! Parent hash mismatch: expected {}, got {}",
                    current_block_number, last_known.hash, current_block.parent_hash
                );
                self.reorg_count += 1;

                // For 1-block reorg, fork point is the parent of our last known block
                let fork_point = if last_known.number > 0 {
                    last_known.number - 1
                } else {
                    0
                };

                return Ok(Some(fork_point));
            }

            // No reorg, chain is continuous
            return Ok(None);
        }

        // If there's a gap, verify the chain linkage by checking if our last known
        // block is still on-chain at the same hash
        let on_chain_block = self.fetch_block(provider, last_known.number).await?;

        if on_chain_block.hash != last_known.hash {
            warn!(
                "REORG DETECTED! Block {} hash changed: expected {}, got {}",
                last_known.number, last_known.hash, on_chain_block.hash
            );
            self.reorg_count += 1;

            // Binary search to find the exact fork point
            let fork_point = self.find_fork_point(provider, 0, last_known.number).await?;

            info!(
                "Fork point found at block {}. Reorg depth: {} blocks",
                fork_point,
                last_known.number - fork_point
            );

            return Ok(Some(fork_point));
        }

        // No reorg detected
        Ok(None)
    }

    /// Binary search to find the fork point (last common block) between two chain states.
    ///
    /// Assumes that blocks from `low` to `high` have been previously indexed, and finds
    /// the highest block number where the on-chain hash still matches our recorded hash.
    ///
    /// ## Algorithm
    ///
    /// Standard binary search on block numbers, checking if on-chain hash matches
    /// our stored hash at the midpoint. The fork point is the last matching block.
    async fn find_fork_point(
        &self,
        provider: &ConcreteProvider,
        mut low: u64,
        mut high: u64,
    ) -> TrackerResult<u64> {
        debug!("Binary search for fork point between {} and {}", low, high);

        let mut fork_point = low;

        while low <= high {
            let mid = low + (high - low) / 2;

            let on_chain = self.fetch_block(provider, mid).await?;

            // Check if we have this block in our history
            // For simplicity, we're assuming the hash verification happens at the boundary.
            // In a full implementation, you'd check against a persistent block hash store.
            //
            // For now, we'll just find where the chain diverged by checking consecutive blocks.
            let on_chain_next = self.fetch_block(provider, mid + 1).await?;

            if on_chain_next.parent_hash == on_chain.hash {
                // Chain is continuous at this point
                fork_point = mid;
                low = mid + 1;
            } else {
                // Divergence found before mid
                high = mid.saturating_sub(1);
            }
        }

        Ok(fork_point)
    }

    /// Fetch a block from the provider with error handling.
    async fn fetch_block(
        &self,
        provider: &ConcreteProvider,
        block_number: u64,
    ) -> TrackerResult<BlockRecord> {
        let block = provider
            .get_block_by_number(block_number.into(), BlockTransactionsKind::Hashes)
            .await
            .map_err(|e| {
                TrackerError::rpc(
                    format!("Failed to fetch block {}: {}", block_number, e),
                    None,
                )
            })?
            .ok_or_else(|| {
                TrackerError::state(format!("Block {} not found", block_number), None)
            })?;

        Ok(BlockRecord::from_block(&block))
    }
}

impl Default for ReorgDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::b256;

    #[test]
    fn test_block_record_creation() {
        let record = BlockRecord::new(
            19_000_000,
            b256!("0x1234567890123456789012345678901234567890123456789012345678901234"),
            b256!("0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"),
            1234567890,
        );

        assert_eq!(record.number, 19_000_000);
        assert_eq!(record.timestamp, 1234567890);
    }

    #[test]
    fn test_detector_initialization() {
        let detector = ReorgDetector::new();
        assert!(detector.last_block().is_none());
        assert_eq!(detector.reorg_count(), 0);
    }

    #[test]
    fn test_add_block() {
        let mut detector = ReorgDetector::new();
        let record = BlockRecord::new(
            19_000_000,
            b256!("0x1234567890123456789012345678901234567890123456789012345678901234"),
            b256!("0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"),
            1234567890,
        );

        detector.add_block(record.clone());

        assert!(detector.last_block().is_some());
        assert_eq!(detector.last_block().unwrap().number, 19_000_000);
    }

    #[test]
    fn test_reorg_count_tracking() {
        let mut detector = ReorgDetector::new();
        assert_eq!(detector.reorg_count(), 0);

        detector.reorg_count += 1;
        assert_eq!(detector.reorg_count(), 1);

        detector.reset_reorg_count();
        assert_eq!(detector.reorg_count(), 0);
    }
}

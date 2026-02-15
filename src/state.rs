//! State management for tracking Uniswap V2 pair reserves.
//!
//! This module provides atomic state tracking for the WETH/USDT pair reserves
//! with proper validation and type safety.
//!
//! ## Design
//!
//! The `State` struct maintains:
//! - Current WETH and USDT reserves (as separate fields, not generic reserve0/1)
//! - Last processed block number for incremental updates
//! - Validation logic to ensure reserves are within reasonable bounds
//!
//! ## Token Ordering
//!
//! Uniswap V2 pairs order tokens by address (lexicographically as bytes).
//! For WETH/USDT:
//! - WETH address: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
//! - USDT address: `0xdAC17F958D2ee523a2206206994597C13D831ec7`
//!
//! Since `0xC0... < 0xdA...`, WETH is `token0` and USDT is `token1`.
//! Therefore:
//! - `reserve0` from Sync events = WETH reserves
//! - `reserve1` from Sync events = USDT reserves
//!
//! ## Example
//!
//! ```
//! use eth_uniswap_alloy::state::State;
//! use eth_uniswap_alloy::events::Sync;
//! use alloy::primitives::Uint;
//!
//! # fn example() -> eyre::Result<()> {
//! let mut state = State::new();
//!
//! // Create mock Sync event
//! let sync_event = Sync {
//!     reserve0: Uint::<112, 2>::from(1_000_000),  // WETH
//!     reserve1: Uint::<112, 2>::from(2_000_000),  // USDT
//! };
//!
//! // Update state with event data
//! state.update_from_sync_event(&sync_event, 19_000_000)?;
//!
//! // Access reserves
//! let (weth, usdt) = state.get_reserves();
//! assert_eq!(weth, alloy::primitives::U256::from(1_000_000));
//! assert_eq!(usdt, alloy::primitives::U256::from(2_000_000));
//! # Ok(())
//! # }
//! ```
//!
//! ## Reorg Safety
//!
//! State now includes block hash tracking for chain reorganization detection:
//! - `last_block_hash`: Hash of the last processed block for chain continuity verification
//! - `reorg_count`: Total number of reorgs detected and handled
//!
//! See the [`reorg`](crate::reorg) module for reorg detection implementation.

use alloy::primitives::{B256, U256};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::error::{TrackerError, TrackerResult};
use crate::events::Sync;

/// Maximum reasonable reserve value (to catch obviously wrong data).
///
/// Set to 10^30 (well above any realistic reserve amount for WETH or USDT).
/// This helps catch data corruption or decoding errors.
const MAX_RESERVE_VALUE: u128 = 1_000_000_000_000_000_000_000_000_000_000; // 10^30

/// State tracker for Uniswap V2 WETH/USDT pair reserves.
///
/// Maintains current reserves and last processed block number with
/// atomic updates and validation.
///
/// ## Fields
///
/// - `weth_reserve`: Current WETH reserve (corresponds to reserve0 in Sync events)
/// - `usdt_reserve`: Current USDT reserve (corresponds to reserve1 in Sync events)
/// - `last_block`: Last block number where reserves were updated
/// - `last_block_hash`: Block hash for reorg detection (chain continuity verification)
/// - `reorg_count`: Total number of chain reorganizations detected
///
/// ## Thread Safety
///
/// This struct is not thread-safe by default. If using across threads,
/// wrap in `Arc<Mutex<State>>` or similar synchronization primitive.
///
/// ## Reorg Detection
///
/// The `last_block_hash` field enables detection of chain reorganizations by
/// verifying that each new block's parent hash matches the last known block hash.
/// When a mismatch is detected, the [`reorg`](crate::reorg) module handles
/// invalidation and re-indexing from the fork point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// WETH reserve amount (token0 in the pair)
    weth_reserve: U256,

    /// USDT reserve amount (token1 in the pair)
    usdt_reserve: U256,

    /// Last block number where state was updated
    last_block: u64,

    /// Hash of the last processed block (for reorg detection)
    #[serde(default)]
    last_block_hash: Option<B256>,

    /// Total number of chain reorganizations detected and handled
    #[serde(default)]
    reorg_count: u64,
}

impl State {
    /// Create a new empty state.
    ///
    /// Initializes with zero reserves and block number 0.
    /// Use `update_from_sync_event()` to populate with actual data.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let state = State::new();
    /// assert_eq!(state.get_last_block(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        debug!("Initializing new State");
        Self {
            weth_reserve: U256::ZERO,
            usdt_reserve: U256::ZERO,
            last_block: 0,
            last_block_hash: None,
            reorg_count: 0,
        }
    }

    /// Update state from a Sync event.
    ///
    /// This method atomically updates both reserves and the block number
    /// after validating the reserve values are within reasonable bounds.
    ///
    /// ## Validation
    ///
    /// Ensures reserves are:
    /// - Non-zero (pools with zero reserves are invalid)
    /// - Below maximum threshold (catches overflow/corruption)
    /// - Properly ordered (WETH = reserve0, USDT = reserve1)
    ///
    /// ## Arguments
    ///
    /// * `event` - The decoded Sync event from the blockchain
    /// * `block_number` - The block number where this event occurred
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - Either reserve is zero
    /// - Either reserve exceeds maximum threshold
    /// - Block number is less than last processed block (reorg detection)
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    /// use eth_uniswap_alloy::events::Sync;
    /// # use eth_uniswap_alloy::error::TrackerResult;
    /// # use alloy::primitives::Uint;
    ///
    /// # fn example() -> TrackerResult<()> {
    /// let mut state = State::new();
    ///
    /// let sync = Sync {
    ///     reserve0: Uint::<112, 2>::from(100_000),
    ///     reserve1: Uint::<112, 2>::from(200_000),
    /// };
    ///
    /// state.update_from_sync_event(&sync, 19_000_000)?;
    /// assert_eq!(state.get_last_block(), 19_000_000);
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_from_sync_event(&mut self, event: &Sync, block_number: u64) -> TrackerResult<()> {
        debug!("Updating state from Sync event at block {}", block_number);

        // Check for potential reorg
        if block_number < self.last_block {
            warn!(
                "Potential reorg detected: new block {} < last block {}",
                block_number, self.last_block
            );
            return Err(TrackerError::state(
                format!(
                    "Block number {} is less than last processed block {}. Possible reorg.",
                    block_number, self.last_block
                ),
                None,
            ));
        }

        // Convert uint112 reserves to U256 for validation and storage
        let weth = U256::from(event.reserve0);
        let usdt = U256::from(event.reserve1);

        // Validate reserves are non-zero
        if weth.is_zero() {
            return Err(TrackerError::state(
                format!("Invalid WETH reserve: zero value at block {block_number}"),
                None,
            ));
        }

        if usdt.is_zero() {
            return Err(TrackerError::state(
                format!("Invalid USDT reserve: zero value at block {block_number}"),
                None,
            ));
        }

        // Validate reserves are within reasonable bounds
        let max = U256::from(MAX_RESERVE_VALUE);
        if weth > max {
            return Err(TrackerError::state(
                format!(
                    "WETH reserve {weth} exceeds maximum threshold {max} at block {block_number}"
                ),
                None,
            ));
        }

        if usdt > max {
            return Err(TrackerError::state(
                format!(
                    "USDT reserve {usdt} exceeds maximum threshold {max} at block {block_number}"
                ),
                None,
            ));
        }

        // Atomic update: all validation passed, now update state
        self.weth_reserve = weth;
        self.usdt_reserve = usdt;
        self.last_block = block_number;

        info!(
            "State updated: WETH={}, USDT={}, block={}",
            weth, usdt, block_number
        );

        Ok(())
    }

    /// Get the current reserves.
    ///
    /// Returns a tuple of `(WETH reserve, USDT reserve)` as `U256` values.
    ///
    /// ## Returns
    ///
    /// - `weth`: Current WETH reserve (token0)
    /// - `usdt`: Current USDT reserve (token1)
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    /// use alloy::primitives::U256;
    ///
    /// let state = State::new();
    /// let (weth, usdt) = state.get_reserves();
    /// assert_eq!(weth, U256::ZERO);
    /// assert_eq!(usdt, U256::ZERO);
    /// ```
    #[must_use]
    pub const fn get_reserves(&self) -> (U256, U256) {
        (self.weth_reserve, self.usdt_reserve)
    }

    /// Get the last processed block number.
    ///
    /// Returns the block number of the most recent `update_from_sync_event()` call.
    /// Returns 0 if no updates have been processed yet.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let state = State::new();
    /// assert_eq!(state.get_last_block(), 0);
    /// ```
    #[must_use]
    pub const fn get_last_block(&self) -> u64 {
        self.last_block
    }

    /// Check if the state has been initialized with any reserve data.
    ///
    /// Returns `true` if reserves are non-zero, indicating at least one
    /// successful update has occurred.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let state = State::new();
    /// assert!(!state.is_initialized());
    /// ```
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        !self.weth_reserve.is_zero() && !self.usdt_reserve.is_zero()
    }

    /// Get the last processed block hash.
    ///
    /// Returns the block hash of the most recent successfully processed block,
    /// or `None` if no blocks have been processed yet.
    ///
    /// This is used for chain reorganization detection by verifying that each
    /// new block's parent hash matches this value.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let state = State::new();
    /// assert!(state.last_block_hash().is_none());
    /// ```
    #[must_use]
    pub const fn last_block_hash(&self) -> Option<B256> {
        self.last_block_hash
    }

    /// Set the last processed block hash.
    ///
    /// Updates the block hash stored for reorg detection. This should be called
    /// after successfully processing a block's events.
    ///
    /// # Arguments
    ///
    /// * `hash` - The block hash to store
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    /// use alloy::primitives::b256;
    ///
    /// let mut state = State::new();
    /// let hash = b256!("0x1234567890123456789012345678901234567890123456789012345678901234");
    /// state.set_block_hash(hash);
    /// assert_eq!(state.last_block_hash(), Some(hash));
    /// ```
    pub fn set_block_hash(&mut self, hash: B256) {
        self.last_block_hash = Some(hash);
        debug!("Block hash updated: {}", hash);
    }

    /// Get the total number of detected chain reorganizations.
    ///
    /// Returns the count of reorgs that have been detected and handled
    /// during this indexer's lifetime (count persists across restarts).
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let state = State::new();
    /// assert_eq!(state.reorg_count(), 0);
    /// ```
    #[must_use]
    pub const fn reorg_count(&self) -> u64 {
        self.reorg_count
    }

    /// Increment the reorg counter.
    ///
    /// Called when a chain reorganization is detected to track how many
    /// reorgs have occurred over the indexer's lifetime.
    pub fn increment_reorg_count(&mut self) {
        self.reorg_count += 1;
        warn!("Reorg count incremented to {}", self.reorg_count);
    }

    /// Invalidate state from a given block number.
    ///
    /// Used during reorg handling to rollback state to a known-good fork point.
    /// Clears the block hash and resets the last block number.
    ///
    /// # Arguments
    ///
    /// * `fork_point` - The last valid block number (state will be reset to this point)
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::state::State;
    ///
    /// let mut state = State::new();
    /// // ... update state to block 19000100 ...
    /// 
    /// // Reorg detected, rollback to fork point
    /// state.invalidate_from(19000050);
    /// assert_eq!(state.get_last_block(), 19000050);
    /// assert!(state.last_block_hash().is_none());
    /// ```
    pub fn invalidate_from(&mut self, fork_point: u64) {
        info!(
            "Invalidating state from block {} (was at block {})",
            fork_point, self.last_block
        );
        self.last_block = fork_point;
        self.last_block_hash = None;
        // Note: We keep the reserves since they represent the state at fork_point
        // The caller should re-fetch events from fork_point to rebuild accurate state
    }

    /// Save state to a JSON file.
    ///
    /// Persists the current reserves and last processed block number to disk
    /// for resuming after shutdown.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file where state will be saved
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be written or JSON serialization fails.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> TrackerResult<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| TrackerError::state("Failed to serialize state", Some(Box::new(e))))?;
        
        fs::write(path.as_ref(), json)
            .map_err(|e| TrackerError::state("Failed to write state file", Some(Box::new(e))))?;
        
        info!("State saved to {}", path.as_ref().display());
        Ok(())
    }

    /// Load state from a JSON file.
    ///
    /// Restores reserves and last processed block number from a previously
    /// saved state file. If the file doesn't exist, returns a new empty state.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the JSON file to load state from
    ///
    /// # Errors
    ///
    /// Returns error if file exists but cannot be read or JSON is invalid.
    pub fn load<P: AsRef<Path>>(path: P) -> TrackerResult<Self> {
        if !path.as_ref().exists() {
            info!("No state file found at {}, starting fresh", path.as_ref().display());
            return Ok(Self::new());
        }

        let json = fs::read_to_string(path.as_ref())
            .map_err(|e| TrackerError::state("Failed to read state file", Some(Box::new(e))))?;
        
        let state: Self = serde_json::from_str(&json)
            .map_err(|e| TrackerError::state("Failed to deserialize state", Some(Box::new(e))))?;
        
        info!("State loaded from {}: last_block={}", path.as_ref().display(), state.last_block);
        Ok(state)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Uint;

    #[test]
    fn test_new_state() {
        let state = State::new();
        assert_eq!(state.get_last_block(), 0);
        assert_eq!(state.get_reserves(), (U256::ZERO, U256::ZERO));
        assert!(!state.is_initialized());
    }

    #[test]
    fn test_default_state() {
        let state = State::default();
        assert_eq!(state, State::new());
    }

    #[test]
    fn test_update_from_sync_event() {
        let mut state = State::new();

        let sync = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000_000_000_000_000_u128), // 1 WETH
            reserve1: Uint::<112, 2>::from(3_000_000_000_u128),             // 3000 USDT
        };

        let result = state.update_from_sync_event(&sync, 19_000_000);
        assert!(result.is_ok());

        let (weth, usdt) = state.get_reserves();
        assert_eq!(weth, U256::from(1_000_000_000_000_000_000_u128));
        assert_eq!(usdt, U256::from(3_000_000_000_u128));
        assert_eq!(state.get_last_block(), 19_000_000);
        assert!(state.is_initialized());
    }

    #[test]
    fn test_update_zero_weth_reserve() {
        let mut state = State::new();

        let sync = Sync {
            reserve0: Uint::<112, 2>::ZERO, // Invalid: zero WETH
            reserve1: Uint::<112, 2>::from(1_000_000),
        };

        let result = state.update_from_sync_event(&sync, 19_000_000);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid WETH reserve"));
        }
    }

    #[test]
    fn test_update_zero_usdt_reserve() {
        let mut state = State::new();

        let sync = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::ZERO, // Invalid: zero USDT
        };

        let result = state.update_from_sync_event(&sync, 19_000_000);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid USDT reserve"));
        }
    }

    #[test]
    fn test_update_excessive_reserve() {
        let mut state = State::new();

        // Try to set a reserve above MAX_RESERVE_VALUE
        // Note: uint112 max is ~5.19e33, but our MAX_RESERVE_VALUE is 10^30
        let excessive = Uint::<112, 2>::from(MAX_RESERVE_VALUE + 1);

        let sync = Sync {
            reserve0: excessive,
            reserve1: Uint::<112, 2>::from(1_000_000),
        };

        let result = state.update_from_sync_event(&sync, 19_000_000);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("exceeds maximum threshold"));
        }
    }

    #[test]
    fn test_update_reorg_detection() {
        let mut state = State::new();

        // First update at block 100
        let sync1 = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::from(2_000_000),
        };
        state.update_from_sync_event(&sync1, 100).ok();

        // Try to update with earlier block (reorg scenario)
        let sync2 = Sync {
            reserve0: Uint::<112, 2>::from(1_100_000),
            reserve1: Uint::<112, 2>::from(2_200_000),
        };
        let result = state.update_from_sync_event(&sync2, 99);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Possible reorg"));
        }

        // Verify state wasn't updated
        assert_eq!(state.get_last_block(), 100);
    }

    #[test]
    fn test_sequential_updates() {
        let mut state = State::new();

        // Update 1
        let sync1 = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::from(2_000_000),
        };
        state.update_from_sync_event(&sync1, 100).ok();

        // Update 2 (later block, different reserves)
        let sync2 = Sync {
            reserve0: Uint::<112, 2>::from(1_100_000),
            reserve1: Uint::<112, 2>::from(2_200_000),
        };
        state.update_from_sync_event(&sync2, 101).ok();

        let (weth, usdt) = state.get_reserves();
        assert_eq!(weth, U256::from(1_100_000));
        assert_eq!(usdt, U256::from(2_200_000));
        assert_eq!(state.get_last_block(), 101);
    }

    #[test]
    fn test_is_initialized() {
        let mut state = State::new();
        assert!(!state.is_initialized());

        let sync = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::from(2_000_000),
        };
        state.update_from_sync_event(&sync, 100).ok();

        assert!(state.is_initialized());
    }

    #[test]
    fn test_state_clone() {
        let mut state1 = State::new();

        let sync = Sync {
            reserve0: Uint::<112, 2>::from(1_000_000),
            reserve1: Uint::<112, 2>::from(2_000_000),
        };
        state1.update_from_sync_event(&sync, 100).ok();

        let state2 = state1.clone();
        assert_eq!(state1, state2);
        assert_eq!(state1.get_reserves(), state2.get_reserves());
        assert_eq!(state1.get_last_block(), state2.get_last_block());
    }
}

//! Configuration management for the Uniswap V2 event indexer.
//!
//! This module handles loading and validating configuration from environment variables.
//! The config is loaded in this order:
//! 1. Attempts to load `.env` file via `dotenvy`
//! 2. Reads environment variables directly
//! 3. Applies defaults for optional variables
//!
//! ## Environment Setup
//!
//! When running with `cargo run`, you need to explicitly export environment variables:
//!
//! ```bash
//! # Option A: Set RPC_URL directly (recommended)
//! export RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
//! cargo run -- price
//!
//! # Option B: Set ALCHEMY_API_KEY (backward compatible)
//! export ALCHEMY_API_KEY="YOUR_API_KEY"
//! cargo run -- price
//!
//! # Option C: Use on command line
//! RPC_URL="..." cargo run -- price
//! ```
//!
//! When running the compiled binary directly, you can also use a `.env` file:
//!
//! ```bash
//! ./target/release/eth-uniswap-alloy price  # Automatically loads .env
//! ```
//!
//! ## Environment Variables
//!
//! Required:
//! - `RPC_URL`: Full Ethereum RPC URL (alternatively `ALCHEMY_API_KEY` for backward compatibility)
//!
//! Optional (with defaults):
//! - `ANVIL_FORK_BLOCK`: Block number for Anvil fork testing (default: 19000000)
//! - `STATE_FILE`: Path to state persistence file (default: "./state.json")
//! - `WATCH_MODE`: Enable continuous monitoring (default: false)
//! - `POLL_INTERVAL_SECS`: Polling interval in watch mode (default: 12)
//! - `BATCH_SIZE`: Maximum blocks per query (default: 1000)
//! - `POOL_ADDRESS`: Uniswap V2 pool address (default: WETH/USDT pool)
//! - `RUST_LOG`: Logging level (default: "info")
//!
//! ## Example
//!
//! ```no_run
//! use eth_uniswap_alloy::config::Config;
//! use eth_uniswap_alloy::error::TrackerResult;
//!
//! # fn main() -> TrackerResult<()> {
//! let config = Config::from_env()?;
//! println!("RPC URL: {}", config.rpc_url());
//! # Ok(())
//! # }
//! ```

use crate::error::{TrackerError, TrackerResult};
use std::env;
use std::path::PathBuf;

/// Main configuration struct for the indexer.
///
/// Contains all runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Ethereum RPC URL constructed from Alchemy API key
    rpc_url: String,

    /// Alchemy API key
    alchemy_api_key: String,

    /// Block number for Anvil fork testing
    anvil_fork_block: u64,

    /// Path to state persistence file
    state_file: PathBuf,

    /// Enable continuous monitoring mode
    watch_mode: bool,

    /// Polling interval in seconds (for watch mode)
    poll_interval_secs: u64,

    /// Maximum blocks to fetch per query
    batch_size: u64,

    /// Uniswap V2 pool address to monitor
    pool_address: String,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// This function:
    /// 1. Loads `.env` file using `dotenvy` (if present)
    /// 2. Reads and validates all environment variables
    /// 3. Applies defaults for optional variables
    /// 4. Constructs the RPC URL from the Alchemy API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required environment variables are missing
    /// - Environment variable values are invalid (e.g., non-numeric for numbers)
    /// - Pool address is not a valid Ethereum address format
    ///
    /// # Example
    ///
    /// ```no_run
    /// use eth_uniswap_alloy::config::Config;
    /// use eth_uniswap_alloy::error::TrackerResult;
    ///
    /// # fn main() -> TrackerResult<()> {
    /// let config = Config::from_env()?;
    /// println!("Configuration loaded successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_env() -> TrackerResult<Self> {
        // Load .env file if present (ignore error if file doesn't exist)
        dotenvy::dotenv().ok();

        // Required: RPC URL (or construct from ALCHEMY_API_KEY for backward compatibility)
        let rpc_url = match env::var("RPC_URL") {
            Ok(url) if !url.is_empty() && url != "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY_HERE" && url.starts_with("http") => {
                url
            }
            Ok(url) if !url.starts_with("http") => {
                // User provided an invalid RPC URL (e.g., just "your_key")
                return Err(TrackerError::config(
                    format!(
                        "Invalid RPC_URL format: '{}'\n\nExpected: https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY\n\nUsage:\n  RPC_URL=\"https://...\" cargo run -- price\n  or\n  ALCHEMY_API_KEY=\"YOUR_KEY\" cargo run -- price",
                        url
                    ),
                    None,
                ));
            }
            _ => {
                // Fallback to ALCHEMY_API_KEY for backward compatibility
                let alchemy_api_key = env::var("ALCHEMY_API_KEY").map_err(|_| {
                    TrackerError::config(
                        "RPC_URL or ALCHEMY_API_KEY environment variable is required\n\nUsage:\n  RPC_URL=\"https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY\" cargo run -- price\n  or\n  ALCHEMY_API_KEY=\"YOUR_KEY\" cargo run -- price",
                        None,
                    )
                })?;

                if alchemy_api_key.is_empty() || alchemy_api_key == "your_alchemy_api_key_here" {
                    return Err(TrackerError::config(
                        "ALCHEMY_API_KEY must be set to a valid Alchemy API key\n\nUsage:\n  ALCHEMY_API_KEY=\"YOUR_ACTUAL_KEY\" cargo run -- price",
                        None,
                    ));
                }

                format!("https://eth-mainnet.g.alchemy.com/v2/{alchemy_api_key}")
            }
        };

        // Extract API key for storage (for backward compatibility)
        let alchemy_api_key = if let Some(api_key) = rpc_url.strip_prefix("https://eth-mainnet.g.alchemy.com/v2/") {
            api_key.to_string()
        } else {
            "custom_rpc".to_string()
        };

        // Optional: Anvil fork block (default: 19000000)
        let anvil_fork_block = env::var("ANVIL_FORK_BLOCK")
            .unwrap_or_else(|_| "19000000".to_string())
            .parse::<u64>()
            .map_err(|e| {
                TrackerError::config(
                    "ANVIL_FORK_BLOCK must be a valid block number",
                    Some(Box::new(e)),
                )
            })?;

        // Optional: State file (default: ./state.json)
        let state_file = env::var("STATE_FILE")
            .unwrap_or_else(|_| "./state.json".to_string())
            .into();

        // Optional: Watch mode (default: false)
        let watch_mode = env::var("WATCH_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|e| {
                TrackerError::config("WATCH_MODE must be 'true' or 'false'", Some(Box::new(e)))
            })?;

        // Optional: Poll interval (default: 12 seconds)
        let poll_interval_secs = env::var("POLL_INTERVAL_SECS")
            .unwrap_or_else(|_| "12".to_string())
            .parse::<u64>()
            .map_err(|e| {
                TrackerError::config(
                    "POLL_INTERVAL_SECS must be a valid number",
                    Some(Box::new(e)),
                )
            })?;

        // Optional: Batch size (default: 1000 blocks)
        let batch_size = env::var("BATCH_SIZE")
            .unwrap_or_else(|_| "1000".to_string())
            .parse::<u64>()
            .map_err(|e| {
                TrackerError::config("BATCH_SIZE must be a valid number", Some(Box::new(e)))
            })?;

        // Optional: Pool address (default: WETH/USDT pool)
        let pool_address = env::var("POOL_ADDRESS")
            .unwrap_or_else(|_| "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string());

        // Validate pool address format (basic check for 0x prefix and length)
        if !pool_address.starts_with("0x") || pool_address.len() != 42 {
            return Err(TrackerError::config(
                format!(
                    "POOL_ADDRESS must be a valid Ethereum address (0x + 40 hex chars), got: {pool_address}"
                ),
                None,
            ));
        }

        Ok(Self {
            rpc_url,
            alchemy_api_key,
            anvil_fork_block,
            state_file,
            watch_mode,
            poll_interval_secs,
            batch_size,
            pool_address,
        })
    }

    /// Get the Ethereum RPC URL.
    #[must_use]
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }

    /// Get the Alchemy API key.
    #[must_use]
    pub fn alchemy_api_key(&self) -> &str {
        &self.alchemy_api_key
    }

    /// Get the Anvil fork block number.
    #[must_use]
    pub const fn anvil_fork_block(&self) -> u64 {
        self.anvil_fork_block
    }

    /// Get the state file path.
    #[must_use]
    pub const fn state_file(&self) -> &PathBuf {
        &self.state_file
    }

    /// Check if watch mode is enabled.
    #[must_use]
    pub const fn watch_mode(&self) -> bool {
        self.watch_mode
    }

    /// Get the polling interval in seconds.
    #[must_use]
    pub const fn poll_interval_secs(&self) -> u64 {
        self.poll_interval_secs
    }

    /// Get the batch size (max blocks per query).
    #[must_use]
    pub const fn batch_size(&self) -> u64 {
        self.batch_size
    }

    /// Get the pool address.
    #[must_use]
    pub fn pool_address(&self) -> &str {
        &self.pool_address
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation_empty_api_key() {
        // Clean up any existing env vars
        env::remove_var("ALCHEMY_API_KEY");
        env::remove_var("POOL_ADDRESS");

        // Set an empty API key
        env::set_var("ALCHEMY_API_KEY", "");

        let result = Config::from_env();
        assert!(result.is_err());

        // Clean up
        env::remove_var("ALCHEMY_API_KEY");
    }

    #[test]
    fn test_config_validation_placeholder_api_key() {
        // Clean up any existing env vars
        env::remove_var("ALCHEMY_API_KEY");
        env::remove_var("POOL_ADDRESS");

        // Set the placeholder API key
        env::set_var("ALCHEMY_API_KEY", "your_alchemy_api_key_here");

        let result = Config::from_env();
        assert!(result.is_err());

        // Clean up
        env::remove_var("ALCHEMY_API_KEY");
    }

    #[test]
    fn test_config_validation_invalid_pool_address() {
        // Clean up any existing env vars
        env::remove_var("ALCHEMY_API_KEY");
        env::remove_var("POOL_ADDRESS");

        // Set valid API key but invalid pool address
        env::set_var("ALCHEMY_API_KEY", "test_key_123");
        env::set_var("POOL_ADDRESS", "invalid_address");

        let result = Config::from_env();
        assert!(result.is_err());

        // Clean up
        env::remove_var("ALCHEMY_API_KEY");
        env::remove_var("POOL_ADDRESS");
    }

    #[test]
    fn test_config_rpc_url_construction() {
        // Clean up any existing env vars
        env::remove_var("ALCHEMY_API_KEY");
        env::remove_var("POOL_ADDRESS");

        env::set_var("ALCHEMY_API_KEY", "test_api_key");

        let config = Config::from_env();
        assert!(config.is_ok());

        if let Ok(config) = config {
            assert_eq!(
                config.rpc_url(),
                "https://eth-mainnet.g.alchemy.com/v2/test_api_key"
            );
        }

        // Clean up
        env::remove_var("ALCHEMY_API_KEY");
    }
}

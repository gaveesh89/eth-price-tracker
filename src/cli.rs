//! Command-line interface for the Uniswap V2 event tracker.
//!
//! This module provides a CLI for fetching and monitoring ETH/USDT prices
//! from the Uniswap V2 pool using event indexing.
//!
//! # Commands
//!
//! - `price`: Fetch current ETH price (one-time)
//! - `watch`: Monitor price updates in real-time
//!
//! # Example
//!
//! ```bash
//! # Fetch current price
//! eth-uniswap-alloy price
//!
//! # Watch for price updates
//! eth-uniswap-alloy watch
//! ```

use crate::config::Config;
use crate::error::{TrackerError, TrackerResult};
use crate::events::{create_sync_filter_for_pair, Sync, UNISWAP_V2_WETH_USDT_PAIR};
use crate::pricing::calculate_eth_price;
use crate::rpc::{create_provider, get_latest_block};
use crate::state::State;
use alloy::primitives::{Log as PrimitiveLog, U256};
use alloy::providers::Provider;
use alloy::rpc::types::Log;
use alloy::sol_types::SolEvent;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Uniswap V2 ETH/USDT Price Tracker
#[derive(Parser, Debug)]
#[command(name = "eth-uniswap-alloy")]
#[command(about = "Production-grade Ethereum event indexer for Uniswap V2", long_about = None)]
#[command(version)]
struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Fetch current ETH/USDT price (one-time)
    Price {
        /// Number of recent blocks to scan (default: 100)
        #[arg(short, long, default_value = "100")]
        blocks: u64,
    },

    /// Monitor price updates in real-time
    Watch {
        /// Polling interval in seconds (default: 12)
        #[arg(short, long, default_value = "12")]
        interval: u64,

        /// Starting block number (default: latest - 100)
        #[arg(short, long)]
        start_block: Option<u64>,
    },
}

/// Parse CLI arguments and execute the appropriate command.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration loading fails
/// - RPC connection fails
/// - Command execution fails
pub async fn run() -> TrackerResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Price { blocks } => run_price_command(blocks).await,
        Commands::Watch {
            interval,
            start_block,
        } => run_watch_command(interval, start_block).await,
    }
}

/// Execute the price command (one-time fetch).
async fn run_price_command(blocks: u64) -> TrackerResult<()> {
    info!("Fetching current ETH/USDT price");

    // Load configuration
    let config = Config::from_env()?;

    // Create provider
    let provider = create_provider(config.rpc_url()).await?;

    // Get latest block
    let latest_block = get_latest_block(&provider).await?;
    info!("Latest block: {}", latest_block);

    // Calculate starting block
    let from_block = latest_block.saturating_sub(blocks);
    info!("Scanning blocks {} to {}", from_block, latest_block);

    // Fetch Sync events
    let logs = fetch_sync_events(&provider, from_block, latest_block).await?;

    if logs.is_empty() {
        warn!("No Sync events found in the last {} blocks", blocks);
        println!(
            "{}",
            "No recent price updates found. Try increasing --blocks."
                .yellow()
                .bold()
        );
        return Ok(());
    }

    info!("Found {} Sync events", logs.len());

    // Process the most recent event
    let latest_log = logs
        .last()
        .ok_or_else(|| TrackerError::state("No logs available", None))?;

    let (sync_event, block_number) = decode_sync_event(latest_log)?;

    // Calculate price
    let weth_reserve = U256::from(sync_event.reserve0);
    let usdt_reserve = U256::from(sync_event.reserve1);
    let price = calculate_eth_price(weth_reserve, usdt_reserve)?;

    // Display result
    print_price_update(block_number, price, weth_reserve, usdt_reserve, None);

    Ok(())
}

/// Execute the watch command (continuous monitoring).
async fn run_watch_command(interval: u64, start_block: Option<u64>) -> TrackerResult<()> {
    info!("Starting price watch mode");
    println!(
        "{}",
        "🔍 Watching for ETH/USDT price updates...".cyan().bold()
    );
    println!();

    // Load configuration
    let config = Config::from_env()?;

    // Create provider
    let provider = create_provider(config.rpc_url()).await?;

    // Initialize state tracker - load from file if exists
    let mut state = State::load(config.state_file())
        .unwrap_or_else(|e| {
            warn!("Failed to load state: {}, starting fresh", e);
            State::new()
        });
    let mut last_price: Option<f64> = None;

    // Determine starting block (use saved state if available)
    let latest_block = get_latest_block(&provider).await?;
    let mut last_processed_block = if state.get_last_block() > 0 {
        info!("Resuming from saved state at block: {}", state.get_last_block());
        state.get_last_block()
    } else {
        start_block.unwrap_or_else(|| latest_block.saturating_sub(100))
    };
    info!("Starting from block: {}", last_processed_block);

    // Setup graceful shutdown handler
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    // Main watch loop
    loop {
        tokio::select! {
            // Handle shutdown signal
            _ = &mut shutdown => {
                info!("Shutdown signal received, cleaning up...");
                println!();
                println!("{}", "🛑 Shutting down gracefully...".yellow().bold());
                
                // Save final state
                if let Err(e) = state.save(config.state_file()) {
                    error!("Failed to save state on shutdown: {}", e);
                    println!("{} Failed to save state: {}", "⚠️".red(), e);
                } else {
                    println!("{} State saved to {}", "✅".green(), config.state_file().display());
                    println!("{} Last processed block: {}", "📍".cyan(), last_processed_block);
                }
                
                println!("{}", "👋 Shutdown complete".green().bold());
                info!("Shutdown complete");
                break;
            }
            
            // Process blocks
            _ = tokio::time::sleep(Duration::from_secs(0)) => {
                match process_new_blocks(
                    &provider,
                    &mut state,
                    &mut last_processed_block,
                    &mut last_price,
                )
                .await
                {
                    Ok(()) => {
                        // Successfully processed, wait for next interval
                        debug!("Waiting {} seconds for next check", interval);
                    }
                    Err(e) => {
                        error!("Error processing blocks: {}", e);
                        println!("{} {}", "⚠️  Error:".red().bold(), e);
                    }
                }

                // Wait before next check
                tokio::time::sleep(Duration::from_secs(interval)).await;
            }
        }
    }

    Ok(())
}

/// Process new blocks since last check (incremental).
///
/// This function only fetches events from blocks that haven't been processed yet,
/// implementing efficient incremental indexing rather than naive polling.
/// Batches queries into 10-block chunks for Alchemy free tier compatibility.
async fn process_new_blocks(
    provider: &crate::rpc::Provider,
    state: &mut State,
    last_processed_block: &mut u64,
    last_price: &mut Option<f64>,
) -> TrackerResult<()> {
    // Get current latest block
    let current_latest = get_latest_block(provider).await?;

    // Check if there are new blocks to process
    if current_latest <= *last_processed_block {
        debug!(
            "No new blocks (current: {}, last: {})",
            current_latest, *last_processed_block
        );
        return Ok(());
    }

    let from_block = last_processed_block.saturating_add(1);
    let to_block = current_latest;

    debug!("Processing new blocks: {} to {}", from_block, to_block);

    // Batch size: 10 blocks (Alchemy free tier limit)
    const BATCH_SIZE: u64 = 10;
    let mut current_block = from_block;
    let mut total_events = 0;

    // Process blocks in batches
    while current_block <= to_block {
        let batch_end = std::cmp::min(current_block + BATCH_SIZE - 1, to_block);
        debug!("Fetching batch: blocks {} to {}", current_block, batch_end);

        // Fetch events from this batch
        let logs = fetch_sync_events(provider, current_block, batch_end).await?;

        if !logs.is_empty() {
            total_events += logs.len();
            debug!("Found {} events in batch", logs.len());

            // Process each event
            for log in logs {
                let (sync_event, block_number) = decode_sync_event(&log)?;

                // Update state
                state.update_from_sync_event(&sync_event, block_number)?;

                // Calculate price
                let (weth_reserve, usdt_reserve) = state.get_reserves();
                let price = calculate_eth_price(weth_reserve, usdt_reserve)?;

                // Calculate price change
                let price_change = last_price.map(|last| ((price - last) / last) * 100.0);

                // Display update
                print_price_update(
                    block_number,
                    price,
                    weth_reserve,
                    usdt_reserve,
                    price_change,
                );

                // Update last price
                *last_price = Some(price);
            }
        }

        current_block = batch_end + 1;
    }

    if total_events > 0 {
        info!("Found {} Sync events in range {} to {}", total_events, from_block, to_block);
    } else {
        debug!("No Sync events in blocks {} to {}", from_block, to_block);
    }

    // Update last processed block
    *last_processed_block = to_block;

    Ok(())
}

/// Fetch Sync events from the Uniswap V2 WETH/USDT pair.
async fn fetch_sync_events(
    provider: &crate::rpc::Provider,
    from_block: u64,
    to_block: u64,
) -> TrackerResult<Vec<Log>> {
    let filter = create_sync_filter_for_pair(UNISWAP_V2_WETH_USDT_PAIR, from_block, to_block);

    let logs = provider
        .get_logs(&filter)
        .await
        .map_err(|e| TrackerError::rpc(format!("Failed to fetch Sync events: {e}"), None))?;

    debug!("Fetched {} logs from blockchain", logs.len());

    Ok(logs)
}

/// Decode a log into a Sync event.
fn decode_sync_event(log: &Log) -> TrackerResult<(Sync, u64)> {
    let block_number = log
        .block_number
        .ok_or_else(|| TrackerError::decoding("Log missing block number", None))?;

    // Convert RPC Log to Primitive Log for decoding
    let primitive_log = PrimitiveLog {
        address: log.address(),
        data: log.data().clone(),
    };

    let sync_event = Sync::decode_log(&primitive_log, true)
        .map_err(|e| TrackerError::decoding(format!("Failed to decode Sync event: {e}"), None))?;

    Ok((sync_event.data, block_number))
}

/// Display a price update with colored formatting.
fn print_price_update(
    block_number: u64,
    price: f64,
    weth_reserve: U256,
    usdt_reserve: U256,
    price_change: Option<f64>,
) {
    // Timestamp
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");

    // Format price with color based on change
    let price_str = format!("${price:.2}");
    let colored_price = price_change.map_or_else(
        || price_str.white().bold().to_string(),
        |change| {
            if change > 0.0 {
                format!(
                    "{} ({}%)",
                    price_str.green().bold(),
                    format!("+{change:.2}").green()
                )
            } else if change < 0.0 {
                format!(
                    "{} ({}%)",
                    price_str.red().bold(),
                    format!("{change:.2}").red()
                )
            } else {
                price_str.white().bold().to_string()
            }
        },
    );

    // Format reserves
    let weth_formatted = format_reserve(weth_reserve, 18);
    let usdt_formatted = format_reserve(usdt_reserve, 6);

    // Print formatted output
    println!(
        "{} {} Block: {} | Price: {} | WETH: {} | USDT: {}",
        "📊".cyan(),
        timestamp.to_string().dimmed(),
        block_number.to_string().yellow(),
        colored_price,
        weth_formatted.blue(),
        usdt_formatted.magenta()
    );
}

/// Format reserve amount with proper decimal places.
fn format_reserve(reserve: U256, decimals: u32) -> String {
    // Convert U256 to f64 for display (with precision loss for very large values)
    let divisor = 10_u128.pow(decimals);
    let reserve_u128 = u128::try_from(reserve).unwrap_or(u128::MAX);
    #[allow(clippy::cast_precision_loss)]
    let reserve_float = reserve_u128 as f64 / divisor as f64;

    format!("{reserve_float:.2}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_reserve() {
        // Test WETH (18 decimals)
        let weth = U256::from(1_000_000_000_000_000_000_u128); // 1 WETH
        assert_eq!(format_reserve(weth, 18), "1.00");

        // Test USDT (6 decimals)
        let usdt = U256::from(2_000_000_u128); // 2 USDT
        assert_eq!(format_reserve(usdt, 6), "2.00");
    }

    #[test]
    fn test_cli_parsing() {
        // Test price command
        let args = vec!["eth-uniswap-alloy", "price"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());

        // Test watch command
        let args = vec!["eth-uniswap-alloy", "watch"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());
    }

    #[test]
    fn test_price_command_with_blocks() {
        let args = vec!["eth-uniswap-alloy", "price", "--blocks", "200"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());

        if let Ok(Cli {
            command: Commands::Price { blocks },
        }) = cli
        {
            assert_eq!(blocks, 200);
        }
    }

    #[test]
    fn test_watch_command_with_interval() {
        let args = vec!["eth-uniswap-alloy", "watch", "--interval", "30"];
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());

        if let Ok(Cli {
            command: Commands::Watch { interval, .. },
        }) = cli
        {
            assert_eq!(interval, 30);
        }
    }
}

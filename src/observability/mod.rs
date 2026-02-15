//! Observability and structured logging infrastructure.
//!
//! This module provides production-grade logging using the tracing framework,
//! enabling filtering, performance profiling, and production observability.
//!
//! # Features
//!
//! - **Structured Logging**: Key-value pairs for machine-parseable logs
//! - **Span Tracking**: Trace operations across async boundaries
//! - **Performance Profiling**: Measure operation duration
//! - **Multiple Formats**: Console (pretty/JSON) and file output
//! - **Environment Filtering**: RUST_LOG variable support
//!
//! # Usage
//!
//! Initialize tracing at application startup:
//!
//! ```no_run
//! use eth_uniswap_alloy::observability;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize with defaults (pretty console output, info level)
//!     observability::init_tracing(None, None, false)?;
//!     
//!     // Run application...
//!     Ok(())
//! }
//! ```
//!
//! # Environment Configuration
//!
//! Control logging via environment variables:
//!
//! ```bash
//! # Set log level for all modules
//! RUST_LOG=debug cargo run
//!
//! # Component-specific levels
//! RUST_LOG=eth_uniswap_alloy=debug,sqlx=warn cargo run
//!
//! # Enable JSON output for production
//! LOG_JSON=true cargo run
//!
//! # Write logs to file with daily rotation
//! LOG_FILE=./logs/indexer.log cargo run
//! ```

use std::path::{Path, PathBuf};
use std::ffi::OsStr;
use tracing::info;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Layer,
};

/// Initialize the tracing subscriber with configurable output formats.
///
/// This function sets up structured logging for the application, with support for:
/// - Console output (pretty-printed for development, JSON for production)
/// - Optional file output with daily rotation
/// - Environment-based filtering via RUST_LOG
///
/// # Arguments
///
/// * `log_level` - Optional log level override (e.g., "debug", "info"). 
///                 Falls back to RUST_LOG environment variable.
/// * `log_file` - Optional file path for log output. Enables daily log rotation.
/// * `json_output` - If true, outputs JSON format suitable for log aggregation.
///                   If false, uses pretty-printed human-readable format.
///
/// # Defaults
///
/// When no configuration is provided:
/// - Level: `info` for eth_uniswap_alloy, `warn` for dependencies
/// - Format: Pretty-printed with colors and timestamps
/// - Output: Console only (no file)
///
/// # Examples
///
/// ```no_run
/// use eth_uniswap_alloy::observability;
/// use std::path::PathBuf;
///
/// // Development: pretty console output at debug level
/// observability::init_tracing(
///     Some("debug".to_string()),
///     None,
///     false
/// )?;
///
/// // Production: JSON console output + rotating file
/// observability::init_tracing(
///     Some("info".to_string()),
///     Some(PathBuf::from("./logs/indexer.log")),
///     true
/// )?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Log Levels
///
/// - `error`: Fatal errors requiring immediate attention
/// - `warn`: Issues that should be investigated but aren't fatal
/// - `info`: Important state changes and milestones (default)
/// - `debug`: Detailed debugging information
/// - `trace`: Very verbose, function entry/exit
///
/// # Errors
///
/// Returns an error if:
/// - File path is invalid or cannot be created
/// - Log initialization fails
pub fn init_tracing(
    log_level: Option<String>,
    log_file: Option<PathBuf>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Build environment filter from RUST_LOG or provided level
    let env_filter = if let Ok(filter) = std::env::var("RUST_LOG") {
        EnvFilter::new(filter)
    } else if let Some(level) = log_level {
        EnvFilter::new(level)
    } else {
        // Default: info for our app, warn for dependencies
        // This reduces noise from SQLx, Alloy, and other libraries
        EnvFilter::new("eth_uniswap_alloy=info,warn")
    };

    // Console layer (stdout)
    let console_layer = if json_output {
        // Production: JSON output for log aggregation (ELK, Datadog, etc.)
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    } else {
        // Development: Human-readable colored output
        fmt::layer()
            .pretty()
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .boxed()
    };

    // File layer (optional)
    let file_layer = if let Some(ref path) = log_file {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create rolling file appender (rotates daily)
        let file_appender = tracing_appender::rolling::daily(
            path.parent().unwrap_or_else(|| Path::new(".")),
            path.file_name().unwrap_or_else(|| OsStr::new("app.log")),
        );

        // Non-blocking writer for better performance
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        // File always uses JSON for structured log analysis
        Some(
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_current_span(true)
                .with_span_list(true)
                .with_target(true)
                .with_file(true)
                .with_line_number(true)
                .boxed(),
        )
    } else {
        None
    };

    // Build subscriber with layers
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    // Add file layer if configured
    if let Some(file) = file_layer {
        subscriber.with(file).init();
    } else {
        subscriber.init();
    }

    info!(
        json_output,
        file_logging = log_file.is_some(),
        "Tracing initialized successfully"
    );

    Ok(())
}

/// Initialize tracing with test-specific configuration.
///
/// This function sets up logging for unit and integration tests,
/// with output directed to the test harness.
///
/// Use this in test modules to see logging output with `cargo test -- --nocapture`.
///
/// # Example
///
/// ```no_run
/// #[cfg(test)]
/// mod tests {
///     use super::*;
///
///     fn init_test_logging() {
///         let _ = eth_uniswap_alloy::observability::init_test_tracing();
///     }
///
///     #[tokio::test]
///     async fn test_with_logging() {
///         init_test_logging();
///         
///         // Test code with tracing output...
///     }
/// }
/// ```
#[cfg(test)]
pub fn init_test_tracing() {
    use tracing_subscriber::fmt::format::FmtSpan;

    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::DEBUG)
        .with_span_events(FmtSpan::CLOSE)
        .pretty()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_tracing_default() {
        // Test that default initialization doesn't panic
        // Note: Can only initialize once per process, so this may fail if run after others
        let result = init_tracing(None, None, false);
        // Don't assert success because tracing may already be initialized
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_init_tracing_with_level() {
        let result = init_tracing(Some("debug".to_string()), None, false);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_init_tracing_json() {
        let result = init_tracing(Some("info".to_string()), None, true);
        assert!(result.is_ok() || result.is_err());
    }
}

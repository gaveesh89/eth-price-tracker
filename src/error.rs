//! Error types for the Ethereum event indexer.
//!
//! This module provides a unified error type [`TrackerError`] that encompasses
//! all possible errors that can occur during event indexing, price calculation,
//! and state management.
//!
//! # Design
//!
//! The error hierarchy is organized by layer:
//! - [`TrackerError::ConfigError`]: Configuration and environment issues
//! - [`TrackerError::RpcError`]: RPC provider and network errors
//! - [`TrackerError::DecodingError`]: Event decoding and parsing errors
//! - [`TrackerError::StateError`]: State management and validation errors
//! - [`TrackerError::MathError`]: Arithmetic and calculation errors
//!
//! All errors implement [`std::error::Error`] and include rich context via
//! the source error chain.
//!
//! # Example
//!
//! ```
//! use eth_uniswap_alloy::error::{TrackerError, TrackerResult};
//!
//! fn validate_reserve(reserve: u128) -> TrackerResult<()> {
//!     if reserve == 0 {
//!         return Err(TrackerError::state(
//!             "reserve cannot be zero",
//!             None
//!         ));
//!     }
//!     Ok(())
//! }
//! ```

use std::fmt;

/// Result type alias using [`TrackerError`].
pub type TrackerResult<T> = Result<T, TrackerError>;

/// Unified error type for the Uniswap V2 event tracker.
///
/// This enum encompasses all error types that can occur during:
/// - Configuration loading
/// - RPC provider operations
/// - Event decoding
/// - State management
/// - Price calculations
#[derive(Debug)]
pub enum TrackerError {
    /// Configuration or environment variable errors.
    ///
    /// Variants include:
    /// - Missing or invalid environment variables
    /// - Invalid addresses or URLs
    /// - Malformed configuration values
    ConfigError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// RPC provider or network errors.
    ///
    /// Variants include:
    /// - Failed to connect to provider
    /// - Network timeout
    /// - Block not found
    /// - RPC method errors
    RpcError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Event decoding or parsing errors.
    ///
    /// Variants include:
    /// - Invalid log structure
    /// - Signature mismatch
    /// - Missing required fields
    /// - Type conversion failures
    DecodingError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// State management or validation errors.
    ///
    /// Variants include:
    /// - Invalid reserve values
    /// - Reorg detection
    /// - State consistency issues
    /// - Missing required state
    StateError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Arithmetic or calculation errors.
    ///
    /// Variants include:
    /// - Division by zero
    /// - Overflow/underflow
    /// - Out of range values
    /// - Precision loss
    MathError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Database operation errors.
    ///
    /// Variants include:
    /// - Connection failures
    /// - Query execution errors
    /// - Migration failures
    /// - Constraint violations
    /// - Transaction errors
    DatabaseError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// WebSocket connection errors.
    ///
    /// Variants include:
    /// - Failed to establish WebSocket connection
    /// - Authentication failures
    /// - Invalid WebSocket URL
    /// - Network connectivity issues
    WebSocketConnectionError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// WebSocket subscription errors.
    ///
    /// Variants include:
    /// - Failed to create subscription
    /// - Invalid filter parameters
    /// - Subscription dropped unexpectedly
    WebSocketSubscriptionError {
        /// Human-readable error message
        message: String,
        /// Optional underlying error
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// WebSocket disconnection error.
    ///
    /// Occurs when the WebSocket stream ends unexpectedly
    /// and reconnection is required.
    WebSocketDisconnected {
        /// Human-readable error message
        message: String,
    },

    /// Max reconnection attempts exceeded.
    ///
    /// Occurs when the WebSocket fails to reconnect after
    /// multiple attempts with exponential backoff.
    MaxReconnectAttemptsExceeded {
        /// Number of attempts made
        attempts: u32,
        /// Last error encountered
        last_error: String,
    },
}

impl TrackerError {
    /// Create a new configuration error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::config("ALCHEMY_API_KEY not set", None);
    /// assert!(matches!(err, TrackerError::ConfigError { .. }));
    /// ```
    #[must_use]
    pub fn config(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ConfigError {
            message: message.into(),
            source,
        }
    }

    /// Create a new RPC error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::rpc("Failed to connect to provider", None);
    /// assert!(matches!(err, TrackerError::RpcError { .. }));
    /// ```
    #[must_use]
    pub fn rpc(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::RpcError {
            message: message.into(),
            source,
        }
    }

    /// Create a new decoding error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::decoding("Invalid log structure", None);
    /// assert!(matches!(err, TrackerError::DecodingError { .. }));
    /// ```
    #[must_use]
    pub fn decoding(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::DecodingError {
            message: message.into(),
            source,
        }
    }

    /// Create a new state error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::state("Reserve cannot be zero", None);
    /// assert!(matches!(err, TrackerError::StateError { .. }));
    /// ```
    #[must_use]
    pub fn state(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::StateError {
            message: message.into(),
            source,
        }
    }

    /// Create a new math error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::math("Division by zero", None);
    /// assert!(matches!(err, TrackerError::MathError { .. }));
    /// ```
    #[must_use]
    pub fn math(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::MathError {
            message: message.into(),
            source,
        }
    }

    /// Create a new database error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::database("Connection failed", None);
    /// assert!(matches!(err, TrackerError::DatabaseError { .. }));
    /// ```
    #[must_use]
    pub fn database(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::DatabaseError {
            message: message.into(),
            source,
        }
    }

    /// Create a new WebSocket connection error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::websocket_connection("Connection refused", None);
    /// assert!(matches!(err, TrackerError::WebSocketConnectionError { .. }));
    /// ```
    #[must_use]
    pub fn websocket_connection(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::WebSocketConnectionError {
            message: message.into(),
            source,
        }
    }

    /// Create a new WebSocket subscription error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::websocket_subscription("Subscription failed", None);
    /// assert!(matches!(err, TrackerError::WebSocketSubscriptionError { .. }));
    /// ```
    #[must_use]
    pub fn websocket_subscription(
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::WebSocketSubscriptionError {
            message: message.into(),
            source,
        }
    }

    /// Create a new WebSocket disconnection error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::websocket_disconnected("Stream ended unexpectedly");
    /// assert!(matches!(err, TrackerError::WebSocketDisconnected { .. }));
    /// ```
    #[must_use]
    pub fn websocket_disconnected(message: impl Into<String>) -> Self {
        Self::WebSocketDisconnected {
            message: message.into(),
        }
    }

    /// Create a max reconnect attempts exceeded error.
    ///
    /// # Example
    ///
    /// ```
    /// use eth_uniswap_alloy::error::TrackerError;
    ///
    /// let err = TrackerError::max_reconnect_exceeded(10, "Connection timeout");
    /// assert!(matches!(err, TrackerError::MaxReconnectAttemptsExceeded { .. }));
    /// ```
    #[must_use]
    pub fn max_reconnect_exceeded(attempts: u32, last_error: impl Into<String>) -> Self {
        Self::MaxReconnectAttemptsExceeded {
            attempts,
            last_error: last_error.into(),
        }
    }
}

impl fmt::Display for TrackerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigError { message, .. } => write!(f, "Configuration error: {message}"),
            Self::RpcError { message, .. } => write!(f, "RPC error: {message}"),
            Self::DecodingError { message, .. } => write!(f, "Decoding error: {message}"),
            Self::StateError { message, .. } => write!(f, "State error: {message}"),
            Self::MathError { message, .. } => write!(f, "Math error: {message}"),
            Self::DatabaseError { message, .. } => write!(f, "Database error: {message}"),
            Self::WebSocketConnectionError { message, .. } => {
                write!(f, "WebSocket connection error: {message}")
            }
            Self::WebSocketSubscriptionError { message, .. } => {
                write!(f, "WebSocket subscription error: {message}")
            }
            Self::WebSocketDisconnected { message } => {
                write!(f, "WebSocket disconnected: {message}")
            }
            Self::MaxReconnectAttemptsExceeded {
                attempts,
                last_error,
            } => {
                write!(
                    f,
                    "Max reconnection attempts ({}) exceeded. Last error: {}",
                    attempts, last_error
                )
            }
        }
    }
}

impl std::error::Error for TrackerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ConfigError { source, .. }
            | Self::RpcError { source, .. }
            | Self::DecodingError { source, .. }
            | Self::StateError { source, .. }
            | Self::MathError { source, .. }
            | Self::DatabaseError { source, .. }
            | Self::WebSocketConnectionError { source, .. }
            | Self::WebSocketSubscriptionError { source, .. } => source
                .as_ref()
                .map(|e| e.as_ref() as &dyn std::error::Error),
            Self::WebSocketDisconnected { .. } | Self::MaxReconnectAttemptsExceeded { .. } => None,
        }
    }
}

/// Convert from `eyre::Report` to `TrackerError`.
///
/// This is primarily used for wrapping eyre errors that don't fit into
/// a specific category. The error is categorized as an RPC error by default.
impl From<eyre::Report> for TrackerError {
    fn from(err: eyre::Report) -> Self {
        Self::RpcError {
            message: err.to_string(),
            source: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_config_error() {
        let err = TrackerError::config("test error", None);
        assert!(matches!(err, TrackerError::ConfigError { .. }));
        assert_eq!(err.to_string(), "Configuration error: test error");
    }

    #[test]
    fn test_rpc_error() {
        let err = TrackerError::rpc("connection failed", None);
        assert!(matches!(err, TrackerError::RpcError { .. }));
        assert_eq!(err.to_string(), "RPC error: connection failed");
    }

    #[test]
    fn test_decoding_error() {
        let err = TrackerError::decoding("invalid log", None);
        assert!(matches!(err, TrackerError::DecodingError { .. }));
        assert_eq!(err.to_string(), "Decoding error: invalid log");
    }

    #[test]
    fn test_state_error() {
        let err = TrackerError::state("zero reserve", None);
        assert!(matches!(err, TrackerError::StateError { .. }));
        assert_eq!(err.to_string(), "State error: zero reserve");
    }

    #[test]
    fn test_math_error() {
        let err = TrackerError::math("overflow", None);
        assert!(matches!(err, TrackerError::MathError { .. }));
        assert_eq!(err.to_string(), "Math error: overflow");
    }

    #[test]
    fn test_error_with_source() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = TrackerError::config("failed to load", Some(Box::new(source)));

        assert!(err.source().is_some());
        assert_eq!(err.to_string(), "Configuration error: failed to load");
    }

    #[test]
    fn test_error_trait() {
        let err = TrackerError::rpc("test", None);
        // Ensure it implements Error trait
        let _: &dyn std::error::Error = &err;
    }
}

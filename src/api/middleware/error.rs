//! Unified API error handling.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::error;

use crate::api::models::ErrorResponse;
use crate::error::TrackerError;

/// API-specific error type.
#[derive(Debug)]
pub enum ApiError {
    /// Resource not found.
    NotFound(String),
    /// Invalid request parameters.
    BadRequest(String),
    /// Internal server error.
    InternalError(String),
    /// Rate limit exceeded.
    RateLimitExceeded,
    /// Database operation failed.
    DatabaseError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limit_exceeded",
                "Rate limit exceeded. Please try again later.".to_string(),
            ),
            ApiError::DatabaseError(msg) => {
                error!(error = %msg, "Database error in API handler");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "database_error",
                    "Database operation failed".to_string(),
                )
            }
            ApiError::InternalError(msg) => {
                error!(error = %msg, "Internal error in API handler");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(ErrorResponse {
            error: error_type.to_string(),
            message,
            details: None,
        });

        (status, body).into_response()
    }
}

impl From<TrackerError> for ApiError {
    fn from(err: TrackerError) -> Self {
        match err {
            TrackerError::DatabaseError { message, .. } => ApiError::DatabaseError(message),
            _ => ApiError::InternalError(err.to_string()),
        }
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::DatabaseError(err.to_string())
    }
}

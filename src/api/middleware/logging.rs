//! Request logging middleware using tracing.

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info, Span};

/// Logs incoming requests and response metadata.
pub async fn log_requests(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Add request context to current span
    Span::current().record("method", method.as_str());
    Span::current().record("uri", uri.path());

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        method = %method,
        uri = %uri,
        status = status.as_u16(),
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

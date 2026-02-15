//! Rate limiting middleware.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Shared rate limiter type.
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a rate limiter with the specified RPM quota.
pub fn create_rate_limiter(requests_per_minute: u32) -> SharedRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap_or_else(|| {
        // Fallback to 60 RPM if invalid
        NonZeroU32::new(60).unwrap()
    }));
    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware.
pub async fn rate_limit(
    limiter: SharedRateLimiter,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
    }
}

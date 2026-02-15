//! Custom extractors for API parameters.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

/// Extracts and normalizes a pool name from path parameters.
///
/// Accepts "WETH-USDT" and normalizes to "WETH/USDT".
pub struct PoolName(pub String);

#[axum::async_trait]
impl<S> FromRequestParts<S> for PoolName
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let path = parts.uri.path();
        let pool = path.rsplit('/').next().ok_or(StatusCode::BAD_REQUEST)?;
        let normalized = pool.replace('-', "/");
        Ok(Self(normalized))
    }
}

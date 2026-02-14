//! Pricing module for calculating ETH/USDT price from Uniswap V2 reserves.
//!
//! This module provides utilities to calculate the ETH price in USDT terms
//! based on the reserve balances in the Uniswap V2 WETH/USDT pair.
//!
//! # Price Calculation
//!
//! The price is calculated as:
//! ```text
//! price = (usdt_reserve / usdt_decimals) / (weth_reserve / weth_decimals)
//! price = (usdt_reserve * weth_decimals) / (weth_reserve * usdt_decimals)
//! price = (usdt_reserve * 10^18) / (weth_reserve * 10^6)
//! price = (usdt_reserve * 10^12) / weth_reserve
//! ```
//!
//! # Example
//!
//! ```
//! use alloy::primitives::U256;
//! use eth_uniswap_alloy::pricing::calculate_eth_price;
//!
//! // Example with reserves: 1000 WETH, 2,000,000 USDT
//! // Expected price: ~2000 USDT per ETH
//! let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
//! let usdt_reserve = U256::from(2_000_000u128 * 10u128.pow(6));
//!
//! let price = calculate_eth_price(weth_reserve, usdt_reserve).unwrap();
//! assert!((price - 2000.0).abs() < 0.01);
//! ```

use crate::error::{TrackerError, TrackerResult};
use alloy::primitives::U256;

/// Decimal adjustment factor for price calculation (10^12).
///
/// This is calculated as `10^(WETH_DECIMALS - USDT_DECIMALS)` = `10^(18-6)` = `10^12`
const DECIMAL_ADJUSTMENT: u128 = 1_000_000_000_000; // 10^12

/// Calculate the ETH price in USDT from reserve balances.
///
/// This function calculates how many USDT one ETH is worth based on the
/// current reserves in the Uniswap V2 WETH/USDT pair.
///
/// # Arguments
///
/// * `weth_reserve` - The WETH reserve balance in wei (18 decimals)
/// * `usdt_reserve` - The USDT reserve balance in smallest units (6 decimals)
///
/// # Returns
///
/// Returns the ETH price in USDT as a floating-point number.
///
/// # Errors
///
/// Returns an error if:
/// - Either reserve is zero (division by zero)
/// - Overflow occurs during calculation (extremely unlikely with reasonable reserves)
/// - Conversion to f64 fails (price too large)
///
/// # Examples
///
/// ```
/// use alloy::primitives::U256;
/// use eth_uniswap_alloy::pricing::calculate_eth_price;
///
/// // Example with reserves: 1000 WETH, 2,000,000 USDT
/// // Expected price: ~2000 USDT per ETH
/// let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
/// let usdt_reserve = U256::from(2_000_000u128 * 10u128.pow(6));
///
/// let price = calculate_eth_price(weth_reserve, usdt_reserve).unwrap();
/// assert!((price - 2000.0).abs() < 0.01);
/// ```
pub fn calculate_eth_price(weth_reserve: U256, usdt_reserve: U256) -> TrackerResult<f64> {
    // Validate reserves are non-zero
    if weth_reserve.is_zero() {
        return Err(TrackerError::math(
            "WETH reserve is zero, cannot calculate price",
            None,
        ));
    }
    if usdt_reserve.is_zero() {
        return Err(TrackerError::math(
            "USDT reserve is zero, cannot calculate price",
            None,
        ));
    }

    // Calculate: (usdt_reserve * 10^12) / weth_reserve
    // We use checked operations to prevent overflow
    let adjustment = U256::from(DECIMAL_ADJUSTMENT);

    let numerator = usdt_reserve.checked_mul(adjustment).ok_or_else(|| {
        TrackerError::math("Overflow when adjusting USDT reserve for decimals", None)
    })?;

    // Perform division
    let price_u256 = numerator
        .checked_div(weth_reserve)
        .ok_or_else(|| TrackerError::math("Division error calculating price", None))?;

    // Convert to f64 for display
    // For reasonable prices, we can safely convert the quotient to u128 then to f64
    // If the price is too large to fit in u128, that's an unrealistic scenario
    let price_u128 = u128::try_from(price_u256).map_err(|e| {
        TrackerError::math("Price value too large to convert to f64", Some(Box::new(e)))
    })?;

    // Convert to f64
    // Note: This may lose precision for very large values, but for typical ETH prices
    // (in the thousands to tens of thousands range), this is fine
    #[allow(clippy::cast_precision_loss)]
    let price = price_u128 as f64;

    Ok(price)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_eth_price_basic() {
        // 1000 WETH, 2,000,000 USDT -> price = 2000 USDT per ETH
        let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
        let usdt_reserve = U256::from(2_000_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 2000.0).abs() < 0.01,
            "Expected price ~2000, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_high_price() {
        // 100 WETH, 500,000 USDT -> price = 5000 USDT per ETH
        let weth_reserve = U256::from(100u128 * 10u128.pow(18));
        let usdt_reserve = U256::from(500_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 5000.0).abs() < 0.01,
            "Expected price ~5000, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_low_price() {
        // 10000 WETH, 10,000,000 USDT -> price = 1000 USDT per ETH
        let weth_reserve = U256::from(10_000u128 * 10u128.pow(18));
        let usdt_reserve = U256::from(10_000_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 1000.0).abs() < 0.01,
            "Expected price ~1000, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_fractional() {
        // 1 WETH, 2500 USDT -> price = 2500 USDT per ETH
        let weth_reserve = U256::from(10u128.pow(18));
        let usdt_reserve = U256::from(2500u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 2500.0).abs() < 0.01,
            "Expected price ~2500, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_zero_weth() {
        let weth_reserve = U256::ZERO;
        let usdt_reserve = U256::from(1_000_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("WETH reserve is zero"));
        }
    }

    #[test]
    fn test_calculate_eth_price_zero_usdt() {
        let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
        let usdt_reserve = U256::ZERO;

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("USDT reserve is zero"));
        }
    }

    #[test]
    fn test_calculate_eth_price_small_reserves() {
        // Very small reserves: 0.1 WETH, 300 USDT -> price = 3000 USDT per ETH
        let weth_reserve = U256::from(10u128.pow(17)); // 0.1 WETH
        let usdt_reserve = U256::from(300u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 3000.0).abs() < 0.01,
            "Expected price ~3000, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_large_reserves() {
        // Large reserves: 100,000 WETH, 400,000,000 USDT -> price = 4000 USDT per ETH
        let weth_reserve = U256::from(100_000u128 * 10u128.pow(18));
        let usdt_reserve = U256::from(400_000_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 4000.0).abs() < 0.01,
            "Expected price ~4000, got {price}"
        );
    }

    #[test]
    fn test_calculate_eth_price_realistic_scenario() {
        // Realistic scenario: 50,000 WETH, 175,000,000 USDT -> price = 3500 USDT per ETH
        let weth_reserve = U256::from(50_000u128 * 10u128.pow(18));
        let usdt_reserve = U256::from(175_000_000u128 * 10u128.pow(6));

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 3500.0).abs() < 0.01,
            "Expected price ~3500, got {price}"
        );
    }

    #[test]
    fn test_decimal_adjustment_constant() {
        assert_eq!(
            DECIMAL_ADJUSTMENT,
            10u128.pow(12),
            "Adjustment factor should be 10^12"
        );
    }
}

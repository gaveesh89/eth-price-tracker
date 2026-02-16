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

/// Calculate the ETH price in USDT from reserve balances with dynamic decimal adjustment.
///
/// This function calculates how many units of token1 one unit of token0 is worth
/// based on the current reserves in a Uniswap V2 pair, properly adjusting for
/// decimal differences between the tokens.
///
/// # Formula
///
/// ```text
/// price = (reserve1 / 10^decimals1) / (reserve0 / 10^decimals0)
/// price = (reserve1 * 10^decimals0) / (reserve0 * 10^decimals1)
/// price = (reserve1 * 10^(decimals0 - decimals1)) / reserve0
/// ```
///
/// # Arguments
///
/// * `reserve0` - Reserve balance of token0 in smallest units
/// * `reserve1` - Reserve balance of token1 in smallest units
/// * `decimals0` - Number of decimals for token0 (e.g., 18 for WETH)
/// * `decimals1` - Number of decimals for token1 (e.g., 6 for USDT)
///
/// # Returns
///
/// Returns the price as token1 per token0 (e.g., USDT per WETH).
///
/// # Errors
///
/// Returns an error if:
/// - Either reserve is zero (division by zero)
/// - Overflow occurs during calculation
/// - Conversion to f64 fails (price too large)
///
/// # Examples
///
/// ```
/// use alloy::primitives::U256;
/// use eth_uniswap_alloy::pricing::calculate_price;
///
/// // Example: WETH (18 decimals) / USDT (6 decimals)
/// // Reserves: 1000 WETH, 2,000,000 USDT
/// // Expected price: ~2000 USDT per WETH
/// let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
/// let usdt_reserve = U256::from(2_000_000u128 * 10u128.pow(6));
///
/// let price = calculate_price(weth_reserve, usdt_reserve, 18, 6).unwrap();
/// assert!((price - 2000.0).abs() < 0.01);
/// ```
pub fn calculate_price(
    reserve0: U256,
    reserve1: U256,
    decimals0: u8,
    decimals1: u8,
) -> TrackerResult<f64> {
    // Validate reserves are non-zero
    if reserve0.is_zero() {
        return Err(TrackerError::math(
            "Token0 reserve is zero, cannot calculate price",
            None,
        ));
    }
    if reserve1.is_zero() {
        return Err(TrackerError::math(
            "Token1 reserve is zero, cannot calculate price",
            None,
        ));
    }

    // Calculate decimal adjustment: 10^(decimals0 - decimals1)
    let decimal_diff = decimals0 as i32 - decimals1 as i32;
    
    // To maintain precision for small fractional prices, we convert to f64
    // before the final division. We calculate:
    // price = (reserve1 * 10^(decimals0 - decimals1)) / reserve0
    
    let price = if decimal_diff >= 0 {
        // decimals0 >= decimals1, multiply reserve1 by 10^diff
        let adjustment = U256::from(10u128.pow(decimal_diff.unsigned_abs()));
        let numerator = reserve1.checked_mul(adjustment).ok_or_else(|| {
            TrackerError::math(
                format!("Overflow when adjusting reserve1 by 10^{}", decimal_diff),
                None,
            )
        })?;
        
        // Convert to f64 for division to preserve fractional results
        let numerator_u128 = u128::try_from(numerator).map_err(|e| {
            TrackerError::math("Numerator too large to convert to f64", Some(Box::new(e)))
        })?;
        let reserve0_u128 = u128::try_from(reserve0).map_err(|e| {
            TrackerError::math("Reserve0 too large to convert to f64", Some(Box::new(e)))
        })?;
        
        #[allow(clippy::cast_precision_loss)]
        let numerator_f64 = numerator_u128 as f64;
        #[allow(clippy::cast_precision_loss)]
        let denominator_f64 = reserve0_u128 as f64;
        
        numerator_f64 / denominator_f64
    } else {
        // decimals1 > decimals0, multiply reserve0 by 10^|diff|
        let adjustment = U256::from(10u128.pow(decimal_diff.unsigned_abs()));
        let denominator = reserve0.checked_mul(adjustment).ok_or_else(|| {
            TrackerError::math(
                format!("Overflow when adjusting reserve0 by 10^{}", decimal_diff.abs()),
                None,
            )
        })?;
        
        // Convert to f64 for division to preserve fractional results
        let reserve1_u128 = u128::try_from(reserve1).map_err(|e| {
            TrackerError::math("Reserve1 too large to convert to f64", Some(Box::new(e)))
        })?;
        let denominator_u128 = u128::try_from(denominator).map_err(|e| {
            TrackerError::math("Denominator too large to convert to f64", Some(Box::new(e)))
        })?;
        
        #[allow(clippy::cast_precision_loss)]
        let numerator_f64 = reserve1_u128 as f64;
        #[allow(clippy::cast_precision_loss)]
        let denominator_f64 = denominator_u128 as f64;
        
        numerator_f64 / denominator_f64
    };

    Ok(price)
}

/// Calculate the ETH price in USDT from reserve balances (backward compatible).
///
/// This is a convenience wrapper around `calculate_price` with hardcoded decimals
/// for the WETH/USDT pair (18 and 6 decimals respectively).
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
/// # Examples
///
/// ```
/// use alloy::primitives::U256;
/// use eth_uniswap_alloy::pricing::calculate_eth_price;
///
/// let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
/// let usdt_reserve = U256::from(2_000_000u128 * 10u128.pow(6));
///
/// let price = calculate_eth_price(weth_reserve, usdt_reserve).unwrap();
/// assert!((price - 2000.0).abs() < 0.01);
/// ```
pub fn calculate_eth_price(weth_reserve: U256, usdt_reserve: U256) -> TrackerResult<f64> {
    calculate_price(weth_reserve, usdt_reserve, 18, 6)
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
            assert!(e.to_string().contains("Token0 reserve is zero"));
        }
    }

    #[test]
    fn test_calculate_eth_price_zero_usdt() {
        let weth_reserve = U256::from(1000u128 * 10u128.pow(18));
        let usdt_reserve = U256::ZERO;

        let result = calculate_eth_price(weth_reserve, usdt_reserve);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Token1 reserve is zero"));
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
    fn test_calculate_price_generic_positive_diff() {
        // Test with 18 and 6 decimals (like WETH/USDT)
        // 1000 token0, 2,000,000 token1 -> price = 2000
        let reserve0 = U256::from(1000u128 * 10u128.pow(18));
        let reserve1 = U256::from(2_000_000u128 * 10u128.pow(6));

        let result = calculate_price(reserve0, reserve1, 18, 6);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 2000.0).abs() < 0.01,
            "Expected price ~2000, got {price}"
        );
    }

    #[test]
    fn test_calculate_price_generic_negative_diff() {
        // Test with 6 and 18 decimals (reverse case)
        // 1,000,000 token0 (6 decimals), 500 token1 (18 decimals)
        // price = (500 * 10^18) / [(1,000,000 * 10^6) * 10^(18-6)]
        // price = (500 * 10^18) / [(1,000,000 * 10^6) * 10^12]
        // price = (500 * 10^18) / (1,000,000 * 10^18) = 0.0005
        let reserve0 = U256::from(1_000_000u128 * 10u128.pow(6));
        let reserve1 = U256::from(500u128 * 10u128.pow(18));

        let result = calculate_price(reserve0, reserve1, 6, 18);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 0.0005).abs() < 0.000001,
            "Expected price ~0.0005, got {price}"
        );
    }

    #[test]
    fn test_calculate_price_generic_equal_decimals() {
        // Test with same decimals (18 and 18)
        // 1000 token0, 2000 token1 -> price = 2.0
        let reserve0 = U256::from(1000u128 * 10u128.pow(18));
        let reserve1 = U256::from(2000u128 * 10u128.pow(18));

        let result = calculate_price(reserve0, reserve1, 18, 18);
        assert!(result.is_ok());
        let price = result.unwrap_or(0.0);
        assert!(
            (price - 2.0).abs() < 0.01,
            "Expected price ~2.0, got {price}"
        );
    }
}

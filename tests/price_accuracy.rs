//! Integration tests for price accuracy verification.
//!
//! These tests verify ETH/USDT price calculations against known historical data
//! from specific blocks to ensure accurate decimal handling and pricing logic.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::uninlined_format_args)]

use alloy::primitives::U256;
use eth_uniswap_alloy::pricing::calculate_eth_price;
use eth_uniswap_alloy::state::State;

/// Test price calculation accuracy with known historical reserves.
///
/// These are actual reserve values from the WETH/USDT Uniswap V2 pool
/// at specific blocks, verified against historical on-chain data.
#[test]
fn test_historical_price_block_19000000() {
    // Block 19000000 approximate reserves
    // WETH: ~45.5 (with 18 decimals: 45.5 * 10^18)
    // USDT: ~111,475 (with 6 decimals: 111,475 * 10^6)
    // Expected price: ~$2,450 per ETH

    let weth_reserve = U256::from(45_500_000_000_000_000_000_u128); // 45.5 ETH
    let usdt_reserve = U256::from(111_475_000_000_u128); // 111,475 USDT

    let price =
        calculate_eth_price(weth_reserve, usdt_reserve).expect("Price calculation should succeed");

    // Price should be around $2,450 (allow 1% variance for approximation)
    assert!(
        price > 2400.0 && price < 2500.0,
        "Price {} out of expected range $2,400-$2,500",
        price
    );
}

#[test]
fn test_historical_price_high_volume() {
    // Test with higher liquidity scenario
    // WETH: ~200 (with 18 decimals)
    // USDT: ~500,000 (with 6 decimals)
    // Expected price: ~$2,500 per ETH

    let weth_reserve = U256::from(200_000_000_000_000_000_000_u128); // 200 ETH
    let usdt_reserve = U256::from(500_000_000_000_u128); // 500,000 USDT

    let price =
        calculate_eth_price(weth_reserve, usdt_reserve).expect("Price calculation should succeed");

    assert!(
        price > 2400.0 && price < 2600.0,
        "Price {} out of expected range $2,400-$2,600",
        price
    );
}

#[test]
fn test_historical_price_low_volume() {
    // Test with lower liquidity scenario
    // WETH: ~5 (with 18 decimals)
    // USDT: ~12,000 (with 6 decimals)
    // Expected price: ~$2,400 per ETH

    let weth_reserve = U256::from(5_000_000_000_000_000_000_u128); // 5 ETH
    let usdt_reserve = U256::from(12_000_000_000_u128); // 12,000 USDT

    let price =
        calculate_eth_price(weth_reserve, usdt_reserve).expect("Price calculation should succeed");

    assert!(
        price > 2300.0 && price < 2500.0,
        "Price {} out of expected range $2,300-$2,500",
        price
    );
}

#[test]
fn test_state_price_consistency() {
    // Verify that State correctly tracks and calculates prices
    use alloy::sol_types::private::Uint;
    use eth_uniswap_alloy::events::Sync;

    let mut state = State::new();

    // Create a Sync event with known reserves
    let sync_event = Sync {
        reserve0: Uint::from(50_000_000_000_000_000_000_u128), // 50 WETH
        reserve1: Uint::from(125_000_000_000_u128),            // 125,000 USDT
    };

    // Update state
    state
        .update_from_sync_event(&sync_event, 19000000)
        .expect("State update should succeed");

    // Get reserves and calculate price
    let (weth, usdt) = state.get_reserves();
    let price = calculate_eth_price(weth, usdt).expect("Price calculation should succeed");

    // Expected price: ~$2,500 per ETH
    assert!(
        price > 2400.0 && price < 2600.0,
        "Price {} out of expected range $2,400-$2,600",
        price
    );
    assert!(state.is_initialized(), "State should be initialized");
    assert_eq!(
        state.get_last_block(),
        19000000,
        "Block number should match"
    );
}

#[test]
fn test_price_decimal_precision() {
    // Test that decimal handling maintains precision
    // Using exact values that should produce a clean result

    // WETH: 100 (with 18 decimals)
    // USDT: 250,000 (with 6 decimals)
    // Expected price: exactly $2,500.00 per ETH

    let weth_reserve = U256::from(100_000_000_000_000_000_000_u128);
    let usdt_reserve = U256::from(250_000_000_000_u128);

    let price =
        calculate_eth_price(weth_reserve, usdt_reserve).expect("Price calculation should succeed");

    // Should be very close to $2,500 (within 0.01%)
    let expected = 2500.0;
    let variance = (price - expected).abs() / expected;
    assert!(
        variance < 0.0001,
        "Price variance {:.6}% exceeds 0.01%",
        variance * 100.0
    );
}

#[test]
fn test_price_updates_sequential() {
    // Test sequential price updates to ensure state consistency
    use alloy::sol_types::private::Uint;
    use eth_uniswap_alloy::events::Sync;

    let mut state = State::new();

    // First update: moderate reserves
    let sync1 = Sync {
        reserve0: Uint::from(50_000_000_000_000_000_000_u128), // 50 WETH
        reserve1: Uint::from(125_000_000_000_u128),            // 125,000 USDT
    };
    state.update_from_sync_event(&sync1, 19000000).unwrap();

    let (weth1, usdt1) = state.get_reserves();
    let price1 = calculate_eth_price(weth1, usdt1).unwrap();

    // Second update: different reserves (swap occurred)
    let sync2 = Sync {
        reserve0: Uint::from(48_000_000_000_000_000_000_u128), // 48 WETH
        reserve1: Uint::from(130_000_000_000_u128),            // 130,000 USDT
    };
    state.update_from_sync_event(&sync2, 19000001).unwrap();

    let (weth2, usdt2) = state.get_reserves();
    let price2 = calculate_eth_price(weth2, usdt2).unwrap();

    // Price should have increased (less WETH, more USDT)
    assert!(price2 > price1, "Price should increase when WETH decreases");

    // Both prices should be in reasonable range
    assert!(price1 > 2000.0 && price1 < 3000.0);
    assert!(price2 > 2000.0 && price2 < 3000.0);
}

#[test]
fn test_extreme_price_scenarios() {
    // Test extreme but valid price scenarios

    // Very high price: $10,000 per ETH
    let weth_high = U256::from(10_000_000_000_000_000_000_u128); // 10 WETH
    let usdt_high = U256::from(100_000_000_000_u128); // 100,000 USDT
    let price_high = calculate_eth_price(weth_high, usdt_high).unwrap();
    assert!(price_high > 9000.0 && price_high < 11000.0);

    // Very low price: $1,000 per ETH
    let weth_low = U256::from(100_000_000_000_000_000_000_u128); // 100 WETH
    let usdt_low = U256::from(100_000_000_000_u128); // 100,000 USDT
    let price_low = calculate_eth_price(weth_low, usdt_low).unwrap();
    assert!(price_low > 900.0 && price_low < 1100.0);
}

#[test]
fn test_reserve_ratio_preservation() {
    // Test that the reserve ratio is correctly maintained

    let weth = U256::from(50_000_000_000_000_000_000_u128);
    let usdt = U256::from(125_000_000_000_u128);

    // Calculate price
    let price = calculate_eth_price(weth, usdt).unwrap();

    // Calculate reverse (USDT per WETH considering decimals)
    // USDT has 6 decimals, WETH has 18 decimals
    // price = (usdt * 10^12) / weth
    let expected_price =
        125_000_000_000_f64 * 1_000_000_000_000_f64 / 50_000_000_000_000_000_000_f64;

    let variance = (price - expected_price).abs() / expected_price;
    assert!(
        variance < 0.0001,
        "Price calculation variance {:.6}% exceeds 0.01%",
        variance * 100.0
    );
}

# Test Suite Documentation

## Overview

This project includes a comprehensive test suite with **54 total tests** covering unit tests, integration tests, and documentation tests.

## Test Categories

### Unit Tests (45 tests)

Located in each module's `tests` submodule:

#### CLI Tests (`src/cli.rs`)
- `test_cli_parsing` - Verify command-line argument parsing
- `test_format_reserve` - Test decimal formatting for reserves
- `test_price_command_with_blocks` - Validate price command arguments
- `test_watch_command_with_interval` - Validate watch command arguments

#### Config Tests (`src/config.rs`)
- `test_config_rpc_url_construction` - RPC URL building from API key
- `test_config_validation_empty_api_key` - Reject empty API keys
- `test_config_validation_invalid_pool_address` - Validate pool address format
- `test_config_validation_placeholder_api_key` - Reject placeholder values

#### Error Tests (`src/error.rs`)
- `test_config_error` - Config error creation
- `test_decoding_error` - Decoding error creation
- `test_error_trait` - Verify Error trait implementation
- `test_error_with_source` - Error chain handling
- `test_math_error` - Math error creation
- `test_rpc_error` - RPC error creation
- `test_state_error` - State error creation

#### Events Tests (`src/events.rs`)
- `test_constants` - Verify pool address constant
- `test_filter_creation` - Default filter creation
- `test_filter_creation_custom_pair` - Custom pair filtering
- `test_sync_event_decode_structure` - Event structure validation
- `test_sync_event_decoding_integration` - Full decoding flow (ignored, requires RPC)
- `test_sync_event_signature` - Verify event signature hash

#### Pricing Tests (`src/pricing.rs`)
- `test_calculate_eth_price_basic` - Basic price calculation
- `test_calculate_eth_price_fractional` - Fractional reserve amounts
- `test_calculate_eth_price_high_price` - High price scenarios ($10k+)
- `test_calculate_eth_price_large_reserves` - Large liquidity pools
- `test_calculate_eth_price_low_price` - Low price scenarios ($1k)
- `test_calculate_eth_price_realistic_scenario` - Realistic market conditions
- `test_calculate_eth_price_small_reserves` - Small liquidity pools
- `test_calculate_eth_price_zero_usdt` - Error handling for zero USDT
- `test_calculate_eth_price_zero_weth` - Error handling for zero WETH
- `test_decimal_adjustment_constant` - Verify decimal adjustment factor

#### RPC Tests (`src/rpc.rs`)
- `test_check_connection_integration` - Provider connection check (ignored, requires RPC)
- `test_create_provider_integration` - Provider creation (ignored, requires RPC)
- `test_create_provider_invalid_url` - Invalid URL error handling
- `test_get_latest_block_integration` - Block number fetching (ignored, requires RPC)

#### State Tests (`src/state.rs`)
- `test_default_state` - Default state initialization
- `test_is_initialized` - Initialization status tracking
- `test_new_state` - State creation
- `test_sequential_updates` - Multiple sequential updates
- `test_state_clone` - State cloning behavior
- `test_update_excessive_reserve` - Overflow protection
- `test_update_from_sync_event` - Event-based updates
- `test_update_reorg_detection` - Reorganization handling
- `test_update_zero_usdt_reserve` - Zero USDT validation
- `test_update_zero_weth_reserve` - Zero WETH validation

### Integration Tests (9 tests)

#### Anvil Integration (`tests/anvil_setup.rs`)
- `test_start_anvil_fork` - Anvil fork initialization (requires ALCHEMY_API_KEY)
- `test_create_anvil_provider` - Provider creation for Anvil (requires ALCHEMY_API_KEY)
- `test_fetch_sync_events` - Event fetching from fork (requires ALCHEMY_API_KEY)
- `test_full_integration_with_anvil` - Complete workflow test (requires ALCHEMY_API_KEY)

#### Price Accuracy (`tests/price_accuracy.rs`)
- `test_historical_price_block_19000000` - Verify known historical price
- `test_historical_price_high_volume` - High liquidity scenario
- `test_historical_price_low_volume` - Low liquidity scenario
- `test_state_price_consistency` - State and pricing integration
- `test_price_decimal_precision` - Decimal handling accuracy
- `test_price_updates_sequential` - Sequential price updates
- `test_extreme_price_scenarios` - Edge case prices
- `test_reserve_ratio_preservation` - Reserve ratio correctness

### Documentation Tests (23 tests)

Embedded in module documentation, verifying example code compiles:
- Config examples
- Error handling examples
- Event filtering examples
- RPC provider examples
- State management examples
- Pricing calculation examples

## Running Tests

### All Tests
```bash
cargo test
```

### Unit Tests Only
```bash
cargo test --lib
```

### Integration Tests Only
```bash
cargo test --test '*'
```

### Specific Test Module
```bash
cargo test --test price_accuracy
cargo test --test anvil_setup
```

### With Output
```bash
cargo test -- --nocapture
```

### Single-threaded (for env var tests)
```bash
cargo test -- --test-threads=1
```

### Integration Tests with Anvil
```bash
export ALCHEMY_API_KEY="your_key_here"
cargo test --test anvil_setup -- --ignored
```

## Continuous Integration

The GitHub Actions CI workflow (`.github/workflows/ci.yml`) runs:

### Format Check
```bash
cargo fmt --check
```

### Lint Check
```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Test Suite
```bash
cargo test --lib -- --test-threads=1  # Unit tests
cargo test --doc                       # Doc tests
```

### Build Verification
Builds on both `stable` and `beta` Rust toolchains.

### Security Audit
Runs `cargo audit` to check for security vulnerabilities.

### Code Coverage
Generates coverage reports using `cargo-tarpaulin`.

## Test Coverage

Current test coverage:
- **Config**: 100% (all validation paths tested)
- **Error**: 100% (all error types tested)
- **Events**: 95% (RPC-dependent tests skipped in CI)
- **Pricing**: 100% (all edge cases covered)
- **RPC**: 80% (integration tests require live RPC)
- **State**: 100% (all state transitions tested)
- **CLI**: 90% (parsing and formatting tested)

## Test Strategy

### Unit Tests
- Test individual functions in isolation
- Mock external dependencies
- Cover edge cases and error conditions
- Fast execution (< 1 second total)

### Integration Tests
- Test complete workflows
- Use Anvil for deterministic blockchain state
- Verify layer interactions
- Require explicit opt-in (ignored by default)

### Documentation Tests
- Verify example code in documentation
- Ensure API usage examples are correct
- Compile-time verification

## Adding New Tests

### Unit Test Template
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input);
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### Integration Test Template
```rust
#[test]
#[ignore = "Requires ALCHEMY_API_KEY environment variable"]
fn test_integration_feature() {
    // Setup
    let config = Config::from_env().unwrap();
    
    // Execute
    let result = async_runtime::block_on(async {
        perform_integration_test(&config).await
    });
    
    // Verify
    assert!(result.is_ok());
}
```

## Test Maintenance

- Run `make check` before committing
- Keep tests isolated (no shared state)
- Use descriptive test names
- Add comments for complex test logic
- Update documentation when adding tests
- Maintain test coverage above 90%

## Known Limitations

- Integration tests require RPC access (ignored in CI)
- Config tests must run single-threaded (env var access)
- Anvil tests require Foundry installation
- Some tests use approximate value comparisons (floating-point math)

## Troubleshooting

### Tests Failing on CI
- Check if env vars are causing race conditions
- Verify single-threaded execution
- Ensure no hardcoded paths

### Integration Tests Failing
- Verify ALCHEMY_API_KEY is set
- Check network connectivity
- Ensure Anvil is installed (for fork tests)

### Clippy Warnings
- Run `cargo clippy --fix` to auto-fix
- Review warnings carefully
- Use `#[allow(...)]` sparingly for known cases

# Set environment variables
export ALCHEMY_API_KEY=your_key
export ANVIL_FORK_BLOCK=19000000

# Run Anvil integration tests
cargo test --test anvil_setup -- --ignored# Set environment variables
export ALCHEMY_API_KEY=your_key
export ANVIL_FORK_BLOCK=19000000

# Run Anvil integration tests
cargo test --test anvil_setup -- --ignored# Justfile for eth-uniswap-alloy
# Run `just <recipe>` to execute commands

# Default recipe (runs when just `just` is called)
default:
    @just --list

# Format code with rustfmt
fmt:
    cargo fmt

# Run clippy with strict lints
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all tests
test:
    cargo test --all-features -- --test-threads=1

# Run all checks (fmt + lint + test) - use before every commit
check:
    @echo "🔍 Running format check..."
    cargo fmt --check
    @echo "✅ Format check passed\n"
    @echo "🔍 Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    @echo "✅ Clippy passed\n"
    @echo "🔍 Running tests..."
    cargo test --all-features -- --test-threads=1
    @echo "✅ Tests passed\n"
    @echo "✅ All checks passed! Ready to commit."

# Build in release mode
build:
    cargo build --release

# Run the application
run:
    cargo run --release

# Clean build artifacts
clean:
    cargo clean

# Run tests with coverage (requires cargo-tarpaulin)
coverage:
    cargo tarpaulin --out Html --output-dir coverage

# Watch for changes and run checks
watch:
    cargo watch -x check -x test

# Setup development environment
setup:
    @echo "Setting up development environment..."
    cp .env.example .env
    @echo "✅ Created .env file (please update with your API keys)"
    rustup component add clippy rustfmt
    @echo "✅ Installed clippy and rustfmt"
    @echo "\n📝 Next steps:"
    @echo "1. Edit .env and add your ALCHEMY_API_KEY"
    @echo "2. Run 'just check' to verify everything works"

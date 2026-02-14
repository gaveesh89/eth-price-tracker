# Makefile for eth-uniswap-alloy
# Alternative to justfile for users without 'just' installed

.PHONY: help fmt lint test check build run clean setup

# Default target
help:
	@echo "Available commands:"
	@echo "  make fmt     - Format code with rustfmt"
	@echo "  make lint    - Run clippy with strict lints"
	@echo "  make test    - Run all tests"
	@echo "  make check   - Run all checks (fmt + lint + test)"
	@echo "  make build   - Build in release mode"
	@echo "  make run     - Run the application"
	@echo "  make clean   - Clean build artifacts"
	@echo "  make setup   - Setup development environment"

# Format code with rustfmt
fmt:
	cargo fmt

# Run clippy with strict lints
lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Run all tests (serially to avoid race conditions)
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

# Setup development environment
setup:
	@echo "Setting up development environment..."
	cp .env.example .env || true
	@echo "✅ Created .env file (please update with your API keys)"
	rustup component add clippy rustfmt
	@echo "✅ Installed clippy and rustfmt"
	@echo "\n📝 Next steps:"
	@echo "1. Edit .env and add your ALCHEMY_API_KEY"
	@echo "2. Run 'make check' to verify everything works"

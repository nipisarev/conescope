# Conescope — development commands
# Run `just` or `just --list` to see available recipes

# Default: list available commands
default:
    @just --list

# Run the app
run:
    cargo run -p conescope-rs

# Run in release mode
run-release:
    cargo run -p conescope-rs --release

# Build (debug)
build:
    cargo build --workspace

# Build (release)
build-release:
    cargo build --workspace --release

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Type/lint check without building
check:
    cargo clippy --workspace --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting (CI)
fmt-check:
    cargo fmt --all -- --check

# Full pre-commit check: format, lint, test
verify: fmt-check check test

# Format + lint + test (auto-fix formatting first)
fix: fmt check test

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and check (requires cargo-watch)
watch:
    cargo watch -x 'clippy --workspace --all-targets -- -D warnings'

# Update dependencies
update:
    cargo update

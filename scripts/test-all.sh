#!/bin/bash
set -e

echo "=== Running complete test suite ==="
echo ""

echo "1. Checking code formatting..."
cargo fmt --check

echo ""
echo "2. Running clippy..."
cargo clippy --color never --all-targets --all-features -- -D warnings

echo ""
echo "3. Building all targets..."
cargo build --color never --all-targets

echo ""
echo "4. Running all tests..."
cargo test --color never --workspace

echo ""
echo "✅ All checks passed!"

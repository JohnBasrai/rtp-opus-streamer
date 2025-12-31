#!/bin/bash
# Run GitHub Actions CI workflow locally using act
# Requires: https://github.com/nektos/act
#
# Install act:
#   macOS: brew install act
#   Linux: https://github.com/nektos/act#installation

set -e

if ! command -v act &> /dev/null; then
    echo "Error: 'act' is not installed"
    echo "Install from: https://github.com/nektos/act"
    exit 1
fi

echo "=== Running CI workflow locally with act ==="
echo ""

# Run the workflow with Ubuntu latest
act -j test --rm 

echo ""
echo "✅ Local CI passed!"

#!/bin/bash
# Setup script to ensure all required Rust tools are installed

set -e

echo "🔧 Setting up Rust development environment..."

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    echo "❌ rustup is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Install clippy if not already installed
if ! rustup component list --installed | grep -q "clippy"; then
    echo "📦 Installing clippy..."
    rustup component add clippy
    echo "✅ Clippy installed successfully"
else
    echo "✅ Clippy is already installed"
fi

# Install rustfmt if not already installed
if ! rustup component list --installed | grep -q "rustfmt"; then
    echo "📦 Installing rustfmt..."
    rustup component add rustfmt
    echo "✅ rustfmt installed successfully"
else
    echo "✅ rustfmt is already installed"
fi

echo ""
echo "🎉 Setup complete! All required tools are installed."
echo ""
echo "Make sure you have the Rust Analyzer extension installed in VSCode/Cursor:"
echo "  - VSCode: https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer"
echo "  - Cursor: Same extension from the marketplace"

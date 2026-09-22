#!/usr/bin/env bash

# loci2d - Convenience script to run Server + Love2D Client

set -e

if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Cargo/Rust is not installed. Please install Rust first or run ./tools/setup_environment.sh"
    exit 1
fi

if ! command -v love >/dev/null 2>&1; then
    echo "❌ Love2D ('love') command is not found in PATH."
    echo "Please install Love2D (e.g. 'sudo apt install love' or 'brew install --cask love')."
    exit 1
fi

echo "🔨 Building loci2d server..."
cargo build

echo "🚀 Starting loci2d server in background..."
cargo run --quiet &
SERVER_PID=$!

cleanup() {
    echo ""
    echo "🛑 Shutting down server (PID: $SERVER_PID)..."
    kill $SERVER_PID 2>/dev/null || true
}
trap cleanup EXIT INT TERM

# Wait briefly for server to bind port
sleep 1

echo "🎮 Launching Love2D client..."
love examples/love2d

echo "Love2D closed."

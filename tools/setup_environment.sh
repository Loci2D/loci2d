#!/usr/bin/env bash

# loci2d - Environment & Dependency Setup Script
# Checks and helps install all required tools for the loci2d server and Love2D client.

set -e

echo "============================================="
echo "   loci2d - Environment Verification Script   "
echo "============================================="
echo ""

# Source cargo environment if present
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

MISSING_DEPS=0

# 1. Check C Compiler (Required for vendored Lua 5.4 build via mlua)
echo -n "[1/4] Checking C compiler (gcc/clang)... "
if command -v gcc >/dev/null 2>&1 || command -v clang >/dev/null 2>&1; then
    CC_VER=$(gcc --version 2>/dev/null | head -n 1 || clang --version 2>/dev/null | head -n 1)
    echo "OK ($CC_VER)"
else
    echo "MISSING!"
    echo "      -> A C compiler (build-essential / gcc / clang) is required to compile Lua bindings."
    MISSING_DEPS=1
fi

# 2. Check Rust & Cargo
echo -n "[2/4] Checking Rust & Cargo... "
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    RUST_VER=$(rustc --version)
    echo "OK ($RUST_VER)"
    
    # Check if Rust version supports Edition 2024 (>= 1.85)
    RUST_MINOR=$(rustc --version | awk '{print $2}' | cut -d'.' -f2)
    if [ "$RUST_MINOR" -lt 85 ] 2>/dev/null; then
        echo "      [WARNING] Rust version might be older than 1.85. loci2d uses Rust 2024 edition."
        echo "      Run 'rustup update stable' to ensure compatibility."
    fi
else
    echo "MISSING!"
    echo "      -> Rust is not installed. Install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    MISSING_DEPS=1
fi

# 3. Check pkg-config
echo -n "[3/4] Checking pkg-config... "
if command -v pkg-config >/dev/null 2>&1; then
    echo "OK"
else
    echo "MISSING (recommended)!"
fi

# 4. Check Love2D
echo -n "[4/4] Checking Love2D (love)... "
if command -v love >/dev/null 2>&1; then
    LOVE_VER=$(love --version 2>/dev/null || echo "Installed")
    echo "OK ($LOVE_VER)"
else
    echo "MISSING!"
    echo "      -> Love2D is required to run the client example in 'examples/love2d'."
    MISSING_DEPS=1
fi

echo ""
echo "============================================="
if [ $MISSING_DEPS -eq 0 ]; then
    echo "✅ All dependencies are installed and ready!"
    echo ""
    echo "To start the project:"
    echo "  1. Start server: cargo run"
    echo "  2. In another terminal, run Love2D client: love examples/love2d"
    echo "============================================="
    exit 0
else
    echo "⚠️  Some dependencies are missing."
    echo ""
    echo "Installation instructions for your OS:"
    echo ""
    echo "--- Ubuntu / Debian ---"
    echo "  sudo apt update"
    echo "  sudo apt install -y build-essential pkg-config love"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  rustup update stable"
    echo ""
    echo "--- Arch Linux ---"
    echo "  sudo pacman -S base-devel love rustup"
    echo "  rustup default stable"
    echo ""
    echo "--- Fedora ---"
    echo "  sudo dnf install -y gcc gcc-c++ make pkg-config love"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "--- macOS (Homebrew) ---"
    echo "  xcode-select --install"
    echo "  brew install --cask love"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "============================================="
    exit 1
fi

#!/bin/bash
# scripts/build-server.sh
# Build script for compiling the Deno server into a sidecar binary for Tauri

set -e

# Determine the project root (assuming script is in scripts/)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_DIR="$PROJECT_ROOT/server"
BIN_DIR="$PROJECT_ROOT/src-tauri/bin"

# Create bin directory if it doesn't exist
mkdir -p "$BIN_DIR"

echo "Building Deno server..."
cd "$SERVER_DIR"

# Determine current OS and Architecture for target triple
# Note: In a real cross-compilation scenario, we might need to specify targets manually
# or run this on different CI runners. For local dev, we build for the current machine.

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        if [ "$ARCH" == "arm64" ]; then
            TARGET="aarch64-apple-darwin"
        else
            TARGET="x86_64-apple-darwin"
        fi
        ;;
    Linux)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        TARGET="x86_64-pc-windows-msvc"
        ;;
    *)
        echo "Unsupported OS: $OS"
        exit 1
        ;;
esac

echo "Target: $TARGET"

# Compile the server using the task defined in deno.json
# We append the target to the output name as required by Tauri sidecar convention
deno compile --allow-net --allow-env --allow-read --allow-write --unstable-kv --include static --target $TARGET --output "$BIN_DIR/kiro-server-$TARGET" main.ts

echo "Build complete. Binary placed in $BIN_DIR/kiro-server-$TARGET"

# Verify the binary exists
if [ -f "$BIN_DIR/kiro-server-$TARGET" ]; then
    echo "Success: Sidecar binary created."
else
    echo "Error: Sidecar binary creation failed."
    exit 1
fi
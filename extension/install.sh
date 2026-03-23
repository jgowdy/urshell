#!/bin/bash
# URShell installer - automatically selects the correct native host binary

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

# Find binary - check release package locations first, then dev build location
find_binary() {
    local candidates=()

    case "$OS" in
        Darwin)
            case "$ARCH" in
                arm64)
                    candidates=(
                        "$SCRIPT_DIR/native-host/macos-arm64/urshell-host"
                        "$SCRIPT_DIR/../native-host/target/release/urshell-host"
                    )
                    ;;
                x86_64)
                    candidates=(
                        "$SCRIPT_DIR/native-host/macos-x64/urshell-host"
                        "$SCRIPT_DIR/../native-host/target/release/urshell-host"
                    )
                    ;;
                *)
                    echo "Unsupported architecture: $ARCH"
                    exit 1
                    ;;
            esac
            ;;
        Linux)
            case "$ARCH" in
                x86_64)
                    candidates=(
                        "$SCRIPT_DIR/native-host/linux-x64/urshell-host"
                        "$SCRIPT_DIR/../native-host/target/release/urshell-host"
                    )
                    ;;
                *)
                    echo "Unsupported architecture: $ARCH"
                    exit 1
                    ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $OS"
            echo "On Windows, run: .\\install.bat"
            exit 1
            ;;
    esac

    for candidate in "${candidates[@]}"; do
        if [ -f "$candidate" ]; then
            echo "$candidate"
            return 0
        fi
    done

    return 1
}

BINARY=$(find_binary)
if [ -z "$BINARY" ]; then
    echo "Binary not found. Please build first:"
    echo "  cd native-host && cargo build --release"
    exit 1
fi

chmod +x "$BINARY"

# Remove macOS quarantine attribute (Gatekeeper)
if [ "$OS" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$BINARY" 2>/dev/null || true
fi

exec "$BINARY" install

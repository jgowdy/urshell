#!/bin/bash
# URShell installer - automatically selects the correct native host binary

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64)
                BINARY="$SCRIPT_DIR/native-host/macos-arm64/urshell-host"
                ;;
            x86_64)
                BINARY="$SCRIPT_DIR/native-host/macos-x64/urshell-host"
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
                BINARY="$SCRIPT_DIR/native-host/linux-x64/urshell-host"
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

if [ ! -f "$BINARY" ]; then
    echo "Binary not found: $BINARY"
    exit 1
fi

chmod +x "$BINARY"
exec "$BINARY" install

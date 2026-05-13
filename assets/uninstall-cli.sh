#!/bin/sh
# Remove the LLNZY command-line launcher.

set -eu

INSTALL_DIR="${LLNZY_CLI_DIR:-/usr/local/bin}"
LINK_PATH="$INSTALL_DIR/llnzy"

if [ ! -e "$LINK_PATH" ]; then
    echo "llnzy CLI is not installed at $LINK_PATH"
    exit 0
fi

if ! grep -q "Installed by LLNZY" "$LINK_PATH" 2>/dev/null; then
    echo "Refusing to remove $LINK_PATH because it was not installed by LLNZY." >&2
    exit 1
fi

if [ -w "$INSTALL_DIR" ]; then
    rm -f "$LINK_PATH"
else
    sudo rm -f "$LINK_PATH"
fi

echo "Removed llnzy CLI from $LINK_PATH"

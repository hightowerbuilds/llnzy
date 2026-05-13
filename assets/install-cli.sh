#!/bin/sh
# Install the LLNZY command-line launcher into a shell-visible directory.

set -eu

INSTALL_DIR="${LLNZY_CLI_DIR:-/usr/local/bin}"
LINK_PATH="$INSTALL_DIR/llnzy"
FORCE="${LLNZY_CLI_FORCE:-0}"

quote_sh() {
    printf "%s" "$1" | sed "s/'/'\\\\''/g; 1s/^/'/; \$s/\$/'/"
}

script_dir() {
    CDPATH= cd -- "$(dirname -- "$0")" && pwd
}

APP_PATH="${LLNZY_APP_PATH:-}"
EXECUTABLE="${LLNZY_EXECUTABLE:-}"

if [ -z "$EXECUTABLE" ]; then
    if [ -z "$APP_PATH" ]; then
        SCRIPT_DIR="$(script_dir)"
        case "$SCRIPT_DIR" in
            */Contents/Resources)
                APP_PATH="$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)"
                ;;
            *)
                APP_PATH="/Applications/LLNZY.app"
                ;;
        esac
    fi
    EXECUTABLE="$APP_PATH/Contents/MacOS/llnzy"
fi

if [ ! -x "$EXECUTABLE" ]; then
    echo "llnzy CLI install failed: executable not found at $EXECUTABLE" >&2
    echo "Set LLNZY_APP_PATH=/path/to/LLNZY.app or LLNZY_EXECUTABLE=/path/to/llnzy and try again." >&2
    exit 1
fi

TMP_FILE="$(mktemp "${TMPDIR:-/tmp}/llnzy-cli.XXXXXX")"
cleanup() {
    rm -f "$TMP_FILE"
}
trap cleanup EXIT

EXECUTABLE_QUOTED="$(quote_sh "$EXECUTABLE")"
cat > "$TMP_FILE" <<EOF
#!/bin/sh
# Installed by LLNZY. Do not edit by hand.
EXECUTABLE=\${LLNZY_EXECUTABLE:-$EXECUTABLE_QUOTED}
if [ ! -x "\$EXECUTABLE" ]; then
    echo "llnzy: executable not found at \$EXECUTABLE" >&2
    echo "Reinstall the LLNZY CLI or set LLNZY_EXECUTABLE=/path/to/llnzy." >&2
    exit 127
fi
exec "\$EXECUTABLE" "\$@"
EOF
chmod 0755 "$TMP_FILE"

if [ -e "$LINK_PATH" ] && [ "$FORCE" != "1" ]; then
    if ! grep -q "Installed by LLNZY" "$LINK_PATH" 2>/dev/null; then
        echo "llnzy CLI install refused: $LINK_PATH already exists and was not installed by LLNZY." >&2
        echo "Set LLNZY_CLI_FORCE=1 to replace it intentionally." >&2
        exit 1
    fi
fi

if [ -d "$INSTALL_DIR" ] && [ -w "$INSTALL_DIR" ]; then
    mv "$TMP_FILE" "$LINK_PATH"
else
    sudo mkdir -p "$INSTALL_DIR"
    sudo mv "$TMP_FILE" "$LINK_PATH"
    sudo chmod 0755 "$LINK_PATH"
fi

trap - EXIT
echo "Installed llnzy CLI at $LINK_PATH"
echo "Try: llnzy stacker list"

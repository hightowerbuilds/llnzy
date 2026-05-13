#!/bin/bash
# Build the GPUI LLNZY app and package it as a macOS .app bundle.
# Usage: ./bundle.sh [--release] [--pkg] [--dmg]

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./bundle.sh [--release] [--pkg] [--dmg]

Options:
  --release   Build target/release/llnzy before bundling.
  --pkg       Build a macOS installer package that installs LLNZY.app and /usr/local/bin/llnzy.
  --dmg       Build a compressed DMG. With --pkg, the DMG contains the installer package.
              Without --pkg, the DMG contains the app plus CLI install helper commands.
EOF
}

PACKAGING_ENV="assets/packaging.env"
if [ -f "$PACKAGING_ENV" ]; then
    # shellcheck disable=SC1090
    . "$PACKAGING_ENV"
fi

APP_ID="${APP_ID:-com.hightowerbuilds.llnzy}"
EXECUTABLE_NAME="${EXECUTABLE_NAME:-llnzy}"
DISPLAY_NAME="${DISPLAY_NAME:-LLNZY}"
ICON_RESOURCE="${ICON_RESOURCE:-llnzy.icns}"
MACOS_MIN_VERSION="${MACOS_MIN_VERSION:-13.0}"
VERSION="$(awk -F '"' '/^version =/ { print $2; exit }' Cargo.toml)"

PROFILE="debug"
BUILD_PKG=0
BUILD_DMG=0

for arg in "$@"; do
    case "$arg" in
        --release)
            PROFILE="release"
            ;;
        --pkg)
            BUILD_PKG=1
            ;;
        --dmg)
            BUILD_DMG=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $arg" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [ "$PROFILE" = "release" ]; then
    cargo build --release --bin "$EXECUTABLE_NAME"
else
    cargo build --bin "$EXECUTABLE_NAME"
fi

APP="target/llnzy.app"
CONTENTS="$APP/Contents"
RESOURCES="$CONTENTS/Resources"

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS"
mkdir -p "$RESOURCES"

cp "target/$PROFILE/$EXECUTABLE_NAME" "$CONTENTS/MacOS/$EXECUTABLE_NAME"
cp assets/Info.plist "$CONTENTS/Info.plist"
cp "assets/$ICON_RESOURCE" "$RESOURCES/$ICON_RESOURCE"
cp assets/install-cli.sh "$RESOURCES/install-cli.sh"
cp assets/uninstall-cli.sh "$RESOURCES/uninstall-cli.sh"
chmod 0755 "$RESOURCES/install-cli.sh" "$RESOURCES/uninstall-cli.sh"

/usr/libexec/PlistBuddy -c "Set :CFBundleName $DISPLAY_NAME" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleDisplayName $DISPLAY_NAME" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $APP_ID" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $EXECUTABLE_NAME" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIconFile ${ICON_RESOURCE%.*}" "$CONTENTS/Info.plist"
/usr/libexec/PlistBuddy -c "Set :LSMinimumSystemVersion $MACOS_MIN_VERSION" "$CONTENTS/Info.plist"

echo "Built $APP ($PROFILE)"
echo "Run with: open $APP"

write_cli_launcher() {
    local launcher_path="$1"
    local executable_path="$2"
    mkdir -p "$(dirname "$launcher_path")"
    cat > "$launcher_path" <<EOF
#!/bin/sh
# Installed by LLNZY. Do not edit by hand.
EXECUTABLE=\${LLNZY_EXECUTABLE:-$executable_path}
if [ ! -x "\$EXECUTABLE" ]; then
    echo "llnzy: executable not found at \$EXECUTABLE" >&2
    echo "Reinstall LLNZY or set LLNZY_EXECUTABLE=/path/to/llnzy." >&2
    exit 127
fi
exec "\$EXECUTABLE" "\$@"
EOF
    chmod 0755 "$launcher_path"
}

write_cli_command() {
    local command_path="$1"
    local script_name="$2"
    cat > "$command_path" <<EOF
#!/bin/sh
APP_PATH="/Applications/$DISPLAY_NAME.app"
SCRIPT="\$APP_PATH/Contents/Resources/$script_name"
if [ ! -x "\$SCRIPT" ]; then
    echo "Install $DISPLAY_NAME.app into /Applications first, then run this command again." >&2
    exit 1
fi
exec "\$SCRIPT"
EOF
    chmod 0755 "$command_path"
}

PKG_PATH="target/$DISPLAY_NAME-$VERSION.pkg"
if [ "$BUILD_PKG" -eq 1 ]; then
    PKG_ROOT="target/pkg-root"
    rm -rf "$PKG_ROOT"
    mkdir -p "$PKG_ROOT/Applications"
    mkdir -p "$PKG_ROOT/usr/local/bin"
    ditto "$APP" "$PKG_ROOT/Applications/$DISPLAY_NAME.app"
    write_cli_launcher \
        "$PKG_ROOT/usr/local/bin/llnzy" \
        "/Applications/$DISPLAY_NAME.app/Contents/MacOS/$EXECUTABLE_NAME"
    pkgbuild \
        --root "$PKG_ROOT" \
        --identifier "$APP_ID" \
        --version "$VERSION" \
        --install-location "/" \
        "$PKG_PATH"
    echo "Built $PKG_PATH"
fi

if [ "$BUILD_DMG" -eq 1 ]; then
    DMG_ROOT="target/dmg-root"
    DMG_PATH="target/$DISPLAY_NAME-$VERSION.dmg"
    rm -rf "$DMG_ROOT"
    mkdir -p "$DMG_ROOT"

    if [ "$BUILD_PKG" -eq 1 ]; then
        cp "$PKG_PATH" "$DMG_ROOT/"
    else
        ditto "$APP" "$DMG_ROOT/$DISPLAY_NAME.app"
        ln -s /Applications "$DMG_ROOT/Applications"
        write_cli_command "$DMG_ROOT/Install LLNZY CLI.command" "install-cli.sh"
        write_cli_command "$DMG_ROOT/Uninstall LLNZY CLI.command" "uninstall-cli.sh"
    fi

    hdiutil create \
        -volname "$DISPLAY_NAME $VERSION" \
        -srcfolder "$DMG_ROOT" \
        -ov \
        -format UDZO \
        "$DMG_PATH"
    echo "Built $DMG_PATH"
fi

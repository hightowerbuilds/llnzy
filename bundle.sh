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

Environment:
  LLNZY_KEEP_RELEASE_ARTIFACTS=1
              Keep old target/LLNZY-*.dmg and target/LLNZY-*.pkg files instead
              of clearing them before a fresh release bundle/package build.
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
CODESIGN_IDENTITY="${CODESIGN_IDENTITY:--}"
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

prepare_release_artifacts() {
    mkdir -p target
    if [ "${LLNZY_KEEP_RELEASE_ARTIFACTS:-0}" = "1" ]; then
        return
    fi

    find target -maxdepth 1 \
        \( -name "$DISPLAY_NAME-*.dmg" -o -name "$DISPLAY_NAME-*.pkg" \) \
        -print -delete

    if [ -f "$DISPLAY_NAME.dmg" ]; then
        echo "Warning: $DISPLAY_NAME.dmg exists at the repo root and may be stale." >&2
    fi
}

prepare_release_artifacts

if [ "$PROFILE" = "release" ]; then
    cargo build --release --bin "$EXECUTABLE_NAME"
else
    cargo build --bin "$EXECUTABLE_NAME"
fi

APP="target/llnzy.app"
APP_STAGING="target/llnzy.app.staging"
APP_PREVIOUS="target/llnzy.app.previous"
APP_SWAP_IN_PROGRESS=0
CONTENTS="$APP_STAGING/Contents"
RESOURCES="$CONTENTS/Resources"

cleanup_bundle_build() {
    rm -rf "$APP_STAGING"
    if [ "$APP_SWAP_IN_PROGRESS" -eq 1 ] && [ -d "$APP_PREVIOUS" ] && [ ! -d "$APP" ]; then
        mv "$APP_PREVIOUS" "$APP"
    fi
}
trap cleanup_bundle_build EXIT

rm -rf "$APP_STAGING" "$APP_PREVIOUS"
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

if [ -n "$CODESIGN_IDENTITY" ]; then
    codesign --force --deep --sign "$CODESIGN_IDENTITY" "$APP_STAGING"
fi

if [ ! -x "$CONTENTS/MacOS/$EXECUTABLE_NAME" ]; then
    echo "Bundle executable missing: $CONTENTS/MacOS/$EXECUTABLE_NAME" >&2
    exit 1
fi

APP_SWAP_IN_PROGRESS=1
if [ -d "$APP" ]; then
    mv "$APP" "$APP_PREVIOUS"
fi
mv "$APP_STAGING" "$APP"
APP_SWAP_IN_PROGRESS=0
rm -rf "$APP_PREVIOUS"
trap - EXIT

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
    # Remove the staging tree once the .pkg is produced so it doesn't sit
    # in target/ as a duplicate of /Applications/LLNZY.app — macOS's
    # Storage view flags those copies as "Duplicates" otherwise.
    rm -rf "$PKG_ROOT"
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
        write_cli_command "$DMG_ROOT/Install $DISPLAY_NAME CLI.command" "install-cli.sh"
        write_cli_command "$DMG_ROOT/Uninstall $DISPLAY_NAME CLI.command" "uninstall-cli.sh"
    fi

    hdiutil create \
        -volname "$DISPLAY_NAME $VERSION" \
        -srcfolder "$DMG_ROOT" \
        -ov \
        -format UDZO \
        "$DMG_PATH"
    # Same cleanup as the .pkg path: drop the staging tree once the
    # .dmg is produced so target/dmg-root/LLNZY.app doesn't sit on disk
    # as a phantom duplicate of /Applications/LLNZY.app.
    rm -rf "$DMG_ROOT"
    echo "Built $DMG_PATH"
fi

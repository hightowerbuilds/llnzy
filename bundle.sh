#!/bin/bash
# Build llnzy and package it as a macOS .app bundle.
# Usage: ./bundle.sh [--release]

set -e

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
if [ "$1" = "--release" ]; then
    PROFILE="release"
    cargo build --release
else
    cargo build
fi

APP="target/llnzy.app"
CONTENTS="$APP/Contents"

rm -rf "$APP"
mkdir -p "$CONTENTS/MacOS"
mkdir -p "$CONTENTS/Resources"

cp "target/$PROFILE/$EXECUTABLE_NAME" "$CONTENTS/MacOS/$EXECUTABLE_NAME"
cp assets/Info.plist "$CONTENTS/Info.plist"
cp "assets/$ICON_RESOURCE" "$CONTENTS/Resources/$ICON_RESOURCE"

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

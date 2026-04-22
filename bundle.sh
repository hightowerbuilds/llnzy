#!/bin/bash
# Build llnzy and package it as a macOS .app bundle.
# Usage: ./bundle.sh [--release]

set -e

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

cp "target/$PROFILE/llnzy" "$CONTENTS/MacOS/llnzy"
cp assets/Info.plist "$CONTENTS/Info.plist"
cp assets/llnzy.icns "$CONTENTS/Resources/llnzy.icns"

echo "Built $APP ($PROFILE)"
echo "Run with: open $APP"

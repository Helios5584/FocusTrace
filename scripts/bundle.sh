#!/usr/bin/env bash
# Build release binary and assemble FocusTrace.app for Apple Silicon.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET="aarch64-apple-darwin"
APP="$ROOT/dist/FocusTrace.app"
BIN_NAME="focustrace"

echo "==> cargo build --release --target $TARGET"
cargo build --release --target "$TARGET"

echo "==> assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "macos/Info.plist" "$APP/Contents/Info.plist"
cp "target/$TARGET/release/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"

# Ad-hoc sign so Gatekeeper does not nuke it on first launch.
codesign --force --deep --sign - "$APP" || true

echo "==> done: $APP"

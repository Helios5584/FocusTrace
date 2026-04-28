#!/usr/bin/env bash
# Build release binary and assemble FocusTrace.app for Apple Silicon.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET="aarch64-apple-darwin"
APP="$ROOT/dist/FocusTrace.app"
BIN_NAME="focustrace"

echo "==> generating AppIcon.icns"
python3 "$ROOT/scripts/make_icon.py"
iconutil -c icns "$ROOT/macos/AppIcon.iconset" -o "$ROOT/macos/AppIcon.icns"

echo "==> cargo build --release --target $TARGET"
cargo build --release --target "$TARGET"

echo "==> assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp "macos/Info.plist" "$APP/Contents/Info.plist"
cp "macos/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"
cp "target/$TARGET/release/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"

# Ad-hoc sign with stable identifier so TCC has a chance of recognizing
# the app across rebuilds. Real persistence requires a Developer ID.
codesign --force --deep --sign - --identifier com.focustrace.app "$APP" || true

echo "==> done: $APP"

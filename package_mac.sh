#!/bin/bash
set -e

echo "Building Golden Cookie Clicker in release mode..."
cargo build --release

echo "Creating GoldenCookie.app bundle structure..."
APP_DIR="GoldenCookie.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"

mkdir -p "$MACOS_DIR"

echo "Copying binary..."
cp target/release/golden_cookie "$MACOS_DIR/GoldenCookie"

echo "Creating Info.plist..."
cat <<EOF > "$CONTENTS_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>GoldenCookie</string>
    <key>CFBundleIdentifier</key>
    <string>com.goldencookie</string>
    <key>CFBundleName</key>
    <string>Golden Cookie Clicker</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>LSUIElement</key>
    <string>true</string>
    <key>NSScreenCaptureUsageDescription</key>
    <string>Required to capture the screen and search for golden cookies.</string>
</dict>
</plist>
EOF

echo "App bundle packaged successfully as '$APP_DIR'!"
echo "To run it, double-click it in Finder or run: open $APP_DIR"
echo "To stop it, move your mouse cursor to any corner of the screen (fail-safe), or run: pkill GoldenCookie"

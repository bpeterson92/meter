#!/bin/bash
# Bundle meter-menubar as a macOS .app with LSUIElement (hides from Dock/Cmd+Tab)

set -e

APP_NAME="Meter"
BUNDLE_ID="com.meter.menubar"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/target/release"
APP_DIR="$BUILD_DIR/$APP_NAME.app"

echo "Building release binary..."
cd "$PROJECT_DIR"
cargo build --release --bin meter-menubar

echo "Creating app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp "$BUILD_DIR/meter-menubar" "$APP_DIR/Contents/MacOS/$APP_NAME"

# Create Info.plist with LSUIElement to hide from Dock
cat > "$APP_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundleDisplayName</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>$BUNDLE_ID</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSUIElement</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Create PkgInfo
echo -n "APPL????" > "$APP_DIR/Contents/PkgInfo"

echo ""
echo "App bundle created: $APP_DIR"
echo ""
echo "To run:"
echo "  open '$APP_DIR'"
echo ""
echo "To install to Applications:"
echo "  cp -r '$APP_DIR' /Applications/"
echo ""
echo "To add to Login Items (start on boot):"
echo "  1. Open System Settings > General > Login Items"
echo "  2. Click '+' and add $APP_NAME from Applications"

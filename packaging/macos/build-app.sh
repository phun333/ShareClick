#!/usr/bin/env bash
# Build ShareClick.app (a menu-bar app) and a distributable ShareClick.dmg.
#
# Usage:
#   packaging/macos/build-app.sh [version]
#
# Produces:
#   dist/ShareClick.app
#   dist/ShareClick-<version>.dmg
#
# By default it builds a universal (arm64 + x86_64) binary so a single .dmg runs
# on both Apple Silicon and Intel Macs. If a target toolchain is missing it
# falls back to a host-only build.

set -euo pipefail

cd "$(dirname "$0")/../.."
VERSION="${1:-$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')}"
APP="dist/ShareClick.app"
BIN_NAME="shareclick"

echo "==> Building ShareClick $VERSION (features: tray)"

build_universal() {
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null 2>&1 || true
  if cargo build --release --features tray,gui --target aarch64-apple-darwin \
     && cargo build --release --features tray,gui --target x86_64-apple-darwin; then
    mkdir -p dist
    lipo -create -output "dist/$BIN_NAME" \
      "target/aarch64-apple-darwin/release/$BIN_NAME" \
      "target/x86_64-apple-darwin/release/$BIN_NAME"
    echo "   universal binary built"
    return 0
  fi
  return 1
}

if ! build_universal; then
  echo "   universal build unavailable; building for host arch only"
  cargo build --release --features tray,gui
  mkdir -p dist
  cp "target/release/$BIN_NAME" "dist/$BIN_NAME"
fi

echo "==> Assembling $APP"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "dist/$BIN_NAME" "$APP/Contents/MacOS/$BIN_NAME"
chmod +x "$APP/Contents/MacOS/$BIN_NAME"

# Optional icon (packaging/macos/AppIcon.icns) if present.
ICON_KEY=""
if [[ -f packaging/macos/AppIcon.icns ]]; then
  cp packaging/macos/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>AppIcon</string>"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>ShareClick</string>
  <key>CFBundleDisplayName</key><string>ShareClick</string>
  <key>CFBundleIdentifier</key><string>com.shareclick.ShareClick</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleExecutable</key><string>$BIN_NAME</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <!-- Menu-bar app: no Dock icon, no main window. -->
  <key>LSUIElement</key><true/>
  $ICON_KEY
</dict>
</plist>
PLIST

echo "==> Ad-hoc signing (lets it launch locally without a paid Developer ID)"
codesign --force --deep --sign - "$APP" 2>/dev/null || \
  echo "   codesign unavailable; users may need right-click > Open"

echo "==> Creating DMG"
DMG="dist/ShareClick-$VERSION.dmg"
rm -f "$DMG"
STAGING="$(mktemp -d)"
cp -R "$APP" "$STAGING/"
ln -s /Applications "$STAGING/Applications"
hdiutil create -volname "ShareClick" -srcfolder "$STAGING" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$STAGING"

echo ""
echo "Done:"
echo "  $APP"
echo "  $DMG"
echo ""
echo "Users: drag ShareClick to Applications, then grant Accessibility +"
echo "Input Monitoring permission on first launch."

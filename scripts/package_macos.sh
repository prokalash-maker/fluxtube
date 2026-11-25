#!/usr/bin/env bash
set -euo pipefail

export MACOSX_DEPLOYMENT_TARGET="11.0"
APP_NAME="FluxTube"
VERSION="${1:-0.1.0}"
BUILD_DIR="dist/macos"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"

echo "Building Universal Binary (x86_64 + aarch64) targeting macOS 11.0+..."
cargo build --release --target x86_64-apple-darwin -p fluxtube-desktop
cargo build --release --target aarch64-apple-darwin -p fluxtube-desktop

rm -rf "${BUILD_DIR}"
mkdir -p "${APP_BUNDLE}/Contents/MacOS" "${APP_BUNDLE}/Contents/Resources"

lipo -create \
  "target/x86_64-apple-darwin/release/fluxtube" \
  "target/aarch64-apple-darwin/release/fluxtube" \
  -output "${APP_BUNDLE}/Contents/MacOS/fluxtube"

cat > "${APP_BUNDLE}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>fluxtube</string>
  <key>CFBundleIdentifier</key><string>dev.fluxtube.desktop</string>
  <key>CFBundleName</key><string>${APP_NAME}</string>
  <key>CFBundleVersion</key><string>${VERSION}</string>
  <key>CFBundleShortVersionString</key><string>${VERSION}</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

echo "Creating DMG archive..."
hdiutil create -volname "${APP_NAME}" -srcfolder "${APP_BUNDLE}" -ov -format UDZO "${BUILD_DIR}/${APP_NAME}-${VERSION}-universal.dmg"
echo "Done: ${BUILD_DIR}/${APP_NAME}-${VERSION}-universal.dmg"

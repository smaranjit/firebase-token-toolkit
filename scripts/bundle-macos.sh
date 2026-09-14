#!/usr/bin/env bash
# Assemble Firebase Token Toolkit.app around an already-built binary.
#
# Split out from build-macos.sh so the release workflow and a local build produce
# an identical bundle. Hand-rolled rather than using cargo-bundle so the layout is
# explicit and doesn't drift with a third-party tool.
#
# Usage: bundle-macos.sh <path-to-binary> <output-dir>
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/common.sh
source scripts/common.sh

BINARY="${1:?usage: bundle-macos.sh <binary> <output-dir>}"
OUTDIR="${2:?usage: bundle-macos.sh <binary> <output-dir>}"
VERSION="$(read_version)"
APP="${OUTDIR}/${DISPLAY_NAME}.app"

rm -rf "$APP"
mkdir -p "${APP}/Contents/MacOS" "${APP}/Contents/Resources"

install -m 755 "$BINARY" "${APP}/Contents/MacOS/${NAME}"
cp assets/icon.icns "${APP}/Contents/Resources/icon.icns"

cat > "${APP}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>${DISPLAY_NAME}</string>
	<key>CFBundleDisplayName</key>
	<string>${DISPLAY_NAME}</string>
	<key>CFBundleIdentifier</key>
	<string>${BUNDLE_ID}</string>
	<key>CFBundleExecutable</key>
	<string>${NAME}</string>
	<key>CFBundleIconFile</key>
	<string>icon</string>
	<key>CFBundleVersion</key>
	<string>${VERSION}</string>
	<key>CFBundleShortVersionString</key>
	<string>${VERSION}</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>NSHumanReadableCopyright</key>
	<string>Copyright (c) 2026 Smaranjit Maiti. MIT licensed.</string>
</dict>
</plist>
PLIST

# Ad-hoc signature. Not optional: lipo invalidates any signature the linker
# produced, and arm64 macOS refuses to launch a binary with no signature at all.
# This is *not* notarization — Gatekeeper still warns on first launch.
if command -v codesign >/dev/null; then
  codesign --force --deep --sign - "$APP"
  # Deliberately NOT `codesign --verify ... && echo`: under `set -e` a failing
  # left operand of && does not abort, so a broken signature would sail through
  # into the published .dmg.
  codesign --verify --deep --strict "$APP"
  echo "==> ad-hoc signature verified"
else
  echo "WARNING: codesign unavailable; bundle is unsigned and will not launch on Apple Silicon" >&2
fi

echo "==> bundled ${APP}"

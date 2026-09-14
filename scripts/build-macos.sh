#!/usr/bin/env bash
# Build the macOS release artifact: dist/<name>-v<version>-universal-macos.dmg
#
# Produces a universal binary (Apple Silicon + Intel) and wraps it in a .dmg with
# an /Applications symlink. Mirrors the build-macos job in release.yml.
#
# The result is ad-hoc signed but NOT notarized, so Gatekeeper will block it on
# first launch. README.md documents the workaround for users.
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/common.sh
source scripts/common.sh

if [ "$(uname -s)" != "Darwin" ]; then
  echo "error: this script must run on macOS (needs lipo, codesign, and hdiutil)" >&2
  echo "  cross-compiling to macOS from another OS is not supported here;" >&2
  echo "  use the GitHub Actions release workflow instead." >&2
  exit 1
fi

require cargo
require lipo
require hdiutil

VERSION="$(read_version)"
STAGE="dist/stage-macos"
OUT="dist/${NAME}-v${VERSION}-universal-macos.dmg"

for target in aarch64-apple-darwin x86_64-apple-darwin; do
  echo "==> building ${NAME} v${VERSION} for ${target}"
  rustup target add "$target" >/dev/null 2>&1 || true
  cargo build --release --target "$target"
done

echo "==> merging universal binary"
rm -rf "$STAGE"
mkdir -p "$STAGE"
lipo -create -output "${STAGE}/${NAME}" \
  "target/aarch64-apple-darwin/release/${NAME}" \
  "target/x86_64-apple-darwin/release/${NAME}"
lipo -info "${STAGE}/${NAME}"

echo "==> bundling"
./scripts/bundle-macos.sh "${STAGE}/${NAME}" "$STAGE"
rm -f "${STAGE}/${NAME}"
cp "${DOCS[@]}" "$STAGE/"
ln -s /Applications "${STAGE}/Applications"

echo "==> packing ${OUT}"
rm -f "$OUT"
hdiutil create -volname "$DISPLAY_NAME" -srcfolder "$STAGE" \
  -ov -format UDZO "$OUT"
rm -rf "$STAGE"

# Verify the artifact we are about to ship, rather than trusting that the steps
# above did what they claimed. An unverifiable release artifact is how a bundle
# with a broken signature reaches users.
echo "==> verifying ${OUT}"
MOUNT="$(mktemp -d)"
hdiutil attach "$OUT" -nobrowse -readonly -mountpoint "$MOUNT" >/dev/null
APP="${MOUNT}/${DISPLAY_NAME}.app"
status=0

if [ ! -d "$APP" ]; then
  echo "error: ${DISPLAY_NAME}.app is missing from the dmg" >&2
  status=1
else
  ARCHS="$(lipo -archs "${APP}/Contents/MacOS/${NAME}")"
  case "$ARCHS" in
    *x86_64*arm64* | *arm64*x86_64*) echo "    architectures: ${ARCHS}" ;;
    *)
      echo "error: expected a universal binary, got: ${ARCHS}" >&2
      status=1
      ;;
  esac
  if codesign --verify --deep --strict "$APP" 2>/dev/null; then
    echo "    signature valid inside the mounted image"
  else
    echo "error: signature is invalid inside the dmg" >&2
    status=1
  fi
fi

hdiutil detach "$MOUNT" >/dev/null
rmdir "$MOUNT" 2>/dev/null || true
[ "$status" -eq 0 ] || exit 1

echo "==> done"
ls -lh "$OUT"

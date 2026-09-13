#!/usr/bin/env bash
# Cross-build the Windows release artifact from Linux:
#   dist/<name>-v<version>-x86_64-windows.zip
#
# Convenience only. Official releases are built natively on a Windows runner by
# .github/workflows/release.yml, because embedding the .exe icon needs a resource
# compiler that isn't always available when cross-compiling (see the warning
# check at the end of this script).
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/common.sh
source scripts/common.sh

require cargo
require zip

if ! cargo xwin --version >/dev/null 2>&1; then
  echo "error: cargo-xwin not installed" >&2
  echo "  install it with: cargo install cargo-xwin" >&2
  echo "  (it also needs a clang/lld toolchain on PATH)" >&2
  exit 1
fi

VERSION="$(read_version)"
TARGET="x86_64-pc-windows-msvc"
STAGE="dist/stage-windows/${NAME}-v${VERSION}-x86_64-windows"
OUT="dist/${NAME}-v${VERSION}-x86_64-windows.zip"
LOG="dist/.xwin-build.log"

echo "==> cross-building ${NAME} v${VERSION} for ${TARGET}"
mkdir -p dist
cargo xwin build --release --target "$TARGET" 2>&1 | tee "$LOG"

echo "==> staging"
rm -rf "dist/stage-windows"
mkdir -p "$STAGE"
cp "target/${TARGET}/release/${NAME}.exe" "$STAGE/"
cp "${DOCS[@]}" "$STAGE/"

echo "==> packing ${OUT}"
rm -f "$OUT"
(cd "dist/stage-windows" && zip -qr "../../${OUT}" "$(basename "$STAGE")")
rm -rf "dist/stage-windows"

# Surface the iconless-binary case loudly rather than letting it ship unnoticed.
if grep -q "could not embed Windows resources" "$LOG"; then
  echo
  echo "WARNING: no resource compiler was available, so this .exe has no icon." >&2
  echo "         Use the CI-built artifact for an actual release." >&2
fi
rm -f "$LOG"

echo "==> done"
ls -lh "$OUT"

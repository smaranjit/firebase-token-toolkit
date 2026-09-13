#!/usr/bin/env bash
# Build the Linux release artifact: dist/<name>-v<version>-x86_64-linux.tar.gz
#
# Mirrors the build-linux job in .github/workflows/release.yml. Official releases
# are built on ubuntu-22.04 so the binary works against glibc 2.35 and newer;
# building here on a newer distro raises that floor for the artifact you produce.
set -euo pipefail

cd "$(dirname "$0")/.."
# shellcheck source=scripts/common.sh
source scripts/common.sh

require cargo
require tar

VERSION="$(read_version)"
TARGET="x86_64-unknown-linux-gnu"
STAGE="dist/stage-linux/${NAME}-v${VERSION}-x86_64-linux"
OUT="dist/${NAME}-v${VERSION}-x86_64-linux.tar.gz"

echo "==> building ${NAME} v${VERSION} for ${TARGET}"
cargo build --release --target "$TARGET"

echo "==> staging"
rm -rf "dist/stage-linux"
mkdir -p "$STAGE"
cp "target/${TARGET}/release/${NAME}" "$STAGE/"
cp "${DOCS[@]}" "$STAGE/"

echo "==> packing ${OUT}"
rm -f "$OUT"
tar -czf "$OUT" -C "dist/stage-linux" "$(basename "$STAGE")"
rm -rf "dist/stage-linux"

echo "==> done"
ls -lh "$OUT"

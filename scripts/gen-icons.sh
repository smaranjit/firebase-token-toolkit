#!/usr/bin/env bash
# Regenerate the committed icon set from assets/icon.svg.
#
# Dev-only: the outputs are committed so neither CI nor a plain `cargo build`
# needs rsvg or Python. Re-run this only when icon.svg changes.
set -euo pipefail

cd "$(dirname "$0")/.."

for tool in rsvg-convert python3; do
  command -v "$tool" >/dev/null || { echo "error: $tool not found" >&2; exit 1; }
done

# 48 is Windows-only; 1024 is macOS-only. The rest are shared.
SIZES=(16 32 48 64 128 256 512 1024)

echo "==> rendering PNGs from assets/icon.svg"
args=()
for size in "${SIZES[@]}"; do
  rsvg-convert -w "$size" -h "$size" assets/icon.svg -o "assets/icon-${size}.png"
  args+=("${size}:assets/icon-${size}.png")
done

echo "==> packing containers"
python3 scripts/pack-icons.py ico  assets/icon.ico  "${args[@]}"
python3 scripts/pack-icons.py icns assets/icon.icns "${args[@]}"

echo "==> done"

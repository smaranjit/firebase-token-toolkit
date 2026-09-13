#!/usr/bin/env python3
"""Pack PNG files into Windows .ico and macOS .icns containers.

Exists because the usual tools aren't reliably available on Linux: ImageMagick
ships no ICNS delegate in most builds, `iconutil` is macOS-only, and libicns is
rarely packaged. ImageMagick *can* write .ico but stores every frame as raw BMP,
which turns a six-size icon into ~370KB of dead weight inside the .exe.

Both formats are simple containers, and both accept PNG payloads verbatim:
  - .ico  since Windows Vista (Rust's windows-msvc target requires Win10+)
  - .icns since OS X 10.7

Usage: pack-icons.py ico|icns <out-file> <size>:<file.png> [...]
"""
import struct
import sys

# icns chunk type -> pixel dimension. The retina types (ic11..ic14) deliberately
# reuse the same PNG as the 1x type of equal pixel size; macOS picks by pixels.
ICNS_TYPES = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic07", 128),
    (b"ic13", 256),
    (b"ic08", 256),
    (b"ic14", 512),
    (b"ic09", 512),
    (b"ic10", 1024),
]

# Windows shell never asks for more than 256; larger frames are pure bloat.
ICO_MAX = 256


def build_icns(pngs):
    chunks = []
    for chunk_type, size in ICNS_TYPES:
        data = pngs.get(size)
        if data is not None:
            chunks.append(chunk_type + struct.pack(">I", len(data) + 8) + data)
    if not chunks:
        raise SystemExit("error: no PNGs matched any icns chunk size")
    body = b"".join(chunks)
    return b"icns" + struct.pack(">I", len(body) + 8) + body


def build_ico(pngs):
    sizes = sorted(s for s in pngs if s <= ICO_MAX)
    if not sizes:
        raise SystemExit(f"error: no PNGs at or below {ICO_MAX}px for the .ico")

    # ICONDIR, then one 16-byte ICONDIRENTRY per image, then the payloads.
    header = struct.pack("<HHH", 0, 1, len(sizes))
    offset = len(header) + 16 * len(sizes)
    entries, payloads = [], []
    for size in sizes:
        data = pngs[size]
        # 256 is encoded as 0 in the single-byte width/height fields.
        dim = 0 if size == 256 else size
        entries.append(
            struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset)
        )
        payloads.append(data)
        offset += len(data)
    return header + b"".join(entries) + b"".join(payloads)


def main() -> int:
    if len(sys.argv) < 4 or sys.argv[1] not in ("ico", "icns"):
        print(__doc__.strip(), file=sys.stderr)
        return 2

    kind, out_path = sys.argv[1], sys.argv[2]
    pngs = {}
    for arg in sys.argv[3:]:
        size, _, path = arg.partition(":")
        if not path:
            print(f"error: expected <size>:<path>, got {arg!r}", file=sys.stderr)
            return 2
        with open(path, "rb") as fh:
            pngs[int(size)] = fh.read()

    blob = build_ico(pngs) if kind == "ico" else build_icns(pngs)
    with open(out_path, "wb") as fh:
        fh.write(blob)
    print(f"{out_path}: {len(blob)} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())

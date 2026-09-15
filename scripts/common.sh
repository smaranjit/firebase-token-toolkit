# Shared helpers for the build scripts. Source this; don't execute it.
# shellcheck shell=bash

NAME="firebase-token-toolkit"
DISPLAY_NAME="Firebase Token Toolkit"
BUNDLE_ID="dev.smaranjit.firebase-token-toolkit"

# Read version straight from Cargo.toml so artifact names can never drift from
# what was actually built. Stops at the first match, which is [package].version.
read_version() {
  sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' Cargo.toml | head -1
}

require() {
  command -v "$1" >/dev/null || {
    echo "error: $1 not found${2:+ — $2}" >&2
    exit 1
  }
}

# Files shipped alongside the binary in every archive.
#
# README.txt rather than README.md: on Windows a .md opens in Notepad as raw
# markup, and the repository README is 90 lines aimed at people browsing the
# project rather than someone who just downloaded a binary. The plain-text
# version carries only what a downloader needs — most importantly the macOS
# Gatekeeper and Windows SmartScreen steps, which are needed exactly when the
# app will not launch and so cannot live behind a link inside it.
#
# LICENSE ships because MIT requires the notice to be included in all copies.
# The changelog and security notes are reachable from the About dialog instead,
# where a link is always current and a shipped copy would be frozen at build
# time.
DOCS=(README.txt LICENSE)

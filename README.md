# Firebase Token Toolkit

A native desktop GUI for the Firebase auth tokens you need during development and testing — mint custom tokens from a UID, exchange them for ID tokens, edit a user's custom claims, and pull App Check debug tokens. One window, no Node, no webview, no Java runtime.

[![CI](https://github.com/smaranjit/firebase-token-toolkit/actions/workflows/ci.yml/badge.svg)](https://github.com/smaranjit/firebase-token-toolkit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

Built with [eframe / egui](https://github.com/emilk/egui) — pure Rust, single binary.

## Install

Grab the latest build for your platform from the [releases page](https://github.com/smaranjit/firebase-token-toolkit/releases/latest).

**Linux** (x86_64, glibc 2.35+ — Ubuntu 22.04 and newer):

```bash
tar xzf firebase-token-toolkit-*-x86_64-linux.tar.gz
./firebase-token-toolkit
```

Two Linux notes:

- The service-account file picker goes through an [XDG desktop portal](https://wiki.archlinux.org/title/XDG_Desktop_Portal), so you need an implementation installed. Most desktop environments ship one; on a bare window manager, install `xdg-desktop-portal-gtk` or similar.
- The **Copy** buttons use the X11 clipboard (via `arboard`, which has no native Wayland backend). Under Wayland you need XWayland running, which is the default almost everywhere. If a copy fails, the button reports the error inline.

**Windows** (x86_64): unzip and run `firebase-token-toolkit.exe`. SmartScreen may warn on first launch because the binary is unsigned — choose *More info* → *Run anyway*.

**macOS** (universal, Apple Silicon + Intel, macOS 11+): open the `.dmg` and drag the app to Applications.

The macOS build is **ad-hoc signed but not notarized** (this project has no Apple Developer account), so Gatekeeper will refuse it on first launch. Either right-click the app → *Open* → *Open*, or clear the quarantine flag:

```bash
xattr -dr com.apple.quarantine "/Applications/Firebase Token Toolkit.app"
```

## Features

- **Multiple profiles** — switch between Firebase projects (dev/staging/prod, multiple clients, …) from a dropdown in the top bar. Each profile remembers its own service-account path, project ID, API key, App ID, and debug token. Switching a profile reloads the service account and clears the user list, OAuth cache, selected UID, and all tab outputs.
- **UID → Custom Token** — sign a Firebase custom token (RS256) using a service account, with optional custom claims.
- **UID → ID Token** — one-shot: sign a custom token, then exchange it via `signInWithCustomToken`.
- **Custom Token → ID Token** — paste a custom token and exchange it for an ID token.
- **User Custom Claims** — set persistent `customAttributes` on a user record via `accounts:update`. Loads the user's existing claims, edits them as JSON, validates the 1000-byte limit, and supports a "clear all" action. Future ID tokens automatically include the saved claims.
- **App Check Debug Token** — exchange a registered debug token via `firebaseappcheck.googleapis.com`.
- **Dynamic UID picker** — a left-side panel lists every user via `accounts:batchGet` (paginated up to 5,000) and lets you search by email, phone, or UID. Direct lookups fall through to `accounts:lookup` for users not present in the loaded page.
- Decoded JWT claims table for every token, with timestamp → ISO conversion and clipboard copy.
- Persists project ID, service-account path, and last-used tab between launches. API keys and App IDs are persisted **only if you opt in** — see [Security](#security).

## Configure

In the top bar:

1. **Service account** → *Browse* and pick the JSON from Firebase Console → Project Settings → Service accounts → *Generate new private key*. The project ID auto-populates from the file.
2. **API key** → Firebase Console → Project Settings → General → Web API Key. Only needed for the ID-token and App Check tabs.
3. **App ID** → only needed for the App Check tab.

The status indicator on the right turns green once the service account is loaded and an API key is set.

## Build from source

Requires Rust 1.88 or newer.

```bash
git clone https://github.com/smaranjit/firebase-token-toolkit
cd firebase-token-toolkit
cargo build --release
# binary at target/release/firebase-token-toolkit
```

**Linux** also needs the windowing and clipboard development headers:

```bash
sudo apt install -y \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

**Windows** needs the MSVC toolchain (Visual Studio Build Tools). **macOS** needs Xcode command line tools. Neither needs anything else — TLS is `rustls`, so there is no OpenSSL system dependency.

To reproduce the packaged release artifacts, see [`scripts/`](./scripts) and [CONTRIBUTING.md](./CONTRIBUTING.md#building-release-artifacts).

## Security

This tool handles Firebase **service-account private keys** and mints signed tokens with them. Please read [SECURITY.md](./SECURITY.md) before using it against anything that matters — it covers what is written to disk, what stays in memory, and how to report a vulnerability.

The short version: the service account is read from the path you pick and never copied; OAuth access tokens live in memory only; API keys and App IDs are written to disk only when you tick **Remember secrets**.

## Contributing

Issues and pull requests are welcome — see [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

MIT — see [LICENSE](./LICENSE).

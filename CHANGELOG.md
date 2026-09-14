# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0]

First public release.

### Added

- Multiple Firebase project profiles, switchable from the top bar.
- **UID → Custom Token** — RS256-signed custom tokens with optional custom claims.
- **UID → ID Token** — sign and exchange in one step via `signInWithCustomToken`.
- **Custom Token → ID Token** — exchange a pasted custom token.
- **User Custom Claims** — read, edit, and save `customAttributes` via `accounts:update`,
  with 1000-byte limit validation and a clear-all action.
- **App Check Debug Token** — exchange a registered debug token.
- Dynamic UID picker backed by `accounts:batchGet`, with email/phone/UID search falling
  through to `accounts:lookup`.
- Decoded JWT claims table with ISO timestamp conversion and clipboard copy.
- Opt-in persistence of API key and App ID; opt-out is the default.
- Linux, Windows, and macOS release builds.

### Notes for anyone upgrading from a pre-release build

The application was renamed from `firebase-tool` to `firebase-token-toolkit`. Because
the old name was also the key `eframe` used to locate saved settings, your profiles
will appear empty on first launch unless you move the config directory:

```bash
# Linux
mv ~/.local/share/firebase-tool ~/.local/share/firebase-token-toolkit

# macOS
mv ~/"Library/Application Support/firebase-tool" \
   ~/"Library/Application Support/firebase-token-toolkit"
```

```powershell
# Windows
Move-Item "$env:APPDATA\firebase-tool" "$env:APPDATA\firebase-token-toolkit"
```

There is deliberately no automatic migration for this: it is a one-time move affecting
a handful of pre-release users, and a path-sniffing fallback would be permanent
complexity in exchange for saving a single command.

[Unreleased]: https://github.com/smaranjit/firebase-token-toolkit/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/smaranjit/firebase-token-toolkit/releases/tag/v0.1.0

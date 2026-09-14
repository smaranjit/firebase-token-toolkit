# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3]

### Fixed

- **The app now opens on machines with no GPU at all**, such as a QEMU/KVM
  Windows guest using the display-only virtio GPU driver. v0.1.2 fell back from
  OpenGL to wgpu but then failed with "no suitable adapter found", because
  egui-wgpu asks wgpu for an adapter with `force_fallback_adapter: false` and
  there was no hardware one. The app now enumerates adapters itself and accepts
  a software adapter as a last resort — WARP on Windows, lavapipe on Linux.
  Hardware is still strongly preferred.

### Changed

- Upgraded egui, eframe and egui_extras from 0.29 to 0.33, which is what exposes
  the adapter selector. The minimum supported Rust version is unchanged at 1.88;
  0.33 is the newest release that still builds on it.

## [0.1.2]

### Fixed

- **The app now opens in virtual machines and remote desktop sessions.** It was
  built against OpenGL only, and those environments commonly expose just a
  software OpenGL 1.1 driver, below the 2.0 `egui_glow` requires. Startup failed
  with `egui_glow requires opengl 2.0+` and the process exited. wgpu is now
  compiled in as a fallback and is used automatically when OpenGL is unavailable
  — Direct3D 12 on Windows, Vulkan elsewhere.
- **A startup failure is now visible.** Release builds link as a Windows GUI
  subsystem application, so there is no console and anything written to stderr is
  discarded; the app simply vanished. Fatal startup errors now open a message
  dialog explaining what failed.

### Changed

- The binary is roughly 5 MB larger, the cost of compiling in the second
  renderer.

Documented the graphics requirement in the README, which had never stated one,
and the bug report form now asks whether you are on a VM or remote desktop.

## [0.1.1]

### Fixed

- **The Windows build now starts on a clean machine.** The 0.1.0 executable was
  dynamically linked against the MSVC runtime, so it imported `VCRUNTIME140.dll`
  and failed with "The code execution cannot proceed because VCRUNTIME140.dll was
  not found" unless the user happened to have the Visual C++ Redistributable
  installed. The CRT is now linked statically.

  CI did not catch this because GitHub's Windows runners ship with the
  redistributable, so the binary ran fine there and only broke for users. Both
  Windows build scripts now fail if the executable imports `VCRUNTIME` at all.

Linux and macOS artifacts are unaffected; only the Windows download changes.

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

### Fixed

Relative to the pre-release `firebase-tool` builds:

- Deleting the active profile no longer panics. The index was left pointing past
  the end of the profile list for the remainder of the frame.
- The Custom Claims editor is cleared when the selected user changes. Previously
  the editor kept the previously loaded user's claims, and Save would write them
  onto the newly selected user's record.
- Custom claims that are not a JSON object (an array, string or number) are now
  rejected before signing instead of producing a token that fails opaquely at
  `signInWithCustomToken`.
- The service-account file picker runs asynchronously; it previously blocked the
  UI thread for as long as the dialog was open.
- Pagination cursors and project/app IDs are percent-encoded into request URLs.
  A UID containing `&`, `+` or `/` could previously corrupt the request.
- Truncated token previews slice on character boundaries, fixing a panic on
  multi-byte input.
- A failed clipboard copy now stays on screen instead of appearing for a single
  frame.
- Clearing the service account no longer discards a project ID that was typed by
  hand; only an auto-filled value is removed.
- Background tasks are cancelled when superseded or when switching profiles, and
  a task that dies without a result now surfaces an error rather than silently
  stopping the spinner.

### Also in this release

- An **About dialog**, reached from the footer, showing the version, author,
  licence and links to the source, contributors and issues. The footer carries
  the version so it can be quoted in a bug report.
- `jsonwebtoken` upgraded to 11 with an explicit crypto provider selected.
- Dropped the unused `directories` and `thiserror` dependencies.

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

[Unreleased]: https://github.com/smaranjit/firebase-token-toolkit/compare/v0.1.3...HEAD
[0.1.3]: https://github.com/smaranjit/firebase-token-toolkit/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/smaranjit/firebase-token-toolkit/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/smaranjit/firebase-token-toolkit/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/smaranjit/firebase-token-toolkit/releases/tag/v0.1.0

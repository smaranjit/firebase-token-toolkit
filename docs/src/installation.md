# Installation

Download the archive for your platform from the
[latest release](https://github.com/smaranjit/firebase-token-toolkit/releases/latest).
There is no installer and nothing else to install: the app is a single native
binary with no Node, webview, or Java runtime behind it.

| Platform | Archive | Requirements |
|---|---|---|
| Linux | `firebase-token-toolkit-<version>-x86_64-linux.tar.gz` | x86_64, glibc 2.35 or newer (Ubuntu 22.04 and later) |
| Windows | `firebase-token-toolkit-<version>-x86_64-windows.zip` | x86_64 |
| macOS | `firebase-token-toolkit-<version>-universal-macos.dmg` | macOS 11 or newer, Apple Silicon or Intel |

Each release also publishes `SHA256SUMS`. To check a download before running
it on Linux or macOS:

```bash
sha256sum --check --ignore-missing SHA256SUMS   # macOS: shasum -a 256 -c --ignore-missing SHA256SUMS
```

## Linux

```bash
tar xzf firebase-token-toolkit-*-x86_64-linux.tar.gz
cd firebase-token-toolkit-*-x86_64-linux
./firebase-token-toolkit
```

Two parts of the app lean on the desktop environment:

- **The Browse… button** opens the file chooser through an
  [XDG desktop portal](https://wiki.archlinux.org/title/XDG_Desktop_Portal).
  GNOME, KDE and most other desktops ship one. On a bare window manager,
  install `xdg-desktop-portal-gtk` or an equivalent, or the picker will not
  open.
- **The Copy buttons** write to the X11 clipboard. Under Wayland that needs
  XWayland, which nearly every Wayland session runs by default. If a copy
  fails, the error appears next to the button instead of failing silently.

## Windows

Unzip the archive and run `firebase-token-toolkit.exe`.

The binary is not code-signed, so SmartScreen may stop it on first launch.
Choose **More info**, then **Run anyway**. The C runtime is linked statically,
so you do not need the Visual C++ Redistributable.

## macOS

Open the `.dmg` and drag **Firebase Token Toolkit** into Applications.

The app is ad-hoc signed but not notarized, because the project has no Apple
Developer account. Gatekeeper therefore blocks the first launch. Either
right-click the app, choose **Open**, then **Open** again in the dialog, or
clear the quarantine flag from a terminal:

```bash
xattr -dr com.apple.quarantine "/Applications/Firebase Token Toolkit.app"
```

## Updating

Download the new archive and replace the old binary. Your profiles and
settings live outside the app (see [Settings and storage](reference/settings-storage.md)),
so they carry over.

## Building from source

If there is no build for your platform, or you would rather compile it
yourself, see [Building and contributing](contributing.md).

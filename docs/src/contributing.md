# Building and contributing

## Build from source

You need Rust 1.90 or newer.

```bash
git clone https://github.com/smaranjit/firebase-token-toolkit
cd firebase-token-toolkit
cargo build --release
# the binary is target/release/firebase-token-toolkit
```

Linux also needs the windowing and clipboard development headers:

```bash
sudo apt install -y \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

Windows needs the MSVC toolchain from Visual Studio Build Tools, and macOS
needs the Xcode command line tools. TLS goes through `rustls`, so no platform
needs OpenSSL.

## Contributing

Issues and pull requests are welcome.
[`CONTRIBUTING.md`](https://github.com/smaranjit/firebase-token-toolkit/blob/main/CONTRIBUTING.md)
covers the checks CI runs, the project layout, building release archives and
cutting a release.

## Editing this guide

The guide is an [mdBook](https://rust-lang.github.io/mdBook/) in the `docs/`
folder of the repository. To preview changes locally:

```bash
mdbook serve docs --open
```

Every page has an edit icon in the top-right corner that opens it on GitHub.
Changes merged to `main` are published automatically.

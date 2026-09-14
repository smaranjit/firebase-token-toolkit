# Contributing

Thanks for taking a look. Issues and pull requests are both welcome.

## Getting set up

Requires Rust 1.88 or newer (the MSRV is declared in `Cargo.toml` and enforced in CI).

```bash
git clone https://github.com/smaranjit/firebase-token-toolkit
cd firebase-token-toolkit
cargo run
```

On Linux you'll need the windowing and clipboard development headers first:

```bash
sudo apt install -y \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libgl1-mesa-dev \
  libx11-dev libxrandr-dev libxi-dev libxcursor-dev \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev pkg-config
```

macOS needs the Xcode command line tools; Windows needs the MSVC toolchain from
Visual Studio Build Tools. TLS is `rustls`, so no OpenSSL is required anywhere.

To exercise the app you'll want a Firebase project and a service-account JSON key.
Please use a **development** project — see [SECURITY.md](./SECURITY.md) for why.

## Before opening a pull request

CI runs these, and they gate merges:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

`cargo clippy` is clean on `main` with warnings denied, so please keep it that way
rather than adding `#[allow]` attributes — if a lint is genuinely wrong for a case,
say so in the PR and we'll decide together.

## Project layout

```
src/
  main.rs         window setup, tokio runtime, eframe entry point
  app.rs          top-level app state, tab dispatch, persistence hooks
  config.rs       persisted profiles and their migration
  async_task.rs   small helper for polling background work from the UI thread
  firebase/       API layer — no egui in here
    jwt.rs           RS256 signing
    oauth.rs         service-account -> access token exchange, with caching
    service_account.rs
    custom_token.rs, id_token.rs, appcheck.rs, users.rs
  ui/             one module per tab, plus the UID picker and shared widgets
assets/           icon source and generated icon set
scripts/          release build and packaging
```

Keep `firebase/` free of UI code and `ui/` free of network calls — the tabs drive
work through `async_task.rs` rather than blocking the UI thread.

## Building release artifacts

Each script produces the same artifact its CI counterpart does, into `dist/`:

```bash
./scripts/build-linux.sh      # -> dist/*-x86_64-linux.tar.gz
./scripts/build-windows.sh    # -> dist/*-x86_64-windows.zip   (cross-build, needs cargo-xwin)
./scripts/build-macos.sh      # -> dist/*-universal-macos.dmg  (macOS only)
```

`build-windows.ps1` is the native equivalent for people actually on Windows.

A caveat on the Linux -> Windows cross-build: embedding the `.exe` icon needs a
resource compiler, and if `cargo-xwin` can't supply one the build still succeeds but
prints a warning and produces an iconless binary. Official releases come from the
native Windows CI runner, so they always have the icon.

## Changing the icon

`assets/icon.svg` is the source. The PNG/ICO/ICNS set is generated and committed so
that neither CI nor a plain `cargo build` needs image tooling:

```bash
./scripts/gen-icons.sh    # needs rsvg-convert and python3
```

Commit the regenerated files along with the SVG.

## Releasing

1. Bump `version` in `Cargo.toml` and add a `CHANGELOG.md` entry.
2. Commit, then tag: `git tag v0.1.0 && git push origin v0.1.0`.
3. `release.yml` builds all three platforms and publishes the GitHub Release. It
   fails fast if the tag doesn't match the version in `Cargo.toml`.

# Security

This tool loads Firebase **service-account private keys** and uses them to mint signed
tokens. That makes it roughly as sensitive as the key file itself. This document
describes exactly what it does with your credentials so you can decide where it is
appropriate to run.

## Reporting a vulnerability

Please report security issues privately through
[GitHub's private vulnerability reporting](https://github.com/smaranjit/firebase-token-toolkit/security/advisories/new)
rather than opening a public issue. I'll acknowledge the report and, once a fix ships,
credit you in the release notes unless you'd rather stay anonymous.

## What touches disk

Settings are persisted by `eframe` to a single `app.ron` file:

| Platform | Location |
|---|---|
| Linux | `~/.local/share/firebase-token-toolkit/app.ron` |
| macOS | `~/Library/Application Support/firebase-token-toolkit/app.ron` |
| Windows | `%APPDATA%\firebase-token-toolkit\data\app.ron` |

Persisted for every profile:

- **Service-account path** — the filesystem path only. The key file's contents are read
  on demand and never copied into the config.
- Project ID, and the last-used tab.

Persisted **only when "Remember secrets" is enabled**:

- Web API key, App ID, and App Check debug token.

These are stored in **plaintext**. There is no OS keychain integration — the file is
protected by nothing but its filesystem permissions. If you would not paste the value
into a plaintext file, leave "Remember secrets" off. Turning it off clears those fields
for *all* profiles on the next save (`clear_secrets_all` in `src/config.rs`).

**Never persisted:** the service-account private key, OAuth access tokens, and any
custom/ID token shown in the UI.

## What stays in memory

- The **service-account private key**, held for the lifetime of the loaded profile and
  used to sign RS256 assertions (`src/firebase/jwt.rs`).
- **OAuth access tokens**, cached and refreshed automatically within 60 seconds of
  expiry (`src/firebase/oauth.rs`). Never written to disk.

Switching profiles drops the loaded service account, the cached OAuth token, the user
list, and every tab's output.

## Network traffic

All requests go to Google endpoints over TLS (`rustls` — no OpenSSL):

- `oauth2.googleapis.com/token` — exchanges a signed JWT assertion for an access token
- `identitytoolkit.googleapis.com` — `signInWithCustomToken`, `accounts:batchGet`,
  `accounts:lookup`, `accounts:update`
- `firebaseappcheck.googleapis.com` — `exchangeDebugToken`

There is **no telemetry, analytics, or update check**. The application makes no
network request you did not initiate.

The one thing that leaves the app besides the calls above is the **GitHub** link
in the footer, which hands the repository URL to your system browser when you
click it. Nothing is sent with it.

## Things worth knowing

- **The OAuth scopes are broad.** Access tokens are requested with
  `auth/identitytoolkit` *and* `auth/cloud-platform` (`src/firebase/oauth.rs`). The
  latter is a wide scope; the token is short-lived and memory-resident, but a service
  account with broad IAM roles will produce a correspondingly powerful token.
- **Tokens on screen are real credentials.** A minted ID token authenticates as that
  user until it expires. Treat the window like a terminal showing secrets, and be
  careful about screen sharing.
- **The UID picker displays real user PII** — emails, phone numbers, and display names
  are fetched via `accounts:batchGet`. Keep that in mind before screenshotting.
- **Custom claims are written to the user record**, not just to a token. Saving claims
  calls `accounts:update` and affects every future ID token that user receives.
- **RS256 signing uses the RustCrypto backend.** `jsonwebtoken` is built with
  its `rust_crypto` feature, which brings in the `rsa` crate. That crate carries
  an unresolved timing-sidechannel advisory for RSA private-key operations
  ([RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071)).
  Exploiting it requires precisely timing many signing operations, which is a
  concern for a network-exposed signing service rather than a desktop tool you
  run locally — but if you are signing with a key that matters, it is worth
  knowing. The alternative backend, `aws_lc_rs`, is constant-time but needs
  cmake and nasm, which breaks the cross-compiled Windows build.
- **The Windows build links the C runtime statically.** That is what lets the
  executable start on a machine without the Visual C++ Redistributable, but it
  means Microsoft's runtime security fixes reach you only when this project
  rebuilds and publishes a new release, rather than through Windows Update.
- **Prefer a non-production service account.** Nothing here requires production
  credentials, and a development project limits the blast radius of a mistake.

## Supported versions

This is a pre-1.0 project. Security fixes land on the latest release only.

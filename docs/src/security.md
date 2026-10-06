# Security

Firebase Token Toolkit loads a service-account private key and mints real,
signed tokens with it. The app is about as sensitive as the key file itself.
[`SECURITY.md`](https://github.com/smaranjit/firebase-token-toolkit/blob/main/SECURITY.md)
in the repository is the authoritative description; this page is the short
version for day-to-day use.

## What the app keeps

- The **private key** is read from the file you choose and held in memory. It
  is never copied into the settings file.
- **OAuth access tokens** and every **token shown on screen** live in memory
  only.
- The **API key, App ID and App Check debug token** are written to disk, in
  plaintext, only when **remember API key & App ID** is ticked. See
  [Settings and storage](reference/settings-storage.md).

## Habits worth keeping

- **Point the app at a development project.** A service-account key can mint a
  token for any user in its project.
- **Treat the window like a terminal full of secrets.** An ID token on screen
  signs in as that user until it expires, usually an hour later. Be careful
  when screen sharing.
- **The Users panel shows personal data.** Emails, phone numbers and display
  names come straight from Firebase Authentication.
- **Custom claims are permanent until changed.** **Save** on the User Custom
  Claims tab writes to the user record, not to one token, so every future ID
  token for that user carries the claims.

## Reporting a vulnerability

Use [GitHub's private vulnerability reporting](https://github.com/smaranjit/firebase-token-toolkit/security/advisories/new)
rather than opening a public issue.

# Settings and storage

Everything the app remembers lives in one file, `app.ron`:

| Platform | Location |
|---|---|
| Linux | `~/.local/share/firebase-token-toolkit/app.ron` |
| macOS | `~/Library/Application Support/firebase-token-toolkit/app.ron` |
| Windows | `%APPDATA%\firebase-token-toolkit\data\app.ron` |

The file is written when the app closes and periodically while it runs.

## What is saved

| Value | Saved | Notes |
|---|---|---|
| Profile names | Always | |
| Service-account path | Always | The path only. The key file itself is never copied. |
| Project ID | Always | |
| Active profile, last-used tab | Always | |
| Window size and position | Always | |
| Each app's name, type and App ID | Always | |
| Android package and SHA-1, iOS bundle ID | Always | Public identifiers, included in every app build |
| Selected app | Always | |
| Each app's API key | Only with **remember API keys & debug tokens** | Plaintext |
| Each app's App Check debug token | Only with **remember API keys & debug tokens** | Plaintext |
| Service-account private key | Never | Re-read from the saved path at every launch |
| OAuth access tokens | Never | Held in memory, refreshed automatically |
| Minted custom, ID and App Check tokens | Never | |

The **remember API keys & debug tokens** checkbox applies to every profile at
once. When it is off, the app clears every app's API key and debug token, in
all profiles, when it starts and again every time it saves, so unticking it
also removes values that were saved earlier. Clicking **Load apps** fetches the
keys again.

If the service-account file has moved or been deleted, the app starts with no
service account loaded and the status indicator shows `● not configured`. Use
**Browse…** to point the profile at the new location.

## Resetting the app

Close the app and delete `app.ron`. The next launch starts with a single empty
profile named `Default`.

## Upgrading from 0.1.x

Version 0.1.x stored one API key, App ID and debug token per profile. The first
launch of a newer version turns them into the profile's first app, named
after its type (for example *Web app*) and with its type taken from the App ID.
If **remember** was off, 0.1.x saved none of the three, so the profile starts
with one empty web app; **Load apps** fills in the project's apps.

## Upgrading from early versions

Versions before profiles existed stored a single set of fields at the top level
of the file. The first launch of a newer version moves those values into a
profile named `Default`; nothing needs to be done by hand.

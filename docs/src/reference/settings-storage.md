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
| Web API key | Only with **remember API key & App ID** | Plaintext |
| App ID | Only with **remember API key & App ID** | Plaintext |
| App Check debug token | Only with **remember API key & App ID** | Plaintext |
| Service-account private key | Never | Re-read from the saved path at every launch |
| OAuth access tokens | Never | Held in memory, refreshed automatically |
| Minted custom, ID and App Check tokens | Never | |

The **remember API key & App ID** checkbox applies to every profile at once.
When it is off, the app clears the API key, App ID and debug token from all
profiles when it starts and again every time it saves, so unticking it also
removes values that were saved earlier.

If the service-account file has moved or been deleted, the app starts with no
service account loaded and the status indicator shows `● not configured`. Use
**Browse…** to point the profile at the new location.

## Resetting the app

Close the app and delete `app.ron`. The next launch starts with a single empty
profile named `Default`.

## Upgrading from early versions

Versions before profiles existed stored a single set of fields at the top level
of the file. The first launch of a newer version moves those values into a
profile named `Default`; nothing needs to be done by hand.

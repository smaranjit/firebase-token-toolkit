# The top bar

Everything the tabs need comes from the top bar. Its values belong to the
active [profile](profiles.md), so switching profiles swaps them all at once.

![The top bar with a service account loaded and the indicator reading ready](images/top-bar-ready.png)

## Service account

**Browse…** opens a file chooser for the service-account JSON key. When the key
loads, a green *Service account loaded* message appears, and if **Project ID**
is empty it is filled in from the key.

**Clear** unloads the key and clears the selected user. It also clears
**Project ID**, but only if the key filled that value in and you have not
edited it since.

The app stores the file's path, not its contents, and reads the key again on
every launch. If the file has moved, the profile starts with no service
account loaded.

## Status indicator

The indicator to the right of the service account shows what the app can do
right now:

| Indicator | Meaning | Tabs available |
|---|---|---|
| **not configured** (red) | No service account loaded | *Custom -> ID Token* and *App Check* only, given an API key |
| **SA only** (yellow) | Service account loaded, no API key | *UID -> Custom Token*, *User Custom Claims*, the Users panel |
| **ready** (green) | Service account and API key | All tabs (*App Check* also needs an App ID) |

## Project ID

The Firebase project ID, for example `fir-token-toolkit-demo`. The Users panel,
User Custom Claims and App Check use it to build their requests. It normally
comes from the key file, but you can edit it.

## API key

The project's Web API key. The ID-token tabs and App Check send it to Google
with each request. The field is masked.

## App ID

The web app's App ID, used only by the App Check tab.

## remember API key & App ID

Off by default. When ticked, the API key, App ID and App Check debug token are
saved to disk in plaintext for **every** profile. When unticked, they are kept
in memory only and cleared from every profile the next time the app saves. See
[Settings and storage](reference/settings-storage.md).

## Selected UID

Once you pick a user in the [Users panel](guide/users-panel.md), a **Selected
UID** row shows their UID and, in grey, their email or phone and display name.
The UID-based tabs act on this user.

# Troubleshooting

Messages appear in one of three places: the status bar under the tabs, the
Users panel, or in red as `Error: …` under a tab's button. Error text from
Google is passed through unchanged, so search the message itself if it is not
listed here.

## The app does not open

If no graphics backend works, the app shows a dialog that begins
*"Firebase Token Toolkit could not start."* The app tries OpenGL first and then
wgpu, which can fall back to a software renderer, so this almost always means a
virtual machine or remote desktop session with no usable graphics at all.
Enabling 3D acceleration for the VM usually fixes it.

On Linux, launching from a terminal shows the detail. A line like
`OpenGL backend unavailable (…); retrying with wgpu` is informational; the app
carries on with wgpu.

## Service account

| Message | Cause and fix |
|---|---|
| `Could not load service account: read service account file: <path>` | The file cannot be read. Check that it exists and that your user can read it. |
| `Could not load service account: parse service account JSON: <path>` | The file is not valid JSON, or not a service-account key. Download a fresh key. |
| `Could not load service account: service account JSON must contain client_email and private_key` | The JSON is some other kind of credential, such as a web app config. Use **Generate new private key** in the Firebase console. |
| `parse RSA private key (must be PEM PKCS#8 / PKCS#1)` | The `private_key` field has been edited or mangled. Download a fresh key. |
| `The file picker closed unexpectedly` | On Linux, the XDG desktop portal is missing or crashed. Install `xdg-desktop-portal-gtk` or your desktop's portal. |
| `● not configured` after a restart | The saved key file has moved. Use **Browse…** to find it again. |

## Authentication with Google

| Message | Cause and fix |
|---|---|
| `oauth2 token error (400): … invalid_grant …` | The key has been deleted or disabled in Google Cloud, or your system clock is off by several minutes. Check the clock, then generate a new key. |
| `oauth2 token error (401): …` | The service account no longer exists. Generate a key for an active account. |
| `POST oauth2 token` (no further detail) | The request never reached Google. Check your network connection and any proxy. |

## Users panel

| Message | Cause and fix |
|---|---|
| `Load a service account and set Project ID to browse users.` | Load a key and make sure **Project ID** is filled in. |
| `No user found for that query` | Lookup needs the whole value: a complete email (case does not matter), a phone number in full `+` country-code form, or an exact UID. To match part of a value, type into the search box without pressing <kbd>Enter</kbd>; that filters the users already loaded. |
| `API error (403): …` | The service account lacks permission to read users, or the Identity Toolkit API is disabled for the project. |
| `GET accounts:batchGet` | The request never reached Google. Check your connection. |

## Tokens

| Message | Cause and fix |
|---|---|
| `API key required` | Enter the Web API key in the top bar. |
| `API error (400): INVALID_CUSTOM_TOKEN : Invalid assertion format. 3 dot separated segments required.` | The pasted text is not a whole token. It usually lost characters while being copied. |
| `API error (400): INVALID_CUSTOM_TOKEN …` with other text | The token has expired (custom tokens last one hour) or is malformed. Generate a new one. |
| `API error (400): CREDENTIAL_MISMATCH` | The custom token was signed with a key from a different project than the API key belongs to. Check that the profile's key and API key are from the same project. |
| `API error (400): API key not valid. Please pass a valid API key.` | The API key is wrong or deleted. Copy it again from **Project settings** → **General**. |
| `API error (403): … are blocked.` | The API key has restrictions that exclude the Identity Toolkit API or the App Check API. Loosen them in Google Cloud Console. |
| `Invalid claims JSON: …` / `custom claims must be a JSON object` | The optional claims box must hold a JSON object, such as `{"role":"admin"}`, or be empty. |
| `Paste a custom token first` | The *Custom -> ID Token* input is empty. |

## User Custom Claims

| Message | Cause and fix |
|---|---|
| `Serialized claims are N bytes; Firebase limit is 1000 bytes.` | Firebase rejects claims over 1,000 bytes. Shorten keys or move data to your database. |
| `Claims editor is empty (use 'Clear all claims' to wipe).` | **Save** with an empty editor is refused on purpose. To remove every claim, use **Clear all claims**. |
| `Claims must be a JSON object.` | Top-level arrays, strings and numbers are not allowed. |
| `User not found` | The user was deleted after you picked them. Refresh the Users panel. |

## App Check

| Message | Cause and fix |
|---|---|
| `Debug token required` | Paste the debug token into the tab. |
| `API error (403): App attestation failed.` | The debug token is not registered for this App ID. Check it under **App Check** → **Manage debug tokens**, and that it was registered for the same app as the **App ID** in the top bar. |
| `API error (403): …` with other text | The App Check API is not enabled for the project, or the API key's restrictions exclude it. |
| `API error (404): …` | The App ID does not exist in the project named in **Project ID**. |

## Linux desktop issues

**Browse… does nothing.** The file chooser goes through an XDG desktop
portal. Install `xdg-desktop-portal-gtk` (or your desktop's portal) and log in
again.

**Copy shows `copy failed: …`.** The clipboard needs X11 or XWayland. Under a
pure Wayland session without XWayland, select the token text and copy it with
<kbd>Ctrl</kbd>+<kbd>C</kbd> instead.

## About dialog links return 404

The **Changelog** and **Security notes** links in the About dialog point at
the git tag matching the app's version. A build from an untagged commit has no
such tag, so the links 404. Release builds are not affected.

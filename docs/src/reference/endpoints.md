# Google endpoints

Every network request the app makes goes to a Google endpoint over TLS, and
only when you click a button that needs it. There is no telemetry, analytics or
update check. Opening a link from the About dialog hands that URL to your
browser; nothing else leaves the app.

| Action in the app | Request | Authenticated with |
|---|---|---|
| **Generate** on *UID -> Custom Token* | None. The token is signed locally. | — |
| **Generate** on *UID -> ID Token*, **Exchange** on *Custom -> ID Token* | `POST identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken` | Selected app's API key |
| **Load users** / **Refresh** | `GET identitytoolkit.googleapis.com/v1/projects/{project}/accounts:batchGet` (1,000 per page, up to 5,000) | OAuth access token |
| **Lookup**, **Load current** | `POST identitytoolkit.googleapis.com/v1/projects/{project}/accounts:lookup` | OAuth access token |
| **Save**, **Clear all claims** | `POST identitytoolkit.googleapis.com/v1/projects/{project}/accounts:update`, then a lookup to read the result back | OAuth access token |
| **Exchange** on *App Check* | `POST firebaseappcheck.googleapis.com/v1beta/projects/{project}/apps/{app}:exchangeDebugToken` | Selected app's API key |
| **Load apps** | `GET firebase.googleapis.com/v1beta1/projects/{project}:searchApps`, then for each app `GET …/webApps/{app}/config`, `…/androidApps/{app}/config` and `…/androidApps/{app}/sha`, or `…/iosApps/{app}/config` | OAuth access token |

## API keys and app identity

Requests authenticated with an API key send it as the `key` query parameter.
For an Android or iOS app, the request also names the app, so that keys
restricted to that app are accepted:

| App type | Headers |
|---|---|
| Web | none |
| Android | `X-Android-Package: <package>`, `X-Android-Cert: <SHA-1, 40 uppercase hex digits>` |
| iOS | `X-Ios-Bundle-Identifier: <bundle ID>` |

A header is left out when its value is empty, and a SHA-1 that is not 40 hex
digits is not sent.

## OAuth access tokens

Calls marked *OAuth access token* first exchange a JWT, signed with the
service-account key, at `oauth2.googleapis.com/token`. The token is requested
with two scopes:

- `https://www.googleapis.com/auth/identitytoolkit`
- `https://www.googleapis.com/auth/cloud-platform`

It is cached in memory and refreshed when it is within about a minute of
expiring. Switching profiles discards it.

`cloud-platform` is a broad scope. What the token can actually do is limited by
the IAM roles granted to the service account, which is one more reason to use
a development project's key. See [Security](../security.md).

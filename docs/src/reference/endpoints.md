# Google endpoints

Every network request the app makes goes to a Google endpoint over TLS, and
only when you click a button that needs it. There is no telemetry, analytics or
update check. Opening a link from the About dialog hands that URL to your
browser; nothing else leaves the app.

| Action in the app | Request | Authenticated with |
|---|---|---|
| **Generate** on *UID -> Custom Token* | None. The token is signed locally. | — |
| **Generate** on *UID -> ID Token*, **Exchange** on *Custom -> ID Token* | `POST identitytoolkit.googleapis.com/v1/accounts:signInWithCustomToken` | Web API key |
| **Load users** / **Refresh** | `GET identitytoolkit.googleapis.com/v1/projects/{project}/accounts:batchGet` (1,000 per page, up to 5,000) | OAuth access token |
| **Lookup**, **Load current** | `POST identitytoolkit.googleapis.com/v1/projects/{project}/accounts:lookup` | OAuth access token |
| **Save**, **Clear all claims** | `POST identitytoolkit.googleapis.com/v1/projects/{project}/accounts:update`, then a lookup to read the result back | OAuth access token |
| **Exchange** on *App Check* | `POST firebaseappcheck.googleapis.com/v1beta/projects/{project}/apps/{app}:exchangeDebugToken` | Web API key |

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

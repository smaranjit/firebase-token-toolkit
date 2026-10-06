# UID -> ID Token

Gets a Firebase ID token for the selected user in one click. The app signs a
custom token locally, exchanges it with Google through `signInWithCustomToken`,
and shows the resulting ID token.

**Needs:** a service account, the Web API key, and a selected user.

## Generating a token

1. Pick a user in the [Users panel](users-panel.md).
2. Optionally, enter claims in **Custom claims (optional JSON)**. They are
   added to this token only.
3. Click **Generate**.

![An ID token for alice@example.com, with refresh token, expiry, and decoded claims](../images/uid-to-id-result.png)

Under the token you get:

- **Refresh token:** a shortened view of the refresh token Google returned.
  It is shown for reference only; there is no button to copy it.
- **Expires in:** the token's lifetime in seconds, normally `3600s`.
- **Decoded JWT claims:** everything the ID token asserts. Claims stored on the
  user with [User Custom Claims](custom-claims.md) appear here alongside the
  standard ones.

## Using the token

Send it as a bearer token to anything that verifies Firebase ID tokens, such as
your backend's `verifyIdToken` call or a Firebase security rule test:

```bash
curl -H "Authorization: Bearer $ID_TOKEN" https://localhost:8080/api/me
```

The token is a real credential for that user until it expires. Treat it like a
password.

## Signing in as the user

Exchanging a custom token is a real sign-in. The user's **Signed in** date in
the Firebase console updates, `auth_time` in the token is set to now, and the
`firebase` claim records `"sign_in_provider": "custom"`. If your backend
treats the sign-in provider specially, or you track sign-in activity, bear that
in mind.

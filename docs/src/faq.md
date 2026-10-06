# FAQ

Short answers to the questions that usually bring people here, with links to
the full pages.

## How do I get a Firebase ID token to test my API in Postman or curl?

Load your service-account key, click **Load apps**, pick a user in the Users
panel, and click **Generate** on the [UID -> ID Token](guide/uid-to-id-token.md)
tab. Click **Copy** and send the token as a bearer token:

```bash
curl -H "Authorization: Bearer <ID token>" https://localhost:8080/api/me
```

In Postman, choose **Authorization** → **Bearer Token** and paste it. The
[Quick start](quick-start.md) walks through it with screenshots.

## How do I test a backend that verifies Firebase ID tokens?

The tokens the app produces are real ID tokens issued by Google for your
project, so they pass `verifyIdToken` in the Firebase Admin SDKs and any other
check against Google's public keys. Generate one for the user you want to test
as, and call your API with it. Add claims for a single token in the **Custom
claims** box to test role checks without touching the user record.

## How do I sign in as a specific user without their password?

The app signs a custom token for the user's UID with your service-account key
and exchanges it with Google, the same flow as `createCustomToken` followed by
`signInWithCustomToken`. You never need the user's password, which is also why
the service-account key must be kept safe. See [Security](security.md).

## How do I set Firebase custom claims without writing code?

Use the [User Custom Claims](guide/custom-claims.md) tab. **Load current**
shows the user's claims as JSON, you edit them, and **Save** writes them to the
user record, the same as `setCustomUserClaims` in the Admin SDK. A counter keeps
you under Firebase's 1,000-byte limit.

## Why don't my new custom claims show up in the ID token?

Tokens issued before the change keep the old claims until they expire, about an
hour later. Generate a new token on the *UID -> ID Token* tab, or force a
refresh on the client, for example `getIdToken(true)`. See
[When changes take effect](guide/custom-claims.md#when-changes-take-effect).

## How do I get a Firebase App Check token for testing?

Register a debug token for your app under **App Check** → **Manage debug
tokens** in the Firebase console, then paste it into the
[App Check](guide/app-check.md) tab and click **Exchange**. Send the result in
the `X-Firebase-AppCheck` header. This works for web, Android and iOS apps.

## Can I use an API key restricted to an Android or iOS app?

Yes. Select the Android or iOS app in the top bar and the app sends its package
name and SHA-1, or bundle ID, with each request, as the Firebase SDKs do. See
[API keys restricted to an app](firebase-setup.md#api-keys-restricted-to-an-app).

## How do I decode a Firebase JWT?

Every token the app shows has a **Decoded JWT claims** table underneath, with
`iat`, `exp` and `auth_time` converted to readable UTC times. The table shows
what a token says; it does not verify the signature. See
[Reading the results](guide/reading-results.md).

## How long do the tokens last?

Custom tokens and ID tokens last one hour. An App Check token lasts for the
time to live set in your App Check settings, shown as **TTL** under the token.
When a token expires, generate a new one.

## Does it work with the Firebase Auth emulator?

No. The app talks to Google's production endpoints for your project, so it needs
a real Firebase project. Use a development project rather than production; see
[Preparing your Firebase project](firebase-setup.md).

## Which platforms does it run on?

Linux (x86_64, glibc 2.35 or newer), Windows (x86_64) and macOS 11 or newer on
Apple Silicon or Intel. See [Installation](installation.md).

## Is it free?

Yes. It is open source under the MIT licence, and its source is on
[GitHub](https://github.com/smaranjit/firebase-token-toolkit).

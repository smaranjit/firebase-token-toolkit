# Preparing your Firebase project

The app needs up to three things from your Firebase project: a
service-account key, the project ID, and the details of the apps you test
with. Which ones depend on the tabs you plan to use:

| Tab | Service-account key | Project ID | App's API key | App ID |
|---|:-:|:-:|:-:|:-:|
| UID -> Custom Token | ✔ | for the Users panel | | |
| UID -> ID Token | ✔ | for the Users panel | ✔ | |
| Custom -> ID Token | | | ✔ | |
| User Custom Claims | ✔ | ✔ | | |
| App Check | | ✔ | ✔ | ✔ |

> **Use a development project.** Nothing in the app needs production
> credentials, and a service-account key can sign in as any user in its
> project. See [Security](security.md).

## Service-account key

1. Open the [Firebase console](https://console.firebase.google.com/) and pick
   your project.
2. Go to **Project settings** → **Service accounts**.
3. Click **Generate new private key**, then **Generate key**. A JSON file
   downloads.

Keep the file somewhere stable. The app saves its path, not its contents, and
reads it again on every launch.

The key the console generates belongs to the project's
`firebase-adminsdk-…@<project>.iam.gserviceaccount.com` account, which already
has the access the app needs. If you use a different service account, it needs
permission to read and update Firebase Authentication users for the Users
panel and the User Custom Claims tab, for example the **Firebase
Authentication Admin** role (`roles/firebaseauth.admin`). Signing custom tokens
happens locally with the key and needs no extra role.

## Project ID

The app fills this in from the service-account file, so you rarely need to
type it. It is also shown under **Project settings** → **General**.

## Apps: web, Android and iOS

Each [profile](profiles.md) holds a list of the project's apps, and the tabs
that need an API key use the one you select. Any app registered in the
project works, whatever its platform.

**The quick way:** with the service-account key loaded, click **Load apps** in
the top bar. The app reads every web, Android and iOS app in the project
through the Firebase Management API and fills in each app's App ID, API key,
package name and SHA-1 (Android) or bundle ID (iOS). See
[The top bar](configuration.md#apps).

The service account needs to be allowed to read Firebase apps
(`firebase.clients.list` and `firebase.clients.get`). The key the console
generates already is; for another account, the **Firebase Viewer** role
(`roles/firebase.viewer`) covers it.

**By hand:** under **Project settings** → **General** → **Your apps**, each app
shows its **App ID**, for example `1:316659987175:android:b9522e663c7b203d12a842`.
Its API key is in the config file offered there (`google-services.json`,
`GoogleService-Info.plist`, or the web snippet's `apiKey`). The project-wide
**Web API Key** on the same page also works for any app, as long as it has no
application restriction.

### API keys restricted to an app

Google Cloud lets you restrict a key to particular Android apps (package name
plus signing-certificate SHA-1) or iOS apps (bundle ID). Google accepts such a
key only from a request that names the app, the way the Firebase SDKs do. The
app sends those details for you, taken from the selected app:

| App type | Sent with the request |
|---|---|
| Web | the API key only |
| Android | `X-Android-Package` (package name) and `X-Android-Cert` (SHA-1) |
| iOS | `X-Ios-Bundle-Identifier` (bundle ID) |

So for a restricted key, make sure the app's **Package** and **SHA-1**, or
**Bundle ID**, match one of the entries in the key's restriction. **Load apps**
fills them from Firebase, using the first SHA-1 certificate registered for an
Android app; if the key allows a different certificate, paste that one in.

If you have restricted which **APIs** a key may call, allow at least the
**Identity Toolkit API**, and the **Firebase App Check API** if you use the
App Check tab.

## App Check debug token (App Check only)

The App Check tab exchanges a *debug token* you have registered for the app.
This works the same for web, Android and iOS apps:

1. Register the app with App Check under **App Check** → **Apps** if you have
   not already.
2. Open the app's overflow menu (⋮) and choose **Manage debug tokens**.
3. Click **Add debug token**, give it a name, and either generate a token or
   paste one your client printed (the browser console on the web, Logcat on
   Android, the Xcode console on iOS). Copy the value; it is a UUID.

Treat the debug token like a password. Anyone holding it can get valid App
Check tokens for your app.

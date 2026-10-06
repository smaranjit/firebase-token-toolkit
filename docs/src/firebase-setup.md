# Preparing your Firebase project

The app needs up to three things from your Firebase project. Which ones depend
on the tabs you plan to use:

| Tab | Service-account key | Project ID | Web API key | App ID |
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

## Web API key

Under **Project settings** → **General**, copy **Web API key**. If your project
has no web app yet, add one from the same page first.

If you have restricted the key in Google Cloud Console, allow at least the
**Identity Toolkit API**, and the **Firebase App Check API** if you use the
App Check tab.

## App ID (App Check only)

Under **Project settings** → **General** → **Your apps**, copy the **App ID** of
the web app, which looks like `1:316659987175:web:ebcef8864368ab1a12a842`.

## App Check debug token (App Check only)

The App Check tab exchanges a *debug token* you have registered for the app:

1. Register the app with App Check under **App Check** → **Apps** if you have
   not already.
2. Open the app's overflow menu (⋮) and choose **Manage debug tokens**.
3. Click **Add debug token**, give it a name, and either generate a token or
   paste one your client printed. Copy the value; it is a UUID.

Treat the debug token like a password. Anyone holding it can get valid App
Check tokens for your app.

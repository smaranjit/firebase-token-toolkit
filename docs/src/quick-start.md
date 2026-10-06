# Quick start

This walk-through goes from a fresh install to an ID token for one of your
users. You need a service-account key file; see
[Preparing your Firebase project](firebase-setup.md) if you do not have one
yet. The app reads everything else from the project.

## 1. Open the app

On first launch the status indicator reads **not configured** in red, and the
tabs ask you to load a service account.

![The app on first launch, with no service account loaded](images/first-run.png)

## 2. Load the service-account key

Click **Browse…** next to **Service account** and choose the JSON key file.

![The file chooser, filtered to JSON files](images/browse-dialog.png)

The path appears in the top bar, **Project ID** fills itself in from the key,
and the indicator changes to **SA only**. A green *Service account loaded*
message confirms it; click **dismiss** to hide it.

![The service account loaded and Project ID filled in](images/service-account-loaded.png)

## 3. Load the project's apps

Click **Load apps**. The app lists the project's web, Android and iOS apps and
fills in each one's App ID and API key. Pick the app you want to test as from
the **App** dropdown; the indicator turns green and reads **ready**.

![The App dropdown listing the project's web, Android and iOS apps](images/apps-dropdown.png)

If the service account is not allowed to read apps, enter an app by hand
instead: **+ Add app**, choose its type, and paste its **API key** and
**App ID**. See [The top bar](configuration.md#apps).

## 4. Load your users

Click **Load users** in the Users panel. The app fetches up to 5,000 users and
lists each one by email (or phone number), display name and UID.

![The Users panel listing five users](images/users-loaded.png)

## 5. Generate an ID token

1. Click a user in the list. Their UID appears under **Selected UID** in the
   top bar.
2. Open the **UID -> ID Token** tab.
3. Click **Generate**.

The ID token appears in a box with a **Copy** button. Under it are a shortened
refresh token, the time until the token expires, and a collapsible **Decoded
JWT claims** table.

![An ID token for alice@example.com with its decoded claims](images/uid-to-id-result.png)

Click **Copy** and use the token wherever your backend expects one, for
example:

```bash
curl -H "Authorization: Bearer <paste token here>" https://localhost:8080/api/me
```

## 6. Keep the setup for next time

The service-account path, Project ID and app list are saved automatically.
API keys are saved only if you tick **remember API keys & debug tokens**;
without it, click **Load apps** again after each launch, or paste the key. See
[Settings and storage](reference/settings-storage.md) for what that writes to
disk.

## Next steps

- Add [custom claims to a token](guide/uid-to-custom-token.md#adding-claims-to-one-token),
  or [store claims on the user](guide/custom-claims.md) so every token carries
  them.
- Set up [profiles](profiles.md) if you work with more than one Firebase
  project.

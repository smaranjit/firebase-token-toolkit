Firebase Token Toolkit
https://github.com/smaranjit/firebase-token-toolkit

A desktop GUI for the Firebase auth tokens you need while developing: custom
tokens, ID tokens, a user's custom claims, and App Check debug tokens.


RUNNING IT

  Windows   Run firebase-token-toolkit.exe

            SmartScreen may warn on first launch because the binary is not
            signed. Choose "More info", then "Run anyway".

  macOS     Drag the app into Applications, then open it.

            Gatekeeper blocks it on first launch because the app is not
            notarized. Either right-click the app and choose Open, or clear
            the quarantine flag from a terminal:

              xattr -dr com.apple.quarantine \
                "/Applications/Firebase Token Toolkit.app"

  Linux     ./firebase-token-toolkit

            The service account file picker needs an XDG desktop portal
            installed. The Copy buttons use the X11 clipboard, so under
            Wayland you need XWayland running.


SETTING IT UP

  1. Service account. Browse for the JSON from the Firebase Console, under
     Project Settings > Service accounts > Generate new private key. The
     project ID fills itself in from the file.

  2. Apps. Click Load apps to import the project's web, Android and iOS
     apps with their API keys. Or click + Add app and paste an API key and
     App ID from Firebase Console > Project Settings > General > Your apps.

  3. Pick the app to use from the App list. Its API key is used by the
     ID token and App Check tabs.

  The indicator turns green once the service account is loaded and the
  selected app has an API key.


IF NOTHING OPENS

  Startup failures are reported in a dialog rather than failing silently.
  Graphics drivers are the usual cause; the app falls back to software
  rendering where no GPU is available, so this should be rare.


MORE

  User guide, with screenshots of every tab and a troubleshooting section:
  https://smaranjit.github.io/firebase-token-toolkit/

  Source, changelog, security notes and issue tracker:
  https://github.com/smaranjit/firebase-token-toolkit

  The About dialog, reached from the footer, links the changelog and
  security notes for this exact version.

  MIT licensed. See the LICENSE file alongside this one.

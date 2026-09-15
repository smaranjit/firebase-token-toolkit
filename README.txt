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

  2. API key. Firebase Console > Project Settings > General > Web API Key.
     Only needed for the ID token and App Check tabs.

  3. App ID. Only needed for the App Check tab.

  The indicator in the top right turns green once the service account is
  loaded and an API key is set.


IF NOTHING OPENS

  Startup failures are reported in a dialog rather than failing silently.
  Graphics drivers are the usual cause; the app falls back to software
  rendering where no GPU is available, so this should be rare.


MORE

  Full documentation, changelog, security notes and issue tracker:
  https://github.com/smaranjit/firebase-token-toolkit

  The About dialog, reached from the footer, links the changelog and
  security notes for this exact version.

  MIT licensed. See the LICENSE file alongside this one.

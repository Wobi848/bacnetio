# Concept: a note when a new version is out

Asked for on 3 November 2026: bacnetio should say when a newer release is published.

## What the user sees

- When a newer release exists, the sidebar shows *New version 0.2.0* under the status, as a link to
  its release page on GitHub. Nothing else changes; bacnetio never downloads or installs anything.
- Settings, card *Keys and version*: the running version, whether a newer one exists or when the
  last check had no answer, and a switch *Look for new versions*. It is on by default.
- Without internet, as on most plants, there is no note in the sidebar; the settings say the check
  had no answer.

## How it works

- The page asks `GET /api/update` at most once a day (the time of the last check is kept in the
  browser), and only while the switch is on. With the switch off, bacnetio never goes online.
- The server asks `https://github.com/Wobi848/bacnetio/releases/latest` and reads only the
  redirect: GitHub answers with the address of the newest release, `…/releases/tag/v0.2.0`. No
  account, no API limit, no body to parse. Without a release the redirect goes to `…/releases`,
  which means *nothing newer*.
- TLS with rustls (the `ring` provider, as in mbusio) and the root certificates of `webpki-roots`,
  so it works the same on Windows, Linux and Android without system libraries.
- A version is newer when its numbers (`major.minor.patch`) are larger; anything else in the tag
  (a pre-release such as `v0.2.0-rc1`) is not offered.

## Privacy

GitHub sees the computer's public address and the request, nothing else: no ID, no version, no
plant data. The switch and this paragraph in the README say so.

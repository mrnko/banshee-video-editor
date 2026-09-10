# Banshee updates

Every push to `main` runs `.github/workflows/release.yml`, builds a signed NSIS installer and portable ZIP, and publishes a GitHub Release with `latest.json`.

## One-time GitHub setup

Create this Actions repository secret:

- `TAURI_SIGNING_PRIVATE_KEY` — the complete contents of the Tauri private key.

The updater key is stored outside the repository at:

`C:\Users\Oleksii\.tauri\banshee-updater.key`

The key has no password, so `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` may be omitted. Keep a secure backup of the private key. Losing it prevents existing installations from accepting future updates. Never commit it.

## Publishing

Update the first heading and change list in `RELEASE_NOTES.md`, then push to `main`. The workflow derives a unique version such as `0.1.3-main.142`, signs the updater artifact, and publishes the release as the repository's latest release.

Raise `VERSION` before starting the next release train. Keep the workspace package, Cargo workspace, desktop package, and Tauri versions synchronized; `npm run version:check` validates them.

# Releasing

How to cut a Shrimp release and publish updates. The update code is described in
[ARCHITECTURE.md](ARCHITECTURE.md#updates-pkg-03-pkg-04); development builds in
[DEVELOPMENT.md](DEVELOPMENT.md).

## Release and development builds

|                   | Release                                            | Development                                 |
| ----------------- | -------------------------------------------------- | ------------------------------------------- |
| Name              | Shrimp                                             | Shrimp Dev                                  |
| Bundle identifier | `com.shrimp.desktop`                               | `com.shrimp.desktop.dev`                    |
| Built by          | `scripts/macos/release.sh`                         | `pnpm app:dev` (`scripts/macos/dev-app.sh`) |
| Signed with       | Developer ID Application (Apple-issued)            | "Shrimp Dev" (self-signed, local)           |
| Notarized         | Yes, app and DMG stapled                           | No                                          |
| tauri-plugin-mcp  | Not registered                                     | Registered                                  |
| Data, keychain    | `~/Library/Application Support/com.shrimp.desktop` | `…/com.shrimp.desktop.dev`                  |

The two identifiers never collide, so a release install and the dev app keep separate
settings, data, keychain entries and privacy grants.

## One-time setup (macOS)

1. **Signing identity.** A "Developer ID Application" certificate with its private key in
   the login keychain. `security find-identity -v -p codesigning` must list it. The script
   uses the first one it finds; set `SHRIMP_SIGNING_IDENTITY` to pick another.
2. **Notarization credentials** in a notarytool keychain profile, named `shrimp-notary`
   by default (`SHRIMP_NOTARY_PROFILE` picks another). With an App Store Connect API key
   (Users and Access › Integrations, role Developer or higher):

   ```sh
   xcrun notarytool store-credentials shrimp-notary \
     --key /path/to/AuthKey_<KEY_ID>.p8 --key-id <KEY_ID> --issuer <ISSUER_ID>
   ```

   Or with an Apple ID and an app-specific password:

   ```sh
   xcrun notarytool store-credentials shrimp-notary \
     --apple-id <APPLE_ID> --team-id <TEAM_ID> --password <APP_SPECIFIC_PASSWORD>
   ```

   `xcrun notarytool history --keychain-profile shrimp-notary` checks it.

3. **Updater signing key.** Updates are signed with a minisign key pair. The private key
   and its password never enter the repository; only the public key is in
   `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).

   | What         | Where                                                                               |
   | ------------ | ----------------------------------------------------------------------------------- |
   | Private key  | `~/.tauri/shrimp-updater.key` (mode 600; `SHRIMP_UPDATER_KEY` overrides)            |
   | Public key   | `~/.tauri/shrimp-updater.key.pub`, and copied into `tauri.conf.json`                |
   | Key password | Login keychain, generic password, service `shrimp-updater-key`, account = your user |

   To create them on a new machine (the owner's already exist; see the warning below):

   ```sh
   mkdir -p ~/.tauri && chmod 700 ~/.tauri
   PW=$(openssl rand -base64 32)
   security add-generic-password -a "$USER" -s shrimp-updater-key \
     -l "Shrimp updater signing key password" -w "$PW"
   ./node_modules/.bin/tauri signer generate --ci -p "$PW" -w ~/.tauri/shrimp-updater.key >/dev/null
   unset PW
   ```

   Run the CLI directly, not through `pnpm tauri`: pnpm prints the command line, password
   included. Then put the content of `shrimp-updater.key.pub` into `plugins.updater.pubkey`.

   **Back up the private key and its password.** Every installed copy only accepts
   updates signed with this key. A lost key, or a new key pair, means existing installs
   can't update and must be reinstalled from a DMG.

## Cutting a release

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
   `package.json`, run `pnpm check`, and commit.
2. Build, with the URL where `latest.json` will be served:

   ```sh
   SHRIMP_UPDATE_FEED=https://example.com/shrimp/latest.json scripts/macos/release.sh
   ```

   Without `SHRIMP_UPDATE_FEED` the build has no feed: its tray menu says "Updates aren't
   configured" and it never checks. Only `https://` is accepted, or `http://` to
   `127.0.0.1` or `localhost` for local tests.

3. Publish:
   - the DMG, for new installs;
   - `Shrimp_<version>_aarch64.app.tar.gz` and `latest.json` together, at the feed's
     location (the archive URL in `latest.json` is the feed URL's directory plus the
     archive name). Upload the archive first and `latest.json` last, so the feed never
     points at a missing file.

`--skip-notarize` signs without notarizing, for a quick local build. `--version X.Y.Z`
builds another version without editing the version files, for test builds.

### What the script does

1. Checks the machine (Apple Silicon), the signing identity, the notary profile (it
   prints the `store-credentials` command when the profile is missing), the updater key
   and its password, and the feed URL.
2. Runs `tauri build --bundles app` with the version and the feed merged into the
   configuration. Tauri signs the app with the Developer ID identity, the hardened runtime,
   a secure timestamp and `src-tauri/entitlements.plist`. The script stops unless
   `codesign --verify --deep --strict` passes and the runtime flag is set.
3. Zips the app, submits it to the notary service, waits, and staples the ticket to
   `Shrimp.app`. A rejection prints Apple's log.
4. Packs the stapled app into `Shrimp_<version>_aarch64.app.tar.gz` (no macOS metadata or
   extended attributes) and signs it with the updater key, bound to the version
   (`tauri signer sign --app-version`). With a feed, writes `latest.json`.
5. Builds the DMG (the stapled app and a link to `/Applications`, HFS+, LZFSE), signs it,
   notarizes it, staples it and checks it with `spctl`.

### Outputs

| File                                                                          | What                       |
| ----------------------------------------------------------------------------- | -------------------------- |
| `src-tauri/target/release/bundle/macos/Shrimp.app`                            | The stapled app            |
| `src-tauri/target/release/bundle/dmg/Shrimp_<version>_aarch64.dmg`            | The DMG                    |
| `src-tauri/target/release/bundle/updater/Shrimp_<version>_aarch64.app.tar.gz` | The update archive         |
| `…/updater/Shrimp_<version>_aarch64.app.tar.gz.sig`                           | Its minisign signature     |
| `…/updater/latest.json`                                                       | The feed (with a feed URL) |

### Entitlements

`src-tauri/entitlements.plist` grants only `com.apple.security.device.audio-input`, which
the hardened runtime needs for microphone capture. Nothing else is needed:

- Shrimp sends no Apple Events to other apps. Media control runs `osascript` as a separate,
  Apple-signed process that loads MediaRemote, and sends commands in-process
  (`platform/macos/media.rs`).
- Accessibility (the event tap, pasting, reading fields) is a privacy grant, not an
  entitlement.
- Everything is linked statically or comes from Apple, so library validation stays on.

### Checking a DMG

Without installing it:

```sh
hdiutil attach -nobrowse -readonly Shrimp_<version>_aarch64.dmg
ls -l /Volumes/Shrimp                    # Shrimp.app and Applications -> /Applications
codesign --verify --deep --strict --verbose=2 /Volumes/Shrimp/Shrimp.app
spctl -a -vv /Volumes/Shrimp/Shrimp.app   # accepted, source=Notarized Developer ID
xcrun stapler validate /Volumes/Shrimp/Shrimp.app
hdiutil detach /Volumes/Shrimp
xcrun stapler validate Shrimp_<version>_aarch64.dmg
spctl -a -t open --context context:primary-signature -vv Shrimp_<version>_aarch64.dmg
```

## The update feed

`latest.json` follows tauri-plugin-updater's static format:

```json
{
  "version": "0.1.1",
  "pub_date": "2026-10-13T22:00:00.000Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<content of the .sig file>",
      "url": "https://example.com/shrimp/Shrimp_0.1.1_aarch64.app.tar.gz"
    }
  }
}
```

A release is offered when its version is greater than the running one. The signature
must verify against the public key, and its signed version must match `version`
(`requireSignedVersion`), so a tampered feed can't pass an older archive off as new.

## Testing an update locally

1. Build the older version with a loopback feed and keep its app somewhere outside
   `/Applications`:

   ```sh
   SHRIMP_UPDATE_FEED=http://127.0.0.1:8765/latest.json scripts/macos/release.sh
   ditto src-tauri/target/release/bundle/macos/Shrimp.app /tmp/shrimp-test/Shrimp.app
   ```

2. Build the newer one with the same feed, `--version 0.1.1`, and serve its `updater/`
   folder: `python3 -m http.server 8765 --bind 127.0.0.1 --directory <folder>`.
3. Open `/tmp/shrimp-test/Shrimp.app`, choose Check for updates… in the menu bar, then
   Restart now. Shrimp replaces the bundle in place, restarts into the new version and
   opens the main window. A fresh install downloads the speech model (about 560 MB) at
   launch; hard-linking the dev app's files from
   `~/Library/Application Support/com.shrimp.desktop.dev/models/` into
   `…/com.shrimp.desktop/models/` first avoids that.
4. Quit it, stop the server, and delete the test app and what the test created:
   `~/Library/Application Support/com.shrimp.desktop`, `~/Library/Caches/com.shrimp.desktop`
   and `~/Library/WebKit/com.shrimp.desktop`.

To see an update wait for a dictation, do the same with the dev app, which tauri-mcp can
drive: build each version with
`pnpm tauri build --debug --bundles app --config src-tauri/tauri.conf.dev.json --config '{"version":"0.1.1","plugins":{"updater":{"endpoints":["http://127.0.0.1:8765/latest.json"],"dangerousInsecureTransportProtocol":true}}}'`,
pack and sign the newer `Shrimp Dev.app` as `release.sh` does, start a dictation with
`flow_bar_control`, and run `run_notice_action` with `restartToUpdate`. Run `pnpm app:dev`
afterwards to rebuild the normal dev app.

## Privacy grants across updates (PKG-05)

macOS keys Microphone and Accessibility grants (TCC) to the app's designated requirement.
For a Developer ID build it is:

```text
designated => identifier "com.shrimp.desktop" and anchor apple generic and certificate 1[field.1.2.840.113635.100.6.2.6] /* exists */ and certificate leaf[field.1.2.840.113635.100.6.1.13] /* exists */ and certificate leaf[subject.OU] = URKK7Q7758
```

It names only the bundle identifier, Apple's Developer ID chain and the team ID. It holds
no version, build, code hash or certificate serial, so every release built this way
satisfies it, and an update keeps the grants. The grants are lost only if one of these
changes:

- the bundle identifier (`identifier` in `tauri.conf.json`);
- the team (signing with another team's certificate);
- the kind of certificate (for example an ad-hoc or self-signed build).

Renewing the Developer ID certificate keeps the same team ID, so grants survive that too.
`app/system.rs` re-checks the grants on the first launch of each new version (ONB-08)
and asks for any that are missing.

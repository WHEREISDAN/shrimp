# Releasing

How to cut a Shrimp release and publish updates. The update code is described in
[ARCHITECTURE.md](ARCHITECTURE.md#updates-pkg-03-pkg-04); development builds in
[DEVELOPMENT.md](DEVELOPMENT.md).

Releases are built locally, not by CI: signing and notarization need credentials that stay
on the release machines (the Developer ID identity, the notarytool profile and the updater
key). The repository has no GitHub Actions workflows. Releases and the update feed are
published on [GitHub Releases](https://github.com/WHEREISDAN/shrimp/releases).

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

On Windows the release is built by `scripts/windows/release.ps1` and the debug app by
`pnpm app:dev:win`; data lives in `%APPDATA%\com.shrimp.desktop` and
`%APPDATA%\com.shrimp.desktop.dev`, API keys in Credential Manager. See
[Windows](#windows).

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

   To create a new key pair (only for a fork or a fresh project; see the warning below):

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

   A fork that publishes its own builds needs its own key pair, its own public key in
   `tauri.conf.json` and its own feed URL.

## Cutting a release

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (and the
   `shrimp` entry in `src-tauri/Cargo.lock`) and `package.json`, add the release to
   `CHANGELOG.md`, run `pnpm notices` if dependencies changed, run `pnpm check`, and
   commit.
2. Build, with the project's feed:

   ```sh
   SHRIMP_UPDATE_FEED=https://github.com/WHEREISDAN/shrimp/releases/latest/download/latest.json \
     scripts/macos/release.sh
   ```

   Every release build must set it. Without `SHRIMP_UPDATE_FEED` the build has no feed:
   its tray menu says "Updates aren't configured" and it never checks. Only `https://` is
   accepted, or `http://` to `127.0.0.1` or `localhost` for local tests.

3. Build Windows with the same feed (see [Windows](#windows)), then publish both
   platforms in one GitHub release (see [Publishing a release](#publishing-a-release)).

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
   (`tauri signer sign --app-version`). With a feed, writes `latest.json` (see
   [The update feed](#the-update-feed) for the archive URL it uses).
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

## Windows

### One-time setup (Windows)

1. **Build tools**: as in [DEVELOPMENT.md](DEVELOPMENT.md#windows), with the
   [Vulkan SDK](https://vulkan.lunarg.com/) (`VULKAN_SDK`).
2. **Updater signing key.** The same key pair as the macOS release: the Windows update is
   signed with the private key whose public key is in `tauri.conf.json`. Copy the private
   key file from the Mac (`~/.tauri/shrimp-updater.key`) to the Windows machine over a
   channel you trust, keep it outside the repository, and give the build its path and
   password:

   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\.tauri\shrimp-updater.key"
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = '<the key password>'
   ```

   Never generate a new key pair for a release: installed copies only accept updates
   signed with the existing one.

3. **Code signing (optional).** `SHRIMP_WINDOWS_SIGN_THUMBPRINT` names a code-signing
   certificate in the certificate store, and `SHRIMP_WINDOWS_TIMESTAMP_URL` its
   timestamp server (default DigiCert). Without it the installer and the app are
   unsigned, and SmartScreen warns on download.

### Cutting a Windows release

```powershell
$env:SHRIMP_UPDATE_FEED = 'https://github.com/WHEREISDAN/shrimp/releases/latest/download/latest.json'
scripts/windows/release.ps1            # or: pnpm release:win
```

The script checks the Vulkan SDK, the updater key and its password, and the feed (the same
rules as on macOS), then builds with Whisper on the GPU (`--features vulkan`) and a
portable CPU baseline into `C:\st` (MSBuild fails on longer paths; `CARGO_TARGET_DIR`
overrides it). Tauri signs every binary through `scripts/windows/sign.ps1`. The update is
the installer itself: the script copies it to `bundle\updater\`, signs it with the
updater key, bound to the version (`tauri signer sign --app-version`), and with a feed
writes `latest.json` for `windows-x86_64`. The key is kept out of the Tauri build.

`-Version X.Y.Z` builds another version without editing the version files, and
`-Config <file>` merges a JSON file into the configuration last (test builds).

`-NoUpdaterKey` builds the installer only, for when the updater key stays on the Mac. Sign
the installer there as the update, bound to the version, with the key's password in
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and write its `windows-x86_64` entry into
`latest.json` as in [The update feed](#the-update-feed):

```sh
pnpm tauri signer sign --private-key-path ~/.tauri/shrimp-updater.key \
  --app-version <version> Shrimp_<version>_x64-setup.exe
```

| File (under `$env:CARGO_TARGET_DIR\release\bundle`) | What                               |
| --------------------------------------------------- | ---------------------------------- |
| `nsis\Shrimp_<version>_x64-setup.exe`               | The installer, for new installs    |
| `updater\Shrimp_<version>_x64-setup.exe`            | The same file, as the update       |
| `updater\Shrimp_<version>_x64-setup.exe.sig`        | Its minisign signature             |
| `updater\latest.json`                               | The feed for Windows (with a feed) |

## Publishing a release

Both platforms ship the same version in one GitHub release tagged `v<version>`. Each
build writes a `latest.json` with its own platform only; the release carries one file that
merges them.

1. Merge the two feeds. With the Mac's file as `mac.json` and the Windows file as
   `windows.json`:

   ```sh
   jq -s '.[0] + {platforms: (.[0].platforms + .[1].platforms)}' mac.json windows.json > latest.json
   ```

   Check that both files name the same `version`, and that the result has both
   `darwin-aarch64` and `windows-x86_64` entries.

2. Create the release with every file attached, `latest.json` included:

   ```sh
   gh release create v0.1.2 --title "Shrimp 0.1.2" --notes-file notes.md \
     Shrimp_0.1.2_aarch64.dmg \
     Shrimp_0.1.2_aarch64.app.tar.gz Shrimp_0.1.2_aarch64.app.tar.gz.sig \
     Shrimp_0.1.2_x64-setup.exe Shrimp_0.1.2_x64-setup.exe.sig \
     latest.json
   ```

   The Windows installer is both the download for new installs and the update. Attaching
   the `.sig` files is optional (their content is in `latest.json`), but it lets anyone
   check the signatures.

3. Make sure the release is marked as the latest one. The feed URL,
   `…/releases/latest/download/latest.json`, always serves the `latest.json` of the latest
   release, so publishing a release publishes its update. Don't mark a pre-release as
   latest.

### How a Windows update installs

The updater downloads the installer and checks its signature like on macOS. Restart to
update runs it in passive mode (a progress window, no questions) with `/UPDATE`, and
Tauri ends Shrimp right away; Shrimp first unloads Whisper and clears its crash marker,
which a normal quit does at `RunEvent::Exit`. The installer replaces the files in
`%LOCALAPPDATA%\Shrimp` and starts the new version with the arguments the old one had.
An update keeps the user's launch-at-login choice (`src-tauri/windows/hooks.nsh`).

## The update feed

`latest.json` follows tauri-plugin-updater's static format. The merged file for a release
looks like this:

```json
{
  "version": "0.1.2",
  "pub_date": "2026-10-15T12:00:00.000Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<content of Shrimp_0.1.2_aarch64.app.tar.gz.sig>",
      "url": "https://github.com/WHEREISDAN/shrimp/releases/download/v0.1.2/Shrimp_0.1.2_aarch64.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<content of Shrimp_0.1.2_x64-setup.exe.sig>",
      "url": "https://github.com/WHEREISDAN/shrimp/releases/download/v0.1.2/Shrimp_0.1.2_x64-setup.exe"
    }
  }
}
```

The release scripts write each `url`. For a GitHub Releases feed
(`https://github.com/<owner>/<repo>/releases/latest/download/latest.json`) it points into
the release tagged `v<version>`, so an installed copy downloads the exact files the feed
announced even after a newer release becomes the latest. For any other feed URL the update
file is expected next to `latest.json`.

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

### Testing an update on Windows

With a throwaway key pair, never committed, and a loopback feed:

1. In a scratch folder outside the repository, run
   `node_modules\.bin\tauri.cmd signer generate --ci -p <password> -w <folder>\test.key`,
   and write its public key into a file such as
   `{"plugins":{"updater":{"pubkey":"<content of test.key.pub>"}}}`.
2. Set `TAURI_SIGNING_PRIVATE_KEY` to `test.key`, its password, and
   `SHRIMP_UPDATE_FEED=http://127.0.0.1:8765/latest.json`; build the older version with
   `release.ps1 -Config <file>` and keep its installer, then the newer one with
   `-Version 0.1.1 -Config <file>`.
3. Serve the newer `updater\` folder:
   `python -m http.server 8765 --bind 127.0.0.1 --directory <folder>`.
4. Install the older version (`Shrimp_0.1.0_x64-setup.exe /S`) and start it. A fresh
   install downloads the speech model at launch; hard-linking the dev app's files from
   `%APPDATA%\com.shrimp.desktop.dev\models` into `%APPDATA%\com.shrimp.desktop\models`
   first avoids that.
5. A minute after launch Shrimp finds 0.1.1, downloads it and shows "Update ready,
   restart now?"; Restart now installs it and the new version opens its main window.
6. Uninstall (`%LOCALAPPDATA%\Shrimp\uninstall.exe /S`), stop the server, and delete
   the key pair, the builds, `%APPDATA%\com.shrimp.desktop` and
   `%LOCALAPPDATA%\com.shrimp.desktop`.

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

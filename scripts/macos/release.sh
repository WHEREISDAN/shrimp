#!/usr/bin/env bash
# Builds the macOS release for Apple Silicon: Shrimp.app signed with Developer ID
# (hardened runtime), notarized and stapled; the DMG, signed, notarized and
# stapled; and the updater archive signed with the updater key, with its
# latest.json when a feed is set. See docs/RELEASING.md.
#
# Usage: scripts/macos/release.sh [--version X.Y.Z] [--skip-notarize]
#
#   --version        Build this version instead of the one in tauri.conf.json
#                    (test builds; a real release bumps the version files).
#   --skip-notarize  Sign only. Gatekeeper rejects the result on other Macs.
#
# Environment:
#   SHRIMP_NOTARY_PROFILE        notarytool keychain profile (default: shrimp-notary)
#   SHRIMP_SIGNING_IDENTITY      codesign identity (default: the first
#                                "Developer ID Application" identity found)
#   SHRIMP_UPDATE_FEED           URL of latest.json, built into the app; the
#                                archive is expected next to it. Unset: the
#                                app says updates aren't configured.
#   SHRIMP_UPDATER_KEY           updater private key (default: ~/.tauri/shrimp-updater.key)
#   SHRIMP_UPDATER_KEY_SERVICE   keychain item holding its password
#                                (default: shrimp-updater-key)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NOTARY_PROFILE="${SHRIMP_NOTARY_PROFILE:-shrimp-notary}"
UPDATER_KEY="${SHRIMP_UPDATER_KEY:-$HOME/.tauri/shrimp-updater.key}"
UPDATER_KEY_SERVICE="${SHRIMP_UPDATER_KEY_SERVICE:-shrimp-updater-key}"
FEED="${SHRIMP_UPDATE_FEED:-}"
IDENTITY="${SHRIMP_SIGNING_IDENTITY:-}"
BUNDLE="$ROOT/src-tauri/target/release/bundle"
APP="$BUNDLE/macos/Shrimp.app"
TAURI="$ROOT/node_modules/.bin/tauri"

VERSION=""
NOTARIZE=1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --skip-notarize) NOTARIZE=0; shift ;;
    *) echo "usage: $0 [--version X.Y.Z] [--skip-notarize]" >&2; exit 2 ;;
  esac
done

fail() { echo "error: $*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$*"; }

preflight() {
  [[ "$(uname -m)" == arm64 ]] || fail "build on an Apple Silicon Mac"
  if [[ -z "$IDENTITY" ]]; then
    IDENTITY="$(security find-identity -v -p codesigning |
      sed -n 's/.*"\(Developer ID Application: .*\)"/\1/p' | head -n 1)"
  fi
  [[ -n "$IDENTITY" ]] || fail "no \"Developer ID Application\" identity in the keychain"
  if [[ "$NOTARIZE" == 1 ]] &&
    ! xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
    cat >&2 <<EOF
error: no working notarytool keychain profile "$NOTARY_PROFILE". Store one once:

  xcrun notarytool store-credentials "$NOTARY_PROFILE" \\
    --key /path/to/AuthKey_<KEY_ID>.p8 --key-id <KEY_ID> --issuer <ISSUER_ID>

or, with an Apple ID and an app-specific password:

  xcrun notarytool store-credentials "$NOTARY_PROFILE" \\
    --apple-id <APPLE_ID> --team-id <TEAM_ID> --password <APP_SPECIFIC_PASSWORD>
EOF
    exit 1
  fi
  [[ -f "$UPDATER_KEY" ]] || fail "updater key $UPDATER_KEY is missing (docs/RELEASING.md)"
  security find-generic-password -a "$USER" -s "$UPDATER_KEY_SERVICE" >/dev/null 2>&1 ||
    fail "no keychain item \"$UPDATER_KEY_SERVICE\" with the updater key's password"
  if [[ -n "$FEED" ]]; then
    case "$FEED" in
      https://*) ;;
      http://127.0.0.1[:/]* | http://localhost[:/]*) ;;
      *) fail "SHRIMP_UPDATE_FEED must be https (http only for 127.0.0.1 or localhost)" ;;
    esac
  fi
  [[ -n "$VERSION" ]] || VERSION="$(node -p "require('$ROOT/src-tauri/tauri.conf.json').version")"
}

# The build-time configuration: the version and the update feed.
release_config() {
  FEED="$FEED" VERSION="$VERSION" node -e '
    const { FEED, VERSION } = process.env;
    const updater = FEED
      ? { endpoints: [FEED], dangerousInsecureTransportProtocol: FEED.startsWith("http:") }
      : {};
    process.stdout.write(JSON.stringify({ version: VERSION, plugins: { updater } }));'
}

build_app() {
  step "Building Shrimp $VERSION (signed with $IDENTITY)"
  # Tauri notarizes by itself when these are set; this script does it instead.
  unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH
  (cd "$ROOT" && APPLE_SIGNING_IDENTITY="$IDENTITY" pnpm tauri build --bundles app \
    --config "$(release_config)")
  codesign --verify --deep --strict "$APP"
  [[ "$(codesign -d --verbose=2 "$APP" 2>&1)" == *"flags=0x10000(runtime)"* ]] ||
    fail "Shrimp.app is not signed with the hardened runtime"
  codesign -d -r- "$APP" 2>&1 | grep designated
}

# Submits a zip or DMG and waits; prints Apple's log when it is rejected.
notarize() {
  local file="$1" result id status
  step "Notarizing $(basename "$file")"
  result="$(xcrun notarytool submit "$file" --keychain-profile "$NOTARY_PROFILE" \
    --wait --output-format json)"
  id="$(node -p 'JSON.parse(process.argv[1]).id' "$result")"
  status="$(node -p 'JSON.parse(process.argv[1]).status' "$result")"
  echo "submission $id: $status"
  if [[ "$status" != Accepted ]]; then
    xcrun notarytool log "$id" --keychain-profile "$NOTARY_PROFILE" >&2 || true
    fail "notarization of $(basename "$file") was $status"
  fi
}

notarize_app() {
  local zip
  zip="$(mktemp -d)/Shrimp.zip"
  ditto -c -k --keepParent "$APP" "$zip"
  notarize "$zip"
  rm -f "$zip"
  rmdir "$(dirname "$zip")"
  xcrun stapler staple "$APP"
}

# The archive the updater downloads, its signature and latest.json.
updater_artifacts() {
  local dir="$BUNDLE/updater" archive password
  archive="Shrimp_${VERSION}_aarch64.app.tar.gz"
  step "Signing the updater archive $archive"
  rm -rf "$dir"
  mkdir -p "$dir"
  COPYFILE_DISABLE=1 tar -czf "$dir/$archive" --no-mac-metadata --no-xattrs \
    -C "$(dirname "$APP")" Shrimp.app
  password="$(security find-generic-password -a "$USER" -s "$UPDATER_KEY_SERVICE" -w)"
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$password" "$TAURI" signer sign \
    --private-key-path "$UPDATER_KEY" --app-version "$VERSION" "$dir/$archive" >/dev/null
  [[ -n "$FEED" ]] || { echo "SHRIMP_UPDATE_FEED is unset: no latest.json"; return; }
  ARCHIVE="$dir/$archive" URL="${FEED%/*}/$archive" VERSION="$VERSION" node -e '
    const fs = require("fs");
    const { ARCHIVE, URL, VERSION } = process.env;
    const signature = fs.readFileSync(`${ARCHIVE}.sig`, "utf8").trim();
    const latest = {
      version: VERSION,
      pub_date: new Date().toISOString(),
      platforms: { "darwin-aarch64": { signature, url: URL } },
    };
    fs.writeFileSync(`${require("path").dirname(ARCHIVE)}/latest.json`,
      JSON.stringify(latest, null, 2) + "\n");'
}

build_dmg() {
  local dir="$BUNDLE/dmg" stage
  DMG="$dir/Shrimp_${VERSION}_aarch64.dmg"
  step "Building $(basename "$DMG")"
  mkdir -p "$dir"
  stage="$(mktemp -d)"
  chmod 755 "$stage"
  ditto "$APP" "$stage/Shrimp.app"
  ln -s /Applications "$stage/Applications"
  hdiutil create -volname Shrimp -srcfolder "$stage" -fs HFS+ -format ULFO -ov "$DMG" >/dev/null
  rm -rf "$stage"
  codesign --force --sign "$IDENTITY" --timestamp "$DMG"
}

preflight
build_app
[[ "$NOTARIZE" == 1 ]] && notarize_app
updater_artifacts
build_dmg
if [[ "$NOTARIZE" == 1 ]]; then
  notarize "$DMG"
  xcrun stapler staple "$DMG"
  spctl -a -t open --context context:primary-signature -vv "$DMG"
fi

step "Done"
echo "App:     $APP"
echo "DMG:     $DMG"
echo "Updater: $BUNDLE/updater"
[[ -n "$FEED" ]] && echo "Feed:    $FEED" || echo "Feed:    none (updates aren't configured)"

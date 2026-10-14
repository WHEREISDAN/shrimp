#!/usr/bin/env bash
# The macOS regression walkthrough (AUD-02). Builds the test helper and the
# target app, saves the dev app's data and the clipboard, runs every case
# against the signed "Shrimp Dev" app, then puts everything back.
#
# Usage: scripts/macos/regression/run.sh [latency] [--build] [--only <ids or suites>] [--report <file>]
#   latency   measure key-release-to-insert latency (AUD-03) instead; takes --runs <n> and --model <name>
#   --build   rebuild Shrimp Dev.app first (otherwise it's built only if missing)
# See docs/DEVELOPMENT.md, "Regression walkthrough".
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
DEV_APP="$REPO/scripts/macos/dev-app.sh"
BUNDLE="$REPO/src-tauri/target/debug/bundle/macos/Shrimp Dev.app"
DATA="$HOME/Library/Application Support/com.shrimp.desktop.dev"
TARGET_ID="com.shrimp.regression.target"

build=0
script="walkthrough.mjs"
args=()
while (($#)); do
  case "$1" in
    --build) build=1 ;;
    latency) script="latency.mjs" ;;
    *) args+=("$1") ;;
  esac
  shift
done

WORK="$(mktemp -d -t shrimp-regression)"
HELPER="$WORK/shrimp-helper"
TARGET_APP="$WORK/Shrimp Regression Target.app"

restore() {
  local status=$?
  set +e
  [[ -x "$HELPER" ]] && "$HELPER" notify "$TARGET_ID" quit
  "$DEV_APP" quit
  sleep 2
  if [[ -d "$WORK/data" ]]; then
    for entry in "$DATA"/* "$DATA"/.[!.]*; do
      [[ -e "$entry" && "$(basename "$entry")" != "models" ]] && rm -rf "$entry"
    done
    cp -Rp "$WORK/data/." "$DATA/"
    echo "Restored the dev app's data."
  fi
  if [[ -f "$WORK/clipboard.json" ]]; then
    "$HELPER" clipboard-restore "$WORK/clipboard.json" >/dev/null && echo "Restored the clipboard."
  fi
  rm -rf "$WORK"
  exit "$status"
}
trap restore EXIT

if pgrep -xq "Wispr Flow"; then
  echo "Warning: Wispr Flow is running and also reacts to fn; quit it for a clean run." >&2
fi

echo "Building the helper and the target app in ${WORK}…"
swiftc -O -o "$HELPER" "$HERE/helper.swift"
mkdir -p "$TARGET_APP/Contents/MacOS"
swiftc -O -o "$TARGET_APP/Contents/MacOS/target" "$HERE/target.swift"
cat >"$TARGET_APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>$TARGET_ID</string>
<key>CFBundleName</key><string>Shrimp Regression Target</string>
<key>CFBundleExecutable</key><string>target</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
codesign -s - --force "$TARGET_APP" >/dev/null 2>&1

if ((build)) || [[ ! -d "$BUNDLE" ]]; then
  "$DEV_APP" build
fi

"$DEV_APP" quit
sleep 2
mkdir -p "$WORK/data"
for entry in "$DATA"/* "$DATA"/.[!.]*; do
  [[ -e "$entry" && "$(basename "$entry")" != "models" ]] && cp -Rp "$entry" "$WORK/data/"
done
"$HELPER" clipboard-save "$WORK/clipboard.json" >/dev/null

token="/tmp/tauri-mcp.sock.token"
before=$(stat -f %m "$token" 2>/dev/null || echo 0)
"$DEV_APP" open
for _ in $(seq 1 60); do
  now=$(stat -f %m "$token" 2>/dev/null || echo 0)
  ((now > before)) && break
  sleep 1
done
sleep 3
open "$TARGET_APP"
sleep 2

node "$HERE/$script" --work "$WORK" "${args[@]+"${args[@]}"}"

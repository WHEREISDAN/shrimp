#!/usr/bin/env bash
# Builds and launches "Shrimp Dev.app": a debug build (tauri-plugin-mcp enabled)
# bundled as a real .app and signed with the stable "Shrimp Dev" identity, so
# macOS privacy grants survive rebuilds. See docs/DEVELOPMENT.md.
#
# Usage: scripts/macos/dev-app.sh build|open|quit|path
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
APP="$ROOT/src-tauri/target/debug/bundle/macos/Shrimp Dev.app"
BUNDLE_ID="com.shrimp.desktop.dev"

build() {
  "$ROOT/scripts/macos/dev-identity.sh" create >/dev/null
  (cd "$ROOT" && pnpm tauri build --debug --bundles app --config src-tauri/tauri.conf.dev.json)
  codesign --verify --strict "$APP"
  codesign -d -r- "$APP" 2>&1 | grep designated
}

quit() {
  osascript -e "tell application id \"$BUNDLE_ID\" to quit" >/dev/null 2>&1 || true
}

open_app() {
  [[ -d "$APP" ]] || build
  quit
  # LaunchServices makes the app its own responsible process for TCC, unlike
  # running the binary from a shell, which attributes requests to the terminal.
  open -n "$APP" "$@"
}

case "${1:-}" in
  build) build ;;
  open) shift; open_app "$@" ;;
  quit) quit ;;
  path) echo "$APP" ;;
  *) echo "usage: $0 build|open|quit|path" >&2; exit 2 ;;
esac

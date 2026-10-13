#!/usr/bin/env bash
# Manages the self-signed "Shrimp Dev" code-signing identity used for dev builds.
#
# macOS ties privacy grants (Microphone, Accessibility, Input Monitoring) to the
# app's designated requirement. Ad-hoc signatures change on every rebuild, so
# grants are lost; a stable certificate keeps them. The identity lives in its own
# keychain with a fixed password so signing never needs the login password or a
# GUI prompt.
#
# Usage: scripts/macos/dev-identity.sh create|unlock|status|delete
set -euo pipefail

IDENTITY="Shrimp Dev"
KEYCHAIN="$HOME/Library/Keychains/shrimp-dev.keychain-db"
KEYCHAIN_PASSWORD="shrimp-dev"

keychain_in_search_list() {
  security list-keychains -d user | grep -q "shrimp-dev.keychain-db"
}

add_to_search_list() {
  keychain_in_search_list && return
  local existing
  existing=$(security list-keychains -d user | sed -e 's/^ *"//' -e 's/"$//')
  # shellcheck disable=SC2086
  security list-keychains -d user -s "$KEYCHAIN" $existing
}

unlock() {
  security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
}

identity_exists() {
  [[ -f "$KEYCHAIN" ]] && security find-certificate -c "$IDENTITY" "$KEYCHAIN" >/dev/null 2>&1
}

create() {
  if identity_exists; then
    unlock
    add_to_search_list
    echo "Identity \"$IDENTITY\" already exists in $KEYCHAIN"
    return
  fi

  local work
  work=$(mktemp -d)
  trap 'rm -rf "$work"' RETURN

  openssl req -x509 -newkey rsa:2048 -sha256 -days 3650 -nodes \
    -keyout "$work/key.pem" -out "$work/cert.pem" \
    -subj "/CN=$IDENTITY" \
    -addext "basicConstraints=critical,CA:false" \
    -addext "keyUsage=critical,digitalSignature" \
    -addext "extendedKeyUsage=critical,codeSigning" 2>/dev/null

  # macOS `security import` only reads PKCS#12 files that use legacy algorithms.
  openssl pkcs12 -export -legacy -name "$IDENTITY" \
    -inkey "$work/key.pem" -in "$work/cert.pem" \
    -out "$work/identity.p12" -passout pass:"$KEYCHAIN_PASSWORD"

  [[ -f "$KEYCHAIN" ]] || security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
  security set-keychain-settings "$KEYCHAIN"
  unlock
  security import "$work/identity.p12" -k "$KEYCHAIN" -P "$KEYCHAIN_PASSWORD" \
    -T /usr/bin/codesign -T /usr/bin/security
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s \
    -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
  add_to_search_list
  echo "Created identity \"$IDENTITY\" in $KEYCHAIN"
}

status() {
  if ! identity_exists; then
    echo "missing"
    return 1
  fi
  security find-certificate -c "$IDENTITY" -Z "$KEYCHAIN" | grep "SHA-1"
  keychain_in_search_list && echo "keychain in search list" || echo "keychain NOT in search list"
}

delete() {
  [[ -f "$KEYCHAIN" ]] && security delete-keychain "$KEYCHAIN"
  echo "Deleted $KEYCHAIN"
}

case "${1:-}" in
  create) create ;;
  unlock) unlock ;;
  status) status ;;
  delete) delete ;;
  *) echo "usage: $0 create|unlock|status|delete" >&2; exit 2 ;;
esac

# Security policy

## Reporting a vulnerability

Please report security problems privately through GitHub's
[private vulnerability reporting](https://github.com/WHEREISDAN/shrimp/security/advisories/new)
(the repository's Security tab, then "Report a vulnerability"). Don't open a public issue
for them.

Include the Shrimp version, your operating system and the steps that show the problem.
Shrimp is a personal project maintained in spare time, so a reply may take a while, but
security reports are read before anything else.

## Supported versions

Only the latest release gets fixes. Shrimp updates itself from GitHub Releases, so most
installs move to a fixed version on their own.

## Scope

Shrimp reads the microphone, the text around the cursor and the clipboard, and stores API
keys in the operating system's keychain (macOS Keychain or Windows Credential Manager).
Problems that expose any of these, or the dictation history, to other apps or to the
network are in scope. Releases are signed: the macOS app with a Developer ID and
notarized, updates on both platforms with the updater key whose public half is in
`src-tauri/tauri.conf.json`. The Windows installer itself is not code-signed.

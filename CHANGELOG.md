# Changelog

All notable changes to Shrimp. Versions follow [Semantic Versioning](https://semver.org).

## 0.1.2 (2026-10-15)

The first public release.

- Updates come from GitHub Releases. Shrimp checks
  `https://github.com/WHEREISDAN/shrimp/releases/latest/download/latest.json` a minute
  after launch and every six hours; "Check for updates…" in the tray checks right away.
- Shrimp is released under the MIT license. The app bundle now includes `LICENSE` and
  `THIRD_PARTY_NOTICES.md`, the licenses of the open-source software it ships.
- Test fixtures and examples use generic sample names.

0.1.0 and 0.1.1 were built without an update feed, so they can't update themselves.
Install 0.1.2 from the DMG or the Windows installer once; later versions arrive through
the in-app updater.

## 0.1.1 (2026-10-14)

- Every screen redesigned in light and dark, with the title bar as part of the window.
- A new waveform shrimp logo, app icons and tray icons.
- The Flow Bar is a calm dark capsule.
- Windows: the main window resizes from every edge and corner, the title area opens the
  window menu, Windows 10 gets a shadow and a hairline instead of the frame line, and the
  tray icon is drawn at the taskbar's own size.

## 0.1.0 (2026-10-13)

The first version, never published.

- Push-to-talk and hands-free dictation into any app, transcribed locally with Whisper
  (Metal on Apple Silicon, Vulkan or the CPU on Windows).
- Optional AI edits through Anthropic, OpenAI, Groq or a local Ollama: filler removal,
  self-corrections, lists, punctuation, app-aware styles, formatting per app, a personal
  dictionary and snippets. Optional cloud transcription through OpenAI or Groq.
- Command Mode, transforms, context awareness and coding features.
- History, insights, languages, microphone handling, sounds, media control, onboarding
  and accessibility.
- A notarized DMG for macOS, a per-user installer for Windows and in-app updates.

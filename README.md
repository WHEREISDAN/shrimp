# Shrimp

Speech to text, anywhere. Shrimp is a desktop dictation app in the style of
[Wispr Flow](https://wisprflow.ai): hold a hotkey, speak, and polished text appears at the
cursor in any app.

- **Private by default.** Everything stays on the device. Speech-to-text runs locally with
  Whisper (Metal-accelerated on Apple Silicon). Cloud engines are optional and use your
  own API key.
- **AI edits.** Filler removal, punctuation, lists, course correction and app-aware styles,
  through the LLM provider you choose: Anthropic, OpenAI, Groq or a local Ollama. API keys
  live in the OS keychain.
- **Platforms.** macOS first, then Windows.

Shrimp is built with [Tauri v2](https://tauri.app), Rust, React and TypeScript.

## Status

Phase 1 (the core dictation loop) is complete on macOS: hold fn to dictate (or fn Space and
double-tap for hands-free), local Whisper transcription with a downloaded model, pasting
at the cursor with the clipboard restored, the Flow Bar with live waveform and notices, the
tray menu and dictation sounds.

Phase 2 adds AI edits: filler removal, self-corrections, lists, spoken punctuation and line
breaks, Auto Cleanup levels, app-aware styles (personal, work, email, other), a personal
dictionary that also biases Whisper, snippets with rich text, and CSV/JSON imports. The
cleanup runs on Anthropic, OpenAI, Groq or a local Ollama with keys in the keychain, and
falls back to the raw transcript whenever the model fails or answers instead of editing.
OpenAI and Groq can also transcribe. [docs/PARITY.md](docs/PARITY.md) tracks every feature.

## Getting started

```sh
pnpm install
pnpm tauri:dev      # hot-reload development
pnpm app:dev:open   # signed dev app bundle, for anything that needs macOS permissions
pnpm check          # formatting, lint, type check and all tests
```

Prerequisites, signing, testing with WAV files and driving the app with tauri-mcp are
covered in [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

## Documentation

- [docs/PARITY.md](docs/PARITY.md): feature checklist against Wispr Flow, by phase.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): modules, data flow, platform layer,
  storage and provider traits.
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md): setup, commands and testing.

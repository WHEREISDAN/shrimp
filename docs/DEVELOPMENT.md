# Development

How to set up, run, test and drive Shrimp during development. Architecture lives in
[ARCHITECTURE.md](ARCHITECTURE.md); feature progress lives in [PARITY.md](PARITY.md).

## Prerequisites

- macOS 14.2 or later on Apple Silicon (the first platform). Windows arrives in phase 5.
- Xcode with its command line tools, and CMake (`brew install cmake`). whisper.cpp is
  built from source by `whisper-rs` from phase 1.
- Rust through rustup. `rust-toolchain.toml` pins Rust 1.99.0 (Tauri 2.12 needs 1.90 or
  later). rustup installs it on the first `cargo` run, or run
  `rustup toolchain install 1.99.0`. The pin does not change your global default.
- Node 22 or later and pnpm 10.

```sh
pnpm install
```

## Commands

| Command                         | What it does                                                                           |
| ------------------------------- | -------------------------------------------------------------------------------------- |
| `pnpm tauri:dev`                | Hot-reload development: Vite on port 1420 plus the debug binary.                       |
| `pnpm app:dev`                  | Builds the signed debug app bundle `Shrimp Dev.app`.                                   |
| `pnpm app:dev:open`             | Quits any running `Shrimp Dev`, then launches the bundle (builds it first if missing). |
| `scripts/macos/dev-app.sh quit` | Quits the running `Shrimp Dev` app.                                                    |
| `pnpm check`                    | Every check: formatting, lint, type check, JS tests, clippy and Rust tests.            |
| `pnpm format`                   | Formats JS/TS/JSON/CSS/Markdown (Prettier) and Rust (rustfmt).                         |
| `pnpm lint`                     | oxlint, with warnings treated as errors.                                               |
| `pnpm typecheck`                | `tsc -b` over the app and the Vite config.                                             |
| `pnpm test`                     | Vitest (jsdom, Testing Library, Tauri IPC mocks).                                      |
| `pnpm clippy`                   | `cargo clippy --all-targets -- -D warnings`.                                           |
| `pnpm test:rust`                | `cargo test` (unit tests and `src-tauri/tests/`).                                      |

## Two ways to run the app

Both run a debug build with the identifier `com.shrimp.desktop.dev` and the name
"Shrimp Dev", so they never touch the data, settings or keychain entries of a release
install.

### Hot reload: `pnpm tauri:dev`

Use this for UI and Rust work that needs no macOS privacy permission. It runs the raw
binary from `src-tauri/target/debug`, not an app bundle. macOS then attributes privacy
requests to the process that launched it (Terminal, the IDE or the agent host). Microphone,
Accessibility and Input Monitoring prompts and grants go to that app instead of Shrimp.
Runs started by an agent often read as "denied" even when the person has granted access.

### Signed app bundle: `pnpm app:dev` and `pnpm app:dev:open`

Use this for anything that needs a privacy permission: microphone capture, the global
hotkey, pasting into other apps, or reading text through accessibility.

`scripts/macos/dev-app.sh` does the following:

1. Makes sure the "Shrimp Dev" signing identity exists and its keychain is unlocked.
2. Runs `tauri build --debug --bundles app --config src-tauri/tauri.conf.dev.json`. This
   gives a debug build, so the MCP bridge is compiled in. The frontend is built in
   development mode, and the bundle is signed with "Shrimp Dev" (hardened runtime, with
   the entitlements in `src-tauri/entitlements.plist`).
3. Launches it with `open`, through LaunchServices. The app becomes its own responsible
   process (its parent is `launchd`), so privacy grants are recorded for
   `com.shrimp.desktop.dev`.

The bundle is at `src-tauri/target/debug/bundle/macos/Shrimp Dev.app`. Pass extra `open`
arguments after `open`, for example environment variables:

```sh
scripts/macos/dev-app.sh open --env SHRIMP_AUDIO_SOURCE=wav:/absolute/path/speech.wav
```

## Stable code-signing identity

macOS ties privacy grants to the app's designated requirement. An ad-hoc signature changes
on every rebuild, so grants silently stop applying. Dev bundles are therefore signed with a
self-signed certificate that never changes:

```text
designated => identifier "com.shrimp.desktop.dev" and certificate leaf = H"38e13c06dbf349f3a3cc1bf3f72a64a6b864b382"
```

The hash above belongs to the identity created on the owner's Mac. Another machine
creates its own identity, with its own hash.

`scripts/macos/dev-identity.sh` manages the identity:

| Subcommand | Effect                                                                                                            |
| ---------- | ----------------------------------------------------------------------------------------------------------------- |
| `create`   | Creates the identity if missing (idempotent), unlocks its keychain and adds the keychain to the user search list. |
| `status`   | Prints the certificate hash and whether the keychain is in the search list.                                       |
| `unlock`   | Unlocks the keychain, for example after a reboot.                                                                 |
| `delete`   | Deletes the keychain and the identity.                                                                            |

Details:

- The certificate is "Shrimp Dev": RSA 2048, code-signing extended key usage, valid for
  10 years.
- It lives in its own keychain, `~/Library/Keychains/shrimp-dev.keychain-db`, with the fixed
  password `shrimp-dev`. A separate keychain makes setup fully non-interactive. Signing
  from the login keychain would need the login password once (for
  `security set-key-partition-list`) or a GUI prompt on every signing.
- The certificate is not marked as trusted, and it does not need to be. `codesign` signs
  with it, and the designated requirement pins the certificate hash, which is what privacy
  grants match. `security find-identity -v` therefore lists it as
  `CSSMERR_TP_NOT_TRUSTED`, which is expected.
- Recreating the identity (`delete`, then `create`) produces a new certificate hash. Every
  privacy grant must then be given again.
- Reset the grants for the dev app with `tccutil reset All com.shrimp.desktop.dev`.

**Security note.** Any process running as your user can sign code with this identity and
inherit the privacy grants given to "Shrimp Dev". This exposure is the same as for any
unlocked development certificate. Do not grant "Shrimp Dev" permissions on a shared
machine, and run `scripts/macos/dev-identity.sh delete` when you no longer need it.
Release builds use a Developer ID identity (phase 5), never this one.

### One-time grants (from phase 1)

The owner gives each permission to "Shrimp Dev" once. The grants then survive rebuilds for
as long as the identity and the bundle identifier stay the same.

- **Microphone:** macOS asks on the first recording. Click Allow.
- **Accessibility** and **Input Monitoring:** System Settings > Privacy & Security, then
  turn on "Shrimp Dev". An agent cannot do this, because macOS protects these panes.

## Driving the app with tauri-mcp

Debug builds register [tauri-plugin-mcp](https://github.com/P3GLEG/tauri-plugin-mcp)
(pinned to commit `c7d271a`, version 0.3.1). It lets an agent read the DOM, click, type,
take screenshots, call IPC commands and read logs in the real app.

- `.mcp.json` registers the MCP server `tauri-mcp`, which runs
  `npx -y tauri-plugin-mcp-server@0.3.1`.
- On the Rust side, `src-tauri/src/app/dev_bridge.rs` registers the plugin only under
  `cfg(debug_assertions)`. It listens on `/tmp/tauri-mcp.sock`, which is where the MCP
  server looks by default; the plugin writes an auth token to `/tmp/tauri-mcp.sock.token`.
- The frontend loads the plugin's guest listeners only when `VITE_MCP_BRIDGE=1`. This
  value comes from `.env.development`, so only development-mode builds include the bridge.
- Release builds include none of it. The plugin is not registered, the plugin itself refuses
  to open its socket in release builds, and the frontend drops the import.

A typical session:

1. Start the app: `pnpm app:dev:open` (or `pnpm tauri:dev` when no permission is involved).
2. Wait for `/tmp/tauri-mcp.sock` to appear.
3. Call `query_page` with `mode: "app_info"`. The app name should be "Shrimp Dev".
4. Use `read_text`, `query_page` with `mode: "map"`, `click` and `type_text` for the UI,
   `manage_ipc` with `action: "invoke"` for backend commands, `take_screenshot` for
   visuals, and `query_logs` for logs.
5. Quit the app (`scripts/macos/dev-app.sh quit`) or stop `pnpm tauri:dev` when finished.

Things to know:

- The first JS-backed call right after launch can time out while the webview is still
  loading. Retry it, or use `wait_for`.
- Rust `log` records are captured into the plugin's buffer in debug builds
  (`capture_rust_logs`). They appear in `query_logs`, not in the terminal. JS `console.*`
  calls are captured too.
- Only one app can own the socket. Spaced dev builds use the same default path. To run
  both, start one app with `TAURI_MCP_IPC_PATH=/tmp/tauri-mcp-shrimp.sock` and give the
  MCP server the same variable. Both sides honour it.
- `restart_app` refuses to work under `tauri dev`. Relaunch with `dev-app.sh open` instead.
- Register every new IPC command in `commands::NAMES` too. `manage_ipc` with
  `action: "commands"` lists them for agents.

## Audio from a WAV file instead of the microphone

The STT pipeline does not depend on the microphone. Every input implements
`audio::AudioSource`:

- `start(sink)` begins pushing interleaved f32 samples into an `AudioSink` from the
  source's own thread or callback, and returns the format.
- `stop()` ends delivery.

There are two implementations:

| Source                     | Used for                                                                                                          |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Microphone (cpal, phase 1) | Normal use.                                                                                                       |
| `WavFileSource`            | Tests (`Pacing::Instant`) and dev builds (`Pacing::Realtime`, one 20 ms chunk per 20 ms, like a live microphone). |

`audio::Recorder` collects whatever a source delivers. It converts the result into an
`AudioClip` (16 kHz mono f32, resampled with rubato's FFT resampler), which is what
`stt::SttEngine::transcribe` takes. Nothing after the recorder knows where the audio came
from.

### In tests

```rust
let source = WavFileSource::new(path, Pacing::Instant);
let mut recorder = Recorder::start(Box::new(source))?;
recorder.wait_until_ended(Duration::from_secs(10))?;
let clip = recorder.finish()?;
let transcript = engine.transcribe(&clip, &SttOptions::default()).await?;
```

`src-tauri/tests/wav_pipeline.rs` drives the fixture
`src-tauri/tests/fixtures/audio/hello-shrimp.wav` through this path. The fixture is 3.69 s
of 16 kHz mono speech of "Hello from Shrimp. This sentence is a test of local dictation.",
generated with macOS `say`. Make more fixtures the same way:

```sh
say -o speech.wav --data-format=LEI16@16000 "Text to speak"
```

### In the dev app

Debug builds read `SHRIMP_AUDIO_SOURCE`, which `audio::AudioSourceSpec` parses:

| Value          | Source                                            |
| -------------- | ------------------------------------------------- |
| `mic`          | Default microphone.                               |
| `mic:<device>` | Microphone by device name.                        |
| `wav:<path>`   | WAV file at real-time pace. Use an absolute path. |

Release builds ignore the variable. Phase 1 connects it to the dictation controller. A
hotkey press then records from the file instead of the microphone, until the file ends or
the hotkey is released. Phase 1 also adds a debug-only IPC command, so an agent can switch
sources through `manage_ipc` without relaunching the app.

A WAV source needs no microphone permission. Everything from recording to transcription
and AI edits can therefore be tested without privacy grants. Only the hotkey and the paste
into another app still need them.

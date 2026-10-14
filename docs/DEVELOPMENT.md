# Development

How to set up, run, test and drive Shrimp during development. Architecture lives in
[ARCHITECTURE.md](ARCHITECTURE.md); feature progress lives in [PARITY.md](PARITY.md);
release builds and updates live in [RELEASING.md](RELEASING.md).

## Prerequisites

- macOS 14.2 or later on Apple Silicon, or Windows 10 21H2 or later (x64).
- macOS: Xcode with its command line tools, and CMake (`brew install cmake`). whisper.cpp
  is built from source by `whisper-rs`.
- Windows: Visual Studio 2022 Build Tools with "Desktop development with C++" (MSVC and
  the Windows SDK), CMake, and LLVM for `whisper-rs`'s bindgen (`LIBCLANG_PATH` pointing
  at the folder with `libclang.dll`). WebView2 ships with Windows 10 and 11. The GPU build
  also needs the [Vulkan SDK](https://vulkan.lunarg.com/) (`VULKAN_SDK`); see
  [Windows](#windows).
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
| `pnpm app:dev:win`              | Windows: builds the debug app `shrimp.exe` (`scripts/windows/dev-app.ps1 build`).      |
| `pnpm app:dev:win:open`         | Windows: quits a running `Shrimp Dev`, then launches it (builds it first if missing).  |
| `pnpm release:win`              | Windows: the per-user NSIS installer with Whisper on the GPU (`release.ps1`).          |
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
Release builds use the Developer ID identity, never this one (see
[RELEASING.md](RELEASING.md)).

### One-time grants (from phase 1)

The owner gives each permission to "Shrimp Dev" once. The grants then survive rebuilds for
as long as the identity and the bundle identifier stay the same.

- **Microphone:** macOS asks on the first microphone dictation. Click Allow. A Mac without
  any input device shows "No microphone found" instead and never prompts.
- **Accessibility:** System Settings > Privacy & Security > Accessibility, then turn on
  "Shrimp Dev". The global hotkey (an active event tap), pasting and reading text all depend
  on it. An agent cannot do this, because macOS protects these panes.
- **Input Monitoring** is not needed: Accessibility already covers listening to and posting
  events. It would only matter if a listen-only event tap were added later.

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
- Every webview that agents should drive must call `setupPluginListeners()` (see
  `src/main.tsx`), including the Flow Bar window from phase 1, and must be listed in the
  capability that grants `mcp:default`.
- `take_screenshot` works on the signed bundle without a Screen Recording grant, but the
  plugin's macOS capture skips windows smaller than 100×100 points, which includes the Flow
  Bar. Capture it with `screencapture -R x,y,w,h`, using the bounds of the window titled
  "Shrimp Flow Bar" from `CGWindowListCopyWindowInfo` (the terminal needs Screen
  Recording). `query_page`, `read_text` and `click` with `window_label: "flow-bar"` work
  regardless.
- `execute_js` fails: the CSP has no `unsafe-eval`. Use `query_page`, `read_text` and
  `manage_ipc` instead.
- The plugin leaves `/tmp/tauri-mcp.sock` and its `.token` file behind when the app quits.
  The next launch replaces them, so they are harmless.
- The crates.io crate called `tauri-plugin-mcp` is an unrelated project. Shrimp depends on
  P3GLEG's plugin through git; the npm packages of the same name are P3GLEG's.

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

Release builds ignore the variable. A dictation then records from the file instead of the
microphone, until the file ends (which finishes the dictation as if the shortcut had been
released) or the shortcut is released.

Debug builds also register two commands, so an agent can switch sources through
`manage_ipc` without relaunching the app. The change applies from the next dictation.

| Command                | Arguments             | Returns                                                                   |
| ---------------------- | --------------------- | ------------------------------------------------------------------------- |
| `dev_get_audio_source` | none                  | The current source, e.g. `mic`.                                           |
| `dev_set_audio_source` | `{ "spec": "wav:…" }` | The new source; an error if the file is missing or the spec is malformed. |

`flow_bar_control` with `{ "control": "start" }` (then `"stop"` or `"cancel"`) starts a
hands-free dictation without any shortcut, which needs no Accessibility grant.

### Fake microphones

This Mac has no microphone, so debug builds can replace the devices with fakes at runtime.
`dev_set_microphones` takes the devices to report (a missing one is "unplugged", and a
source recording from it fails as a disconnect), the default input and whether the lid
is closed; `{"devices": null}` goes back to the real devices:

```json
{
  "devices": [
    {
      "id": "usb",
      "name": "Yeti",
      "kind": "wired",
      "behaviour": { "type": "wav", "path": "/abs/speech.wav" }
    },
    {
      "id": "pods",
      "name": "AirPods",
      "kind": "bluetooth",
      "behaviour": { "type": "tone", "amplitude": 0.2 }
    },
    { "id": "mute", "name": "Muted mic", "kind": "builtIn", "behaviour": { "type": "silent" } },
    {
      "id": "slow",
      "name": "Slow mic",
      "kind": "wired",
      "behaviour": { "type": "slowOpen", "millis": 9000 }
    }
  ],
  "default": "usb",
  "lidClosed": false
}
```

`kind` is `builtIn`, `wired`, `bluetooth` or `virtual`; `behaviour` is `tone`, `wav`
(looped), `silent`, `slowOpen` (then silence) or `dead` (opens, never delivers). A
dictation uses them when the audio source is `mic` (the default). Tests use the same fakes
through `FakeDevices`.

### Permissions, login, disk space, install location

Debug builds can pretend states that are hard to produce for real:

| Override                                              | Effect                                                                                                                                                                                                                                                                                                      |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dev_set_permissions` `{ "accessibility": "denied" }` | Reports a permission as `granted`, `denied` or `notDetermined` (`null` goes back to the real state); its request and settings pane do nothing. Real grants are never touched.                                                                                                                               |
| `SHRIMP_PRETEND_PERMISSIONS=accessibility=denied`     | The same from launch, so the check after an update runs with it (`microphone=notDetermined` too, comma-separated).                                                                                                                                                                                          |
| `SHRIMP_LAUNCHED_AT_LOGIN=1`                          | The launch counts as one from the login item: the main window stays hidden.                                                                                                                                                                                                                                 |
| `SHRIMP_FAKE_FREE_BYTES=52000000`                     | Reports that much free disk space, to show the "Storage Full" screen.                                                                                                                                                                                                                                       |
| `SHRIMP_PRETEND_OUTSIDE_APPLICATIONS=1`               | macOS: the app counts as running outside the Applications folders, so the main window shows "Move Shrimp to the Applications folder" and no shortcut dictates. Debug builds skip the real check, and their Move to Applications only logs what a release would copy where (the dev bundle is never copied). |

Set environment variables with `scripts/macos/dev-app.sh open --env NAME=value`. To test
onboarding, quit the app and move `~/Library/Application Support/com.shrimp.desktop.dev`
aside first (it starts on a fresh install), then put it back. The login item is visible
with `osascript -e 'tell application "System Events" to get the name of every login item'`;
don't use `sfltool dumpbtm`, which asks for an administrator password.

### Keyboards without an Fn key

Debug builds read `SHRIMP_FN_KEY`: `0` pretends no keyboard has Apple's Fn key, so the
defaults become Ctrl+Opt, Ctrl+Opt+Space and Cmd+Ctrl+Opt; `1` forces Fn. Without it,
IOKit decides (CORE-10).

Two more debug-only commands help with AI edits:

| Command                | Arguments                                                                                               | Effect                                                                                                                                                                                                                                                                               |
| ---------------------- | ------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `dev_edit_text`        | `{ "text": "…", "appId": "com.apple.TextEdit" }` (`appId` optional), or `{ "text": "…", "live": true }` | Runs the edit pipeline on `text` as if dictated into that app, or with `live` into the frontmost app's focused field as it is (its context is read, nothing is pasted), with the current settings, dictionary and snippets. Returns `plain`, `html`, `formatted`, `notice`, `llmMs`. |
| `dev_set_app_category` | `{ "appId": "com.apple.TextEdit", "category": "email" }` (`null` resets)                                | Treats that bundle ID as `personal`, `work`, `email`, `other` or `ai` (an AI assistant), so one app can test every style.                                                                                                                                                            |
| `dev_read_context`     | `{ "delayMs": 0 }` (optional)                                                                           | Reads the frontmost app's field as a dictation would: app kind, field kind, selection, text around it, URL, category, names, identifiers, file names and read time.                                                                                                                  |

A typical end-to-end check: make a TextEdit document frontmost, set the audio source to a
`say` WAV, call `flow_bar_control` `start`, and read the result back with
`osascript -e 'tell application "TextEdit" to get text of document 1'`. A rich-text
document shows pasted lists as `\t1\t…` list markers; a plain one (Format › Make Plain
Text) receives the plain text.

### Local Whisper tests

`src-tauri/tests/whisper_local.rs` runs the real model on the fixtures. It needs the model
files, so it is ignored by default. The dev app downloads them to
`~/Library/Application Support/com.shrimp.desktop.dev/models/` (Windows:
`%APPDATA%\com.shrimp.desktop.dev\models\`) on first launch:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --test whisper_local -- --ignored --nocapture
```

`SHRIMP_MODELS_DIR` points it at another folder and `SHRIMP_WHISPER_MODEL` picks another
catalog model (for example `small-q5_1`). It prints the latency of each run. The other
fixtures are `silence.wav` (3 s of digital silence) and `hello-shrimp-padded.wav` (the
speech fixture with 2 s of silence on each side).

## AI edits

### Providers and keys

- **Ollama** needs no key: install it, pull a model (`ollama pull qwen3.5:4b`, the measured
  pick) and choose Ollama in Settings › AI provider. With no model chosen, Shrimp uses the
  first installed of `qwen3.5:4b`, `gemma3:4b` and `llama3.2:3b`, else the first installed
  model.
- **Cloud keys** go into the keychain through the app: Settings › AI provider, or the
  `set_api_key` command (`{ "account": "anthropic" | "openai" | "groq", "key": "…" }`). An
  item written by another tool (for example `security add-generic-password`) belongs to
  that tool, so the first read from Shrimp Dev raises a keychain access prompt. The dev
  app's entries use the service `com.shrimp.desktop.dev`; remove one with
  `security delete-generic-password -s com.shrimp.desktop.dev -a <account>`.
- Settings › AI provider › Test runs the cleanup on a sample sentence and shows the
  latency or the provider's error.
- A key that is rejected, out of credits or rate limited is a quick way to see the error
  notices; a wrong Ollama address shows "Ollama isn't running".

### The eval set

`src-tauri/tests/ai_eval.rs` runs the 37 cases in `tests/fixtures/eval/cases.json`
(fillers, self-corrections, lists, spoken punctuation, line commands, questions and
injection attempts, code, dictionary terms, snippets, styles and email layout) through
the real edit pipeline and a real provider. It prints a table of outputs, failed checks
and cleanup latency, then a summary. It is ignored by default because it calls a model:

```sh
SHRIMP_EVAL_PROVIDER=ollama SHRIMP_EVAL_MODEL=qwen3.5:4b \
  cargo test --manifest-path src-tauri/Cargo.toml --test ai_eval -- --ignored --nocapture
```

| Variable               | Meaning                                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------------------- |
| `SHRIMP_EVAL_PROVIDER` | `anthropic`, `openai`, `groq` or `ollama` (default).                                              |
| `SHRIMP_EVAL_MODEL`    | The model ID; empty uses the provider's default (Ollama: the preferred installed model).          |
| `SHRIMP_EVAL_RUNS`     | Runs per case; latency is the median. Default 1.                                                  |
| `SHRIMP_EVAL_CASE`     | Comma-separated case IDs to run only those.                                                       |
| `SHRIMP_EVAL_STRICT`   | `1` fails the test when any check fails.                                                          |
| `ANTHROPIC_API_KEY` …  | Keys for cloud providers (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`), held in memory. |

"raw fallback" in the results means the guard rejected the model's output (or the
provider failed) and the unedited transcript would have been inserted. The pipeline's own
log lines, including rejected outputs at debug level, are printed under the case.

`src-tauri/tests/command_eval.rs` does the same for Command Mode and transforms: the 15
cases in `tests/fixtures/eval/commands.json` (edits, questions, drafts, injection
attempts in the selection and the document, Polish, Prompt Engineer and a custom
transform). The same variables apply, except `SHRIMP_EVAL_RUNS`:

```sh
SHRIMP_EVAL_PROVIDER=ollama SHRIMP_EVAL_MODEL=qwen2.5-coder:7b \
  cargo test --manifest-path src-tauri/Cargo.toml --test command_eval -- --ignored --nocapture
```

The kind column says whether the reply was an edit, an answer, or "withheld": an edit
that drops most of the text, which the app shows instead of pasting.

## Testing shortcuts and pasting in the real app

The keyboard hook sees events posted through the HID event tap like physical keys, so a
small helper run from a terminal that has Accessibility can drive dictation: post a
`kCGEventFlagsChanged` event with key code 63 and `kCGEventFlagMaskSecondaryFn` for Fn
down, then one with no flags for Fn up (Space with the Fn flag for Fn+Space). Read the
result back from the target app through System Events, for example the `value` of
TextEdit's text area.

Things to know:

- handy-keys tracks Command, Control, Option and Shift from `FlagsChanged` events. A
  synthetic Cmd+key posted with flags only (System Events `keystroke … using command
down`, or a helper that does the same) leaves Command looking held until the next plain
  key event, so the next Fn press does not match. Post a plain key such as Esc first, or
  post helper keys at `kCGAnnotatedSessionEventTap`, after the session taps. Physical keys
  always send `FlagsChanged` and are not affected; Shrimp posts its own paste after the
  session taps for the same reason.
- Command Mode's shortcut is Fn+Ctrl. Post Ctrl's `FlagsChanged` first and then Fn's
  (both flags set) to start a command directly; Fn first starts a dictation, which Ctrl
  then turns into a command. Turn Command Mode on first (Settings › Experimental, or
  `update_settings` with `{"experimental": {"commandMode": true}}`).
- A listen-only tap appended at the tail of the session (`tailAppendEventTap`) only sees
  what every other tap let through, which shows whether a shortcut was swallowed.
- Another dictation app that uses Fn (for example Wispr Flow) also hooks the keyboard and
  may draw its own bar where the Flow Bar is. Real clicks on the Flow Bar can land on it.
- Esc exits full screen in many apps, so do not use it as a reset key while testing in a
  full-screen Space.

- Check which app is frontmost before every synthetic key or dictation. A crash report
  dialog, a permission prompt or an IDE's chat panel can hold focus; Cursor often
  focuses its AI chat input (`AXDOMClassList` `aislash-editor-input`) when a window
  opens, so click into the editor first and confirm with `dev_read_context`.
- TextEdit saves new documents to iCloud Drive (`~/Library/Mobile
Documents/com~apple~TextEdit/Documents/Untitled*.rtf`) as soon as they change. Close
  test documents with `close every document saving no` and delete those files afterwards.

A WAV source needs no microphone permission. Everything from recording to transcription
and AI edits can therefore be tested without privacy grants. Only the hotkey and the paste
into another app still need them.

## Windows

### Building

- `pnpm check` works as on macOS. Debug and test builds run Whisper on the CPU.
- The GPU build adds the `vulkan` feature (`dev-app.ps1 build -Vulkan`, and always in
  `release.ps1`). It needs `VULKAN_SDK`, and a short `CARGO_TARGET_DIR`: whisper.cpp builds
  its shader generator as a nested CMake project whose files sit about 230 characters
  below the target directory, and MSBuild's file tracker fails beyond `MAX_PATH`, so even
  `%LOCALAPPDATA%\…` is too long. The scripts use `C:\st` unless `CARGO_TARGET_DIR` is set.
- `release.ps1` builds whisper.cpp for a portable CPU baseline (`GGML_NATIVE=OFF` with
  AVX2, BMI2, FMA and F16C) instead of the build machine's own instructions, and signs every
  binary through `scripts/windows/sign.ps1` when `SHRIMP_WINDOWS_SIGN_THUMBPRINT` names a
  code-signing certificate in the certificate store (`SHRIMP_WINDOWS_TIMESTAMP_URL`
  optional). Without it the installer is unsigned and the build says so. It also needs the
  updater key (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) and
  takes the feed from `SHRIMP_UPDATE_FEED`; see [RELEASING.md](RELEASING.md#windows).
- The installer (`tauri.windows.conf.json`) is NSIS, per user, into
  `%LOCALAPPDATA%\Shrimp` without administrator rights. A first install adds the login
  item (`src-tauri/windows/hooks.nsh`); updates keep the user's choice; uninstalling
  removes it. The Tauri CLI downloads NSIS into its cache on the first installer build.
  `Shrimp_<version>_x64-setup.exe /S` installs and `uninstall.exe /S` uninstalls without
  prompts; a silent uninstall keeps `%APPDATA%\com.shrimp.desktop` and
  `%LOCALAPPDATA%\com.shrimp.desktop` (WebView2's data).
- A login launch (`--background`) shows the main window anyway until onboarding is
  finished, as on macOS.

### Running the debug app

- `shrimp.exe` is a GUI app, also in debug builds (no console window); its logs go to
  tauri-mcp. `shrimp.exe --quit` asks the running debug instance to quit cleanly, which
  `dev-app.ps1 quit` uses; killing the process instead leaves the crash marker and the
  next launch shows "Shrimp quit unexpectedly". Logging off or shutting down exits
  cleanly: tao turns `WM_ENDSESSION` into `RunEvent::Exit`.
- Data lives in `%APPDATA%\com.shrimp.desktop.dev`; launch at login is the value
  "Shrimp Dev" under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`; API keys are
  generic credentials named `<account>.com.shrimp.desktop.dev` in Credential Manager.
- Windows asks no permission for the hooks, pasting or UI Automation. A normal process
  can't see keys typed into, or send keys to, an app running as administrator; pasting
  there leaves the text on the clipboard with "Run Shrimp as administrator…".
- tauri-mcp: on Windows the MCP server always connects to the pipe
  `\\.\pipe\tmp\tauri-mcp.sock` and can't read a token file next to it, so debug builds
  open that pipe without a token. A pipe's default security already limits it to the
  same user and administrators. `take_screenshot` of the main window can return a stale
  frame; `query_page` and `read_text` read the live page.

### Testing input on Windows

- Every `SendInput` event Shrimp sends carries `dwExtraInfo` 0x53485250 ("SHRP"), and
  Shrimp's hooks let those pass. Keys any other program sends, a test helper included,
  count as the user's, so a helper can hold Ctrl+Win to dictate.
- The keyboard hook, `SendInput`, the clipboard and UI Automation belong to the desktop
  session. From a remote shell (SSH runs in a session of its own), run helpers in the
  interactive session through a scheduled task with an interactive logon
  (`Register-ScheduledTask` with `-LogonType Interactive`, then `Start-ScheduledTask`).
- `GetLastInputInfo` counts injected input too. To tell whether the person at the PC is
  idle, watch with a low-level hook that ignores `LLKHF_INJECTED` and `LLMHF_INJECTED`.
- Esc while Ctrl+Win is held cancels the dictation; if a test sends Esc after the
  dictation ended, Ctrl+Esc reaches Windows and opens Start.
- The Flow Bar's notices disappear after a few seconds; read them with `read_text` on
  `flow-bar` right after the action.

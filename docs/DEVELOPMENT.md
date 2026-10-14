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
| `pnpm regression:mac`           | The macOS regression walkthrough against `Shrimp Dev` (see below).                     |
| `pnpm app:dev:win`              | Windows: builds the debug app `shrimp.exe` (`scripts/windows/dev-app.ps1 build`).      |
| `pnpm app:dev:win:open`         | Windows: quits a running `Shrimp Dev`, then launches it (builds it first if missing).  |
| `pnpm release:win`              | Windows: the per-user NSIS installer with Whisper on the GPU (`release.ps1`).          |
| `pnpm regression:win`           | Windows: the regression walkthrough against `Shrimp Dev` (see below).                  |
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

### Permissions, login, disk space, install location, region, mouse

Debug builds can pretend states that are hard to produce for real:

| Override                                              | Effect                                                                                                                                                                        |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `dev_set_permissions` `{ "accessibility": "denied" }` | Reports a permission as `granted`, `denied` or `notDetermined` (`null` goes back to the real state); its request and settings pane do nothing. Real grants are never touched. |
| `SHRIMP_PRETEND_PERMISSIONS=accessibility=denied`     | The same from launch, so the check after an update runs with it (`microphone=notDetermined` too, comma-separated).                                                            |
| `SHRIMP_LAUNCHED_AT_LOGIN=1`                          | The launch counts as one from the login item: the main window stays hidden.                                                                                                   |
| `SHRIMP_FAKE_FREE_BYTES=52000000`                     | Reports that much free disk space, to show the "Storage Full" screen.                                                                                                         |
| `SHRIMP_PRETEND_OUTSIDE_APPLICATIONS=1`               | macOS: shows "Move Shrimp to the Applications folder" and no shortcut dictates. Debug builds skip the real check; Move only logs what it would copy.                          |
| `SHRIMP_REGION=CA`                                    | Pretends the system region (two letters), for the Canadian English suggestion (LANG-07).                                                                                      |
| `SHRIMP_FAKE_MOUSE_BUTTONS=5`                         | Pretends a connected mouse with that many buttons, for the mouse button suggestion (CORE-18); `0` pretends none.                                                              |
| `SHRIMP_NO_HOTKEYS=1`                                 | Installs no keyboard or mouse hook: no shortcut dictates, and fn reaches the release Shrimp (or Wispr Flow) running beside it. Dictate with `flow_bar_control` instead.       |

Set environment variables with `scripts/macos/dev-app.sh open --env NAME=value`.

### Beside a release Shrimp

To work on the dev app while a release Shrimp is in use on the same Mac:

- Launch the bundle in the background with its hooks off:
  `scripts/macos/dev-app.sh open -g --env SHRIMP_NO_HOTKEYS=1`. `-g` keeps the person's
  app in front; tauri-mcp's `manage_window` `show` shows the window without taking focus.
- `pnpm tauri:dev` runs the raw binary, which comes to the front when it launches, and
  restarts (coming to the front again) whenever `src-tauri/` changes. For hot reloading
  beside someone's work, run it as
  `SHRIMP_NO_HOTKEYS=1 pnpm tauri dev --no-watch --config src-tauri/tauri.conf.dev.json`
  and give focus back once it is up.
- `dev_set_theme` `{ "theme": "dark" }` (`"light"`, or `null` for the system's) shows every
  window light or dark, so both appearances can be checked without changing the Mac's own
  setting.
- A dictation pastes into the app in front, so drive only states that insert nothing:
  notices (`manage_ipc` `emit` of `notify://notice` to `flow-bar`), `dictation://state`, and
  the pages and dialogs. The
  Canadian English and mouse button suggestions show once ever: set
  `notices.canadianEnglishShown` or `notices.mouseButtonsShown` back to `false`
  (`update_settings`) to see one again. To test onboarding, quit the app and move
  `~/Library/Application Support/com.shrimp.desktop.dev` aside first (it starts on a fresh
  install), then put it back. The login item is visible with
  `osascript -e 'tell application "System Events" to get the name of every login item'`;
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

`src-tauri/tests/command_eval.rs` does the same for Command Mode and transforms: the 19
cases in `tests/fixtures/eval/commands.json` (edits, questions, drafts, injection
attempts in the selection and the document, Polish, Prompt Engineer, a custom transform,
and the changes view's Turn to list, Translate to English and Empathize). The same variables apply, except `SHRIMP_EVAL_RUNS`:

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

## Regression walkthrough (macOS)

`pnpm regression:mac` (`scripts/macos/regression/run.sh`) drives the signed dev app through
every automatable PARITY item (AUD-02) and prints one line per case with the items it
covers: 85 cases over 128 items, in about 10 minutes. The cases in
`scripts/macos/regression/suites` also run on Windows (see
[Regression walkthrough (Windows)](#regression-walkthrough-windows)); `runner.mjs` holds
what both platforms share, and a case marked `platforms: ["darwin"]` runs on macOS only.

```sh
pnpm regression:mac                         # every case
pnpm regression:mac --only core,CMD-01      # suites (core, editing, library, commands, app, audit, relaunch) or PARITY ids
pnpm regression:mac --build --report out.json
pnpm regression:mac latency --runs 10       # AUD-03 instead: key-release-to-insert latency
```

What it needs:

- The terminal (or agent host) that runs it has Accessibility, and Shrimp Dev has
  Accessibility (see [One-time grants](#one-time-grants-from-phase-1)). The microphone is not
  used.
- The default Whisper model downloaded. Ollama running with a model for the AI cases;
  without it they are skipped.
- Wispr Flow quit: it also reacts to fn (the script warns).
- Nobody using the Mac: the cases press fn and other keys, and bring windows to the front.

What it does:

1. Builds `helper.swift` (synthetic keys and mouse buttons at the HID event tap, the
   clipboard, Accessibility reads and writes, and the latency timer) and `target.swift`, a
   small app with a text view, a text field, a password field and a list, into a temporary
   folder. Every dictation goes into that app, so no TextEdit document or iCloud file is
   created.
2. Builds `Shrimp Dev.app` if it is missing (or with `--build`), quits it, copies its data
   folder except `models/`, and saves the clipboard.
3. Launches the app and the target app and runs `walkthrough.mjs`, which talks to the app
   over tauri-mcp's socket (`mcp-client.mjs`: the same JSON lines the MCP server sends),
   switches the WAV source with `dev_set_audio_source`, uses the fake microphones, and
   reads the app's log for notices. The fixtures are the repository's WAV files plus a few
   sentences made with `say`.
4. Afterwards, whatever happened: quits both apps, puts the data folder and the clipboard
   back, and deletes the temporary folder.

Results are `PASS`, `FAIL`, `WARN` (the pipeline worked but a local model's output missed a
quality check, such as a list that wasn't formatted; model quality is measured by the eval
in [AI edits](#ai-edits)) and `SKIP`. The exit status is 1 when a case failed.

The `relaunch` suite runs last: it kills the app during a dictation, relaunches it with
the debug overrides (`SHRIMP_FN_KEY`, `SHRIMP_LAUNCHED_AT_LOGIN`,
`SHRIMP_PRETEND_OUTSIDE_APPLICATIONS`, `SHRIMP_REGION`, `SHRIMP_FAKE_MOUSE_BUTTONS`),
restarts it from the tray and resets it (APP-08); the saved data folder comes back
afterwards. The `latency` mode holds fn for 5 s over a 4.96 s sentence and times from the
release until the target field changes, polling Accessibility every 2 ms; it runs one
unmeasured dictation per mode first.

Not covered, and why:

| Items                                                                    | Why                                                                                                      |
| ------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| ENG-04, MIC-01, ENG-05, ENG-06                                           | Need a microphone or cloud keys with credits (the owner's checks).                                       |
| SND-03, SND-04                                                           | Mute and pause act on whatever the Mac is playing.                                                       |
| INS-03, INS-09, INS-10, INS-11, CORE-08, CORE-14, HIS-04, FMT-13, FMT-14 | Keyboard layouts, terminals and remote desktops, and long or dated timings: unit and controller tests.   |
| KEYS-04, KEYS-09, CORE-02 (the emoji picker)                             | Caps Lock and Secure Keyboard Entry change system state; the Globe key needs a physical press.           |
| DEV-01, DEV-02, STY-03, STY-04, STY-05, CTX-04, DIC-07                   | Cursor, Windsurf and browsers aren't launched, and respelling needs a person: unit and controller tests. |
| ONB-01 to ONB-08, A11Y-01, A11Y-02, MIC-10, TRF-03                       | Onboarding needs a fresh data folder; the rest is UI covered by Vitest and axe.                          |
| STA-03, STA-04                                                           | Sharing opens the browser; the voice profile needs 2,000 words.                                          |
| PKG-01 to PKG-05                                                         | Release builds: see [RELEASING.md](RELEASING.md).                                                        |
| WIN-01 to WIN-12                                                         | Windows.                                                                                                 |

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
- A helper can't bring Shrimp's window to the front while an elevated app (Task Manager,
  for one) is in front; one started by a scheduled task with `-RunLevel Highest` can, with
  `SetForegroundWindow` alone. Start Shrimp itself from a normal task, so it isn't elevated.
- Show and hide the main window through Tauri (tauri-mcp's `manage_window`), not with
  `ShowWindow`: tao keeps its own visibility flag and hides the window again at its next
  style change, such as maximizing.

### Regression walkthrough (Windows)

`pnpm regression:win` (`scripts/windows/regression/run.ps1`) runs the macOS walkthrough's
cases against `Shrimp Dev` on Windows, plus the Windows-only cases in
`scripts/windows/regression/suites/windows.mjs`, and prints the same one line per case.

```powershell
pnpm regression:win                               # every case
pnpm regression:win -Only core,CMD-01             # suites (as on macOS, plus windows) or PARITY ids
pnpm regression:win -Build -Report C:\out.json    # rebuild Shrimp Dev first
pnpm regression:win latency -Runs 10 -Model qwen2.5-coder:7b   # AUD-03 instead
```

What it needs:

- The interactive desktop session: from SSH or another session, start it through a
  scheduled task (see [Testing input on Windows](#testing-input-on-windows)).
- Shrimp Dev's data folder with onboarding finished and the default model downloaded. Ollama
  running with a model for the AI cases; without it they are skipped. Load the model once
  beforehand (for example `ollama run <model> hi`): AI edits give up after 5 s, and the first
  request after Ollama starts may take longer.
- Nobody using the PC. A guard (`shrimp-helper.exe watch`) stops the run at the first key
  press or mouse movement that wasn't injected, and the helper refuses to send keys when an
  app other than the target app or Shrimp is in front.

What it does:

1. Builds `helper.cs` (keys and mouse buttons through `SendInput`, the clipboard, the target
   app's fields, Shrimp's tray icon, the speakers' mute and the guard) and `target.cs`, a
   window with a text box, a field and a password field (the system's own EDIT controls, as
   in Notepad) and a list, with the .NET Framework compiler that ships with Windows, into a
   temporary folder.
2. Builds `shrimp.exe` with Whisper on the GPU if it is missing (or with `-Build`; `-Cpu` for
   a CPU build), quits it, copies its data folder except `models\`, and saves the clipboard,
   clipboard history's entries and the window in front. `-MuteSpeakers` also mutes the
   speakers, as the cues play many times.
3. Launches Shrimp Dev with a silent WAV as its audio source (the microphone is never used)
   and the target app, and runs `walkthrough.mjs`. Its context (`context.mjs`) turns the
   macOS defaults the cases press into the Windows defaults (fn into Ctrl+Win, ⌥1 into
   Win+Alt+1, ⌃⌘V into Shift+Alt+Z…). The spoken fixtures are made with Windows' text to
   speech (`speak.ps1`). Mute while dictating is turned off for the run, so no app left
   playing is muted.
4. Afterwards, whatever happened: quits both apps, puts the data folder, the clipboard, the
   speakers and the window in front back, and deletes the clipboard-history entries the run
   added (Copy last transcript adds one on purpose) and the temporary folder.

The Windows cases imitate other apps by running the target app under their executable names
(`Code.exe`, `Hyper.exe`, `wezterm-gui.exe`, `WINWORD.EXE`, `mstsc.exe`): the paste key, the
clipboard hold, context spacing and Insights' categories follow the executable name, and the
target app records whether a paste came as Ctrl+V or Shift+Insert. `-Extra <module>` also
runs the cases a module exports as `cases`, for one-off checks with apps the repository
doesn't ship. `-Report <file>` writes the app's log beside the report (`<file>.app.log`), and
`-KeyLog <file>` has the guard write down every injected key (Shrimp's carry its tag), which
shows what reached Windows when a shortcut misbehaves.

Not covered on Windows, besides the macOS list's microphone, cloud, onboarding and release
items: WIN-06 (an elevated app needs a UAC prompt), WIN-07 (placeholders shown as text are a
browser's; a native field's cue banner is no value), WIN-08 and WIN-12 (they act on whatever
the PC is playing), and the macOS-only APP-07, APP-10, APP-11 and CORE-10. APP-03 and APP-12
click Shrimp's tray icon with the mouse, at the taskbar button UI Automation finds for it,
and skip when the icon sits in the taskbar's flyout of hidden icons: Windows gives no
position for it there (`Shell_NotifyIconGetRect` returns `S_FALSE`), and tray-icon ignores a
click it can't place. The walkthrough doesn't open the flyout, which belongs to Explorer. To
run them, set Shrimp's rows to "Show icon and notifications" in the Notification Area Icons
control panel (`explorer shell:::{05d7b0f4-2121-4eff-bf6b-ed3f69b894d9}`) before the run and
back afterwards. Windows 10 drops that choice when Shrimp quits, so APP-12, which restarts
Shrimp first, still skips in a full walkthrough.

Things to know:

- `GetLastInputInfo` counts the run's own input, so right after a run it can't tell whether
  the person at the PC is idle; the guard's hook, which ignores injected input, can.
- `SendMessageTimeout` returns a wrong `EM_GETSEL` result across processes; the helper reads
  the selection with `SendMessage`.
- WinForms text boxes don't give UI Automation the text patterns Notepad's EDIT control does,
  so the target app creates EDIT controls itself.

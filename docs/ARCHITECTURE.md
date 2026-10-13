# Architecture

Shrimp is one Tauri v2 process: a Rust core plus React webviews. This document covers the
module layout, the dictation data flow, the platform abstraction layer, storage and the
provider traits. Crate versions and API facts were checked against crates.io, docs.rs and the
crates' source on 2026-10-13; the [dependency table](#dependencies) lists them.

Status markers: **(built)** exists in the repository now; **(P1)** to **(P5)** names the
phase that adds it (phases are defined in [PARITY.md](PARITY.md)).

## Principles

- **Local first.** Audio, transcripts, history and settings stay on the device. Cloud STT and
  LLM providers are opt-in and use the user's own key.
- **Rust owns behaviour, the webview owns presentation.** Hotkeys, audio, STT, AI edits,
  insertion and storage all run in Rust. The UI renders state and sends intents through IPC.
- **Platform code sits behind traits.** Everything OS-specific lives in `platform/`, with a
  macOS and a Windows backend. The rest of the core is portable and testable on any OS.
- **Everything is testable without a microphone or permissions.** Audio sources, STT
  engines, LLM providers and platform services are traits, so tests and dev builds can swap
  in WAV files and fakes (see [DEVELOPMENT.md](DEVELOPMENT.md)).
- **Latency matters more than features.** The hot path (key release to text inserted) avoids
  blocking work on the async runtime, keeps the Whisper model loaded, and gathers context
  in parallel with recording.

## Processes, windows and threads

| Piece         | What it is                                                                                                                                               |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Main window   | `main` webview: Home, Insights, Dictionary, Snippets, Style, Transforms, Settings, onboarding. Closing hides it **(built)**.                             |
| Flow Bar      | `flow-bar` webview in a transparent, always-on-top window that never takes focus. On macOS it is a non-activating `NSPanel` (tauri-nspanel) **(built)**. |
| Tray          | Menu-bar / notification-area icon and menu: Open Shrimp, Shortcuts…, Paste last transcript, Copy last transcript, Quit **(built)**.                      |
| Hotkey thread | Owns the OS keyboard hook (a CGEventTap run loop on macOS, `WH_KEYBOARD_LL` on Windows) **(built)**.                                                     |
| Audio thread  | The cpal input stream callback, which only converts samples to f32 and sends them **(built)**.                                                           |
| Async runtime | Tauri's tokio runtime: the dictation controller task, IPC commands, HTTP calls.                                                                          |
| Blocking pool | `spawn_blocking` for Whisper inference, SQLite and accessibility calls.                                                                                  |
| Main thread   | AppKit/Win32 work that must run there (windows, panels, pasteboard and key posting where required), reached with `AppHandle::run_on_main_thread`.        |

## Rust module layout

`src-tauri/src/`:

```text
main.rs                 entry point; calls shrimp_lib::run()                           (built)
lib.rs                  Tauri builder: plugins, commands, setup                        (built)
app/                    shell                                                          (built)
  mod.rs                run-loop events (Dock reopen)                                  (built)
  setup.rs              builds the services at startup, managed state                  (built)
  window.rs             main window show/hide, close-to-tray                           (built)
  tray.rs               tray icon and menu                                             (built)
  flow_bar.rs           Flow Bar window creation, sizing, bottom-centre placement      (built)
  events.rs             DictationUi over Tauri events; notices queued until ready      (built)
  dev_bridge.rs         tauri-plugin-mcp, debug builds only                            (built)
commands/               thin IPC handlers, one file per area                           (built)
  app.rs, dictation.rs, models.rs, permissions.rs, dev.rs (debug only)
audio/                  audio input                                                    (built)
  source.rs             AudioSource trait, AudioSink, AudioFormat                      (built)
  wav.rs                WavFileSource, read_wav                                        (built)
  mic.rs                MicrophoneSource (cpal), device presence check                 (built)
  recorder.rs           Recorder: collects a source into an AudioClip                  (built)
  clip.rs               AudioClip (16 kHz mono f32), downmix + rubato resampling       (built)
  level.rs              RMS, waveform level, silence trim                              (built)
  spec.rs               AudioSourceSpec, SHRIMP_AUDIO_SOURCE parsing, open()           (built)
  devices.rs            device listing, ranking, fallback                              (P4)
  wav_encode.rs         16-bit WAV encoding for history files and cloud uploads        (P2)
stt/                    speech-to-text                                                 (built)
  engine.rs             SttEngine trait, SttOptions, Transcript, SttError              (built)
  whisper_local.rs      LocalWhisper (whisper-rs, Metal) with Silero VAD               (built)
  slot.rs               EngineSlot: the engine dictations use, empty until loaded      (built)
  models.rs             model catalog, download, SHA-256 check                         (built)
  manager.rs            ModelManager: download progress, load into the slot            (built)
  openai_compat.rs      OpenAI and Groq transcription (multipart)                      (P2)
llm/                    LLM providers                                                  (built: trait)
  provider.rs           LlmProvider trait, LlmRequest, LlmResponse, LlmError           (built)
  anthropic.rs          Anthropic Messages API                                         (P2)
  openai.rs             OpenAI Responses API                                           (P2)
  groq.rs               Groq chat completions (OpenAI-compatible)                      (P2)
  ollama.rs             Ollama /api/chat                                               (P2)
edit/                   AI edit pipeline                                               (P2)
  pipeline.rs           pre-pass, LLM pass, post-pass, raw fallback                    (P2)
  prompts.rs            system prompts per cleanup level, style and mode               (P2)
  spoken.rs             spoken commands: "new line", punctuation words                 (P2)
  snippets.rs           trigger matching and expansion                                 (P2)
  dictionary.rs         replacements and vocabulary hints                              (P2)
  style.rs              app category, casing and trailing-period rules                 (P2)
  context_format.rs     casing and spacing from the text around the cursor             (P3)
  command.rs            Command Mode and Transforms                                    (P3)
dictation/              orchestration                                                  (built)
  controller.rs         the dictation actor (state machine below)                      (built)
  gestures.rs           hold / double-tap / short-tap / busy recognition, pure         (built)
  pipeline.rs           trim, transcribe, insert                                       (built)
  state.rs              DictationState, DictationUi, Timing, inputs, outcomes          (built)
hotkeys/                bindings, platform independent                                 (built)
  binding.rs            Action, Binding, macOS defaults and labels                     (built)
  matcher.rs            raw key events to shortcut press/release                       (built)
  validation.rs         shortcut rules and error texts (KEYS-03..05)                   (P4)
platform/               OS integration behind traits (next section)                    (built)
  mod.rs                traits + Platform bundle + GlobeKeyAction
  macos/                hotkeys, insert, pasteboard, keymap, focus, permissions,
                        sound, globe, flow_bar (NSPanel)                               (built)
  windows/              Windows backend                                                (P5)
storage/                SQLite                                                         (P2 dictionary/snippets, P4 history)
  db.rs                 open, WAL, migrations, backups
  history.rs, dictionary.rs, snippets.rs, stats.rs
settings/               typed Settings, persistence, change events                     (P2)
secrets/                API keys in the OS keychain (keyring)                          (P2)
sounds/                 start/stop/lock/cancel/warning cues, synthesised               (built)
notify/                 notices shown next to the Flow Bar                             (built)
```

Rules: keep files under about 300–500 lines with one responsibility each; domain modules never
import `commands/`; only `platform/` contains `#[cfg(target_os)]` code (plus the few lines in
`app/` that Tauri requires).

## Frontend layout

```text
index.html              main window entry                                              (built)
flow-bar.html           Flow Bar entry (Vite multi-page build)                         (built)
src/
  main.tsx              main window bootstrap + dev MCP bridge                         (built)
  app/                  main window: model, permissions and shortcuts cards            (built)
  flow-bar/             pill, waveform, notices, content-size reporting                (built)
  lib/ipc.ts            typed wrappers for every command and event                     (built)
  lib/useTauriEvent.ts  event subscription hook                                        (built)
  lib/devBridge.ts      loads the tauri-plugin-mcp listeners in development builds     (built)
  lib/settings.ts       settings types mirrored from Rust                              (P2)
```

React 19 + TypeScript (strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`),
Vite 8, Vitest + Testing Library with `@tauri-apps/api/mocks` for IPC. Rust and TypeScript
types are mirrored by hand. Each Rust type that crosses IPC has a serialisation test that pins
its JSON shape (see `commands/app.rs`).

Each webview entry point (main window and Flow Bar) loads the tauri-plugin-mcp guest
listeners in development-mode builds, so agents can drive every window.

IPC conventions:

- Commands are `snake_case` verbs (`get_settings`, `update_settings`, `list_history`).
- Events are namespaced `area://name` (`dictation://state`, `dictation://level`,
  `settings://changed`, `history://added`).
- Payloads use `camelCase` (`#[serde(rename_all = "camelCase")]`).
- Every command is also listed in `commands::NAMES` for the dev MCP bridge.

Built in phase 1:

| Kind     | Names                                                                                                                                                                                                                   |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Commands | `app_info`, `get_dictation_state`, `flow_bar_control`, `flow_bar_ready`, `flow_bar_layout`, `run_notice_action`, `get_shortcuts`, `get_models`, `download_default_model`, `get_permissions`, `open_permission_settings` |
| Dev only | `dev_get_audio_source`, `dev_set_audio_source` (debug builds; see DEVELOPMENT.md)                                                                                                                                       |
| Events   | `dictation://state`, `dictation://level` (Flow Bar only), `notify://notice`, `models://status`, `app://navigate`                                                                                                        |

The Flow Bar calls `flow_bar_ready` once its notice listener is registered; notices raised
before that (for example a missing permission at startup) are queued and delivered then. It
also reports its content size through `flow_bar_layout`, and Rust resizes the window to fit,
so transparent margins never sit over other apps.

## Data flow: hotkey to inserted text

```text
 hotkey thread            dictation controller (tokio task)                    UI
 ─────────────            ──────────────────────────────────                   ──
 key down ──HotkeyEvent──▶ gestures: press(PushToTalk)
                           ├─ Recorder::start(source)  ◀── audio thread pushes samples
                           ├─ spawn: capture context (frontmost app, selection,
                           │         text around cursor)        ──────────────▶ Flow Bar: recording
                           ├─ media: mute or pause (if enabled)
                           ├─ sound: start cue
                           │  every ~33 ms: Recorder::poll() → RMS ─ dictation://level ─▶ waveform
 key up ───HotkeyEvent──▶  ├─ gestures: release (≥ 0.5 s) → stop
                           ├─ Recorder::finish() → AudioClip (16 kHz mono)   ─────────▶ processing
                           ├─ media: restore; sound: stop cue
                           ├─ SttEngine::transcribe(clip, options)  [blocking pool / HTTP]
                           ├─ edit::pipeline(transcript, context, settings)
                           │    pre-pass → LlmProvider::complete → post-pass (raw fallback)
                           ├─ platform.inserter.insert(text, target app)
                           ├─ storage: history row + daily stats; recording WAV
                           └─ state → Idle                          ─ history://added ─▶ Home
```

### Controller state machine (P1)

```text
Idle ── press PTT ─────────────▶ Recording(Hold)
Idle ── press Hands-free ──────▶ Recording(Locked)
Idle ── press Command Mode ────▶ Recording(Hold, Command)                       (P3)
Recording(Hold) ── release < 0.5 s ──▶ AwaitSecondTap ── 2nd press ≤ 0.5 s ──▶ Recording(Locked)
                                              └── timeout ──▶ Idle (tip, CORE-03)
Recording(Hold) ── release ──────▶ Processing
Recording(Locked) ── press PTT / Hands-free / Flow Bar ■ ──▶ Processing
Recording(*) ── Cancel (Esc) / Flow Bar ✕ ──▶ Idle (audio kept for Undo, CORE-11)
Recording(*) ── 20 min ──────────▶ Processing (warning at 19 min, CORE-08)
Processing = Transcribing → Editing → Inserting → Idle
Processing ── any dictation shortcut ──▶ rejected, "Transcript currently processing" (CORE-07)
```

Phase 1 runs the steps in this diagram except context capture (P3), media mute and pause
(P4), the edit pipeline (P2) and storage (P4). The last dictation (its text, or its audio
when it failed or was cancelled) is kept in memory for paste and copy last transcript.

Notes:

- Recording starts on key down, not after a decision delay, so first words are never lost
  (ENG-04). A short tap discards the audio.
- After a double-tap lock, a third press within 0.5 s cancels (CORE-05). A Flow Bar click
  within 0.5 s of a click start cancels (CORE-09).
- Fn+Space is a chord: Fn goes down first and starts a hold recording, and Space then locks
  it. To stop, the Fn press finishes the recording, and the Space that follows while Fn is
  still held is ignored rather than reported as a busy press.
- Esc stays bound (swallowed) until its key-up, so neither half of the press reaches the app.
- A WAV source that runs out finishes the dictation as if the shortcut had been released.
- If Ctrl joins a held Fn, the session is upgraded from dictation to Command Mode in place
  **(P3)**.
- Context capture runs in parallel with recording and is dropped if it is not ready when
  editing starts (CTX-01: never delay dictation).
- The controller is a single task that owns all session state and receives `HotkeyEvent`s,
  UI intents and internal results on one channel. State changes are emitted as
  `dictation://state`.
- Insertion re-checks that the target app is still frontmost. If not, or if the paste fails,
  the text stays on the clipboard and a notice offers Copy (INS-04).

## Platform abstraction layer

`platform/mod.rs` defines one trait per capability and a `Platform` bundle that `lib.rs`
builds once with `platform::native()`. Tests use fakes.

```rust
pub struct Platform {
    pub hotkeys: Box<dyn HotkeyBackend>,
    pub inserter: Arc<dyn TextInserter>,
    pub permissions: Arc<dyn Permissions>,
    pub sounds: Arc<dyn CuePlayer>,
    pub default_bindings: Vec<Binding>,
    pub label: fn(&Hotkey) -> String, // "fn Space", "⌃⌘V"
    pub language: Option<String>,     // preferred language, ISO 639-1
    pub globe_key: GlobeKeyAction,    // what Fn alone does (CORE-02)
}

/// Global shortcuts, including Fn and modifier-only chords, with press and release.
pub trait HotkeyBackend: Send {
    /// Starts the OS hook on its own thread; matches are passed to `on_event`.
    fn start(&mut self, on_event: Box<dyn Fn(ShortcutEvent) + Send + Sync>) -> Result<(), PlatformError>;
    /// Replaces the active bindings. Bound input is swallowed, so it never reaches other apps.
    fn set_bindings(&mut self, bindings: &[Binding]);
    fn stop(&mut self);
    // P4: fn record_combination(&mut self) -> Result<Receiver<RawCombination>, PlatformError>;
}

pub trait TextInserter: Send + Sync {
    /// Pastes into the focused field: snapshot the clipboard, write `text`, send the
    /// paste keystroke, restore after `hold` unless the user copied something meanwhile.
    /// Leaves the text on the clipboard instead when no text field is focused (INS-04).
    fn insert(&self, text: &str, hold: Duration) -> Result<InsertOutcome, PlatformError>;
    fn copy(&self, text: &str) -> Result<(), PlatformError>;
    // P2: rich text (InsertText); P3: fn send_enter(&self, modifiers: Modifiers)
}

pub trait Permissions: Send + Sync {
    fn status(&self, permission: Permission) -> PermissionStatus; // Microphone, Accessibility
    fn request(&self, permission: Permission) -> Result<(), PlatformError>;
    fn open_settings(&self, permission: Permission) -> Result<(), PlatformError>;
}

pub trait CuePlayer: Send + Sync {
    fn play(&self, cue: Cue);
}

// P3: FocusInspector (frontmost app, selection, text around the cursor) for context.
// P4: MediaController (mute or pause while dictating).
```

Phase 1 needs only one piece of focus information, whether an editable field is focused,
and the macOS inserter reads it itself (`platform/macos/focus.rs`). The `FocusInspector`
trait arrives with context awareness in phase 3.

Bindings, their validation rules and the per-platform defaults live in `hotkeys/`. Gesture
recognition (hold, double-tap, short tap) lives in `dictation/gestures.rs`. Both are platform
independent and unit-tested with synthetic event sequences. The cancel binding (Esc) is only
registered while a dictation is recording; a blocking hook would otherwise swallow every Esc.
Keys Shrimp posts itself (paste, later Enter) go in after the session event taps, so its own
hook never sees them, and carry the `EventSourceUserData` tag `0x53485250` ("SHRP").

### macOS backend (`platform/macos/`)

The required permissions are **Microphone** and **Accessibility**. Accessibility covers both
listening to and posting events, so Input Monitoring is only needed if a listen-only tap is
ever used (Apple DTS, developer forums thread 828052).

| Capability                | Implementation                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Permission                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------- |
| Global hotkey, incl. Fn   | handy-keys 0.3.4 `KeyboardListener::new_with_blocking(set)`: an active `CGEventTap` (session tap, head insert, default options) on its own `CFRunLoop` thread. Fn is detected from `FlagsChanged` with key code `0x3F` and `MaskSecondaryFn`; left and right modifiers are distinct; the tap is re-enabled after `TapDisabledByTimeout`. Shrimp matches the raw `KeyEvent`s to bindings itself (`hotkeys/matcher.rs`, the same rules as handy-keys' `HotkeyManager`), because `HotkeyManager`'s receiver is not `Sync`: registering from the controller and receiving on the hook thread cannot share it. The blocking set is an `Arc<Mutex<HashSet<Hotkey>>>` that `set_bindings` replaces. For Fn alone only the key-down is swallowed. handy-keys tracks Command, Control, Option and Shift from `FlagsChanged` and corrects them only on the next key event, so a synthetic Cmd+key without `FlagsChanged` leaves Command looking held until then; that is why Shrimp posts its own keys after the session taps. Never poll `CGEventTapIsEnabled` (it leaked kernel resources in Handy).                                                                                                                                                                                                                                                                                               | Accessibility               |
| Mouse-button shortcuts    | handy-keys handles keys only. Phase 4 adds an objc2-core-graphics tap for other-mouse-button events (CORE-13).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | Accessibility               |
| Secure Event Input        | While another app has Secure Keyboard Entry on, taps still receive `FlagsChanged` (Fn and modifier-only shortcuts keep working) but no key down/up. Shrimp polls `IsSecureEventInputEnabled()` and meanwhile registers keyed shortcuts (Esc, Cmd+Ctrl+V) through tauri-plugin-global-shortcut, whose Carbon `RegisterEventHotKey` is unaffected. That plugin cannot bind Fn or modifier-only chords (KEYS-09).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | none                        |
| Fn / Globe key            | Pressing Fn alone also triggers the system Globe action (`com.apple.HIToolbox` `AppleFnUsageType`: 0 nothing, 1 input source, 2 emoji (the default when unset), 3 dictation). Only Fn's key-down is swallowed, and whether that stops the Globe action could not be tested without a physical key press. At startup Shrimp reads the setting with `CFPreferencesGetAppIntegerValue` and, unless it is "Do Nothing", shows a notice that opens Keyboard settings (CORE-02); onboarding repeats this in phase 4.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | none                        |
| Text insertion            | First, AX decides whether an editable field is focused (`AXFocusedUIElement` of the frontmost app: text roles, `AXSelectedTextRange` or a settable `AXValue` count as editable; buttons, lists and similar do not; anything else is unknown and pasted into). Chromium and Electron get `AXManualAccessibility` set first, and a "not editable" answer is checked once more after 80 ms, because WebKit and Chromium build their tree on demand. Then, on the main thread: snapshot every `NSPasteboard` item and type as bytes (text, RTF and images survive), write the text (plain; rich text in P2) marked `org.nspasteboard.TransientType` and `AutoGeneratedType`, and post Cmd+V with objc2-core-graphics `CGEvent::new_keyboard_event` and explicit `set_flags(MaskCommand)`, so a physically held Option can't turn it into Cmd+Opt+V. The keys are posted at `kCGAnnotatedSessionEventTap`, after the session event taps: apps receive them, keyboard hooks (Shrimp's own included) do not. The key code is the one that types "v" with Command held in the current layout, from `TISCopyCurrentKeyboardLayoutInputSource` + `UCKeyTranslate` (fallback `kVK_ANSI_V`, 9; Dvorak gives 47, AZERTY and "Dvorak - QWERTY ⌘" give 9). After 0.5 s the snapshot is restored only if `changeCount` is still Shrimp's. enigo is not used: its layout lookups crash off the main thread. | Accessibility               |
| Frontmost app             | `NSWorkspace::sharedWorkspace().frontmostApplication()`: bundle ID, name, PID (objc2-app-kit features `NSWorkspace`, `NSRunningApplication`, `libc`). The window title comes from AX (`AXFocusedWindow` → `AXTitle`); `CGWindowList` titles would need Screen Recording.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | none (title: Accessibility) |
| Selection and nearby text | objc2-application-services: `AXUIElement::new_application(pid)` with `set_messaging_timeout(0.25)` → `AXFocusedUIElement` → `AXSelectedText`, `AXValue`, `AXSelectedTextRange` (a `CFRange` inside an `AXValue`) and `AXStringForRange` for the text around the cursor; `AXRole`/`AXSubrole` to skip secure fields; `AXPlaceholderValue`. Use the per-app element: the system-wide element returned `kAXErrorCannotComplete` in testing. The crate does not export the `kAX…` names (they are `CFSTR` macros), so attributes are built with `CFString::from_static_str("AXFocusedUIElement")`. Electron and Chromium apps may need `AXManualAccessibility` set first (unverified). Fallback: clipboard snapshot, Cmd+C, wait for `changeCount`, read, restore. Writing `AXSelectedText` is ignored by many web editors, so replacements are pasted.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | Accessibility               |
| Browser URL               | The `AXURL` / `AXDocument` attribute of the focused web area (P3, STY-05).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Accessibility               |
| Mute while dictating      | Core Audio: `kAudioDevicePropertyDeviceIsRunningSomewhere` on the default output device, then device mute (volume 0 as fallback) and restore (SND-03).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | none                        |
| Pause media (P4)          | Since macOS 15.4, in-process MediaRemote reads return nothing for third-party apps, but commands still work. So: return early if the output device isn't running; probe "now playing" with `osascript -l JavaScript` (`MRNowPlayingRequest`, about 60–130 ms); if something plays, send `MRMediaRemoteSendCommand(kMRPause)` through `dlopen` and remember the bundle ID; afterwards send `kMRPlay` only if that app is still paused. Any failure means doing nothing. A play/pause media-key event is a toggle and is only safe when the state is known.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | none                        |
| Permissions               | Direct calls: objc2-av-foundation `AVCaptureDevice::authorizationStatusForMediaType` / `requestAccessForMediaType_completionHandler`; `AXIsProcessTrusted` (the Accessibility pane is opened, not prompted); objc2-io-kit `IOHIDCheckAccess` / `IOHIDRequestAccess(ListenEvent)` for Input Monitoring, if a listen-only tap is ever added. The microphone is requested on the first microphone dictation, after checking that an input device exists, so a Mac without one never sees the prompt. Deep links: `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility` (also `Privacy_Microphone`, `Privacy_ListenEvent`). tauri-plugin-macos-permissions 2.3.0 is not used: it reads an `NSInteger` as `i32`, which objc2 rejects with a panic in debug builds.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | —                           |
| Flow Bar panel            | tauri-nspanel 2.1.1: a `tauri_panel!` class with `can_become_key_window: false` and `is_floating_panel: true`; `window.to_panel()`, `PanelLevel::Status`, the non-activating style mask, collection behaviour `can_join_all_spaces().full_screen_auxiliary().stationary().ignores_cycle()`, `set_hides_on_deactivate(false)`. Panel calls must run on the main thread. Tauri's `focusable(false)` alone does not stop a click from activating the app; the window also needs `accept_first_mouse(true)` so the first click on the inactive panel registers. Show it with Tauri's `window.show()` before `orderFrontRegardless`: while Tauri believes the window is hidden, WebKit keeps painting a stale snapshot of the first frame over later content. Needs Tauri's `macos-private-api` feature (`macOSPrivateApi` in tauri.conf.json).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | none                        |
| Dictation sounds          | `NSSound` from in-memory WAV data, created and played on the main thread, so no audio output stream stays open between cues (rodio would hold one). The cues are synthesised in `sounds/`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | none                        |
| Launch at login (P4)      | objc2-service-management `SMAppService::mainAppService()` (`registerAndReturnError`, `status`), the modern login-item API. tauri-plugin-autostart would install a LaunchAgent or use AppleScript instead.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | none                        |

### Windows backend (`platform/windows/`, P5)

| Capability                | Implementation                                                                                                                                                                                                                                                                                       |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Global hotkey             | handy-keys' `WH_KEYBOARD_LL` backend; a `WH_MOUSE_LL` hook for mouse buttons. Callbacks stay fast (Windows silently drops hooks that exceed `LowLevelHooksTimeout`) and skip `LLKHF_INJECTED` events. Ctrl+Win keeps Start closed while held (WIN-02).                                               |
| Text insertion            | clipboard-rs 0.3.5 for a multi-format snapshot and restore; the temporary write adds `ExcludeClipboardContentFromMonitorProcessing` and `CanIncludeInClipboardHistory = 0` (WIN-05); `SendInput` Ctrl+V, or Shift+Insert in editors (WIN-04).                                                        |
| Frontmost app             | `GetForegroundWindow` → `GetWindowThreadProcessId` → `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW`; `GetWindowTextW` for the title.                                                                                                                                |
| Selection and nearby text | uiautomation 0.25.1: `UIAutomation::new()?.get_focused_element()?`, `UITextPattern` (`get_selection`, `get_document_range`, `get_caret_range`), `UIValuePattern`; password fields are skipped.                                                                                                       |
| Media                     | `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?` on a multithreaded COM thread (windows 0.62 removed `.get()`): pause sessions whose status is `Playing` and resume the same ones (WIN-12). Per-app mute through `IAudioSessionManager2` / `ISimpleAudioVolume` (WIN-08). |
| Permissions               | Microphone privacy setting check; Windows has no accessibility permission.                                                                                                                                                                                                                           |
| Flow Bar                  | tao's `focusable(false)` sets `WS_EX_NOACTIVATE`, but tao rewrites the extended style on every show and hide, so the bar is shown with `ShowWindow(SW_SHOWNOACTIVATE)` and `WS_EX_TOOLWINDOW` is re-applied. Whether WebView2 still takes focus is to be verified.                                   |

Crates evaluated and not used: rdev (unmaintained, crashes under `tauri dev`), device_query
(archived), inputbot (no release since 2023), enigo for pasting (main-thread crashes on
macOS), accessibility and active-win-pos-rs (legacy objc and core-graphics bindings), x-win
(window titles need Screen Recording), tauri-plugin-macos-permissions (bug above), and the
crates.io crate named `tauri-plugin-mcp`, which is an unrelated project (Shrimp uses
P3GLEG's plugin from git).

## Audio

Built in phase 0; see [DEVELOPMENT.md](DEVELOPMENT.md#audio-from-a-wav-file-instead-of-the-microphone)
for usage.

- `AudioSource` (trait): `start(sink) -> AudioFormat` and `stop()`. Sources push interleaved
  f32 chunks through `AudioSink`, which wraps an `mpsc` channel. Implementations:
  `WavFileSource` **(built)** and `MicrophoneSource` **(built)**.
- `MicrophoneSource` **(built)**: cpal 0.18 input stream on the selected device in its default
  configuration, owned by its own thread because CoreAudio streams are not `Send`. The
  callback converts samples to f32 and sends them; it does no other work. cpal 0.18 streams
  start only after `play()`. A Mac with no input hardware can still report a default input
  device; a device without a usable default configuration counts as no microphone.
- `Recorder` **(built)**: drains the channel, exposes newly added samples for level
  metering, and converts to `AudioClip` on `finish()`.
- `AudioClip` **(built)**: mono f32 at 16 kHz. Downmix averages channels; resampling uses
  rubato 5's `Fft` resampler with `process_all`, which trims the filter delay.
- Speech detection (ENG-03): first an RMS check per 20 ms frame (about -50 dBFS) trims
  leading and trailing silence and rejects clips with no signal before any STT call
  (`audio/level.rs`). Then `LocalWhisper` runs Silero VAD (`ggml-silero-v6.2.0.bin`, 885 KB)
  through its own `WhisperVadContext`: no speech segments means no Whisper call, and the
  audio is cut to the span of speech plus 0.25 s. `FullParams::enable_vad` is not used: in
  testing, Whisper still turned 3 s of silence into "you" with it enabled.
- Recordings for history are written as 16-bit WAV (hound) to `recordings/<id>.wav` and
  deleted after 14 days (HIS-04).

## Speech-to-text

```rust
#[async_trait]
pub trait SttEngine: Send + Sync {
    fn id(&self) -> SttEngineId;                      // LocalWhisper | OpenAi | Groq
    async fn transcribe(&self, clip: &AudioClip, options: &SttOptions) -> Result<Transcript, SttError>;
}
// SttOptions { languages: Vec<String> /* empty = auto-detect */, vocabulary: Vec<String> }
// Transcript { text, language: Option<String> }
// SttError: NoSpeech | Unavailable(String) | Failed(String) | Cancelled
```

`async-trait` is required: `async fn` in traits is not dyn-compatible (E0038) on Rust 1.99,
and engines are chosen at runtime as `Arc<dyn SttEngine>`.

| Engine                        | Details                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| LocalWhisper (default, built) | whisper-rs 0.16 (whisper.cpp 1.8.3) with the `metal` feature and flash attention. One `WhisperContext` is loaded at startup and kept; a `WhisperState` runs each dictation with `FullParams` (greedy decoding, the language, `set_initial_prompt` with dictionary terms, `set_no_timestamps`, `set_suppress_blank`, `set_suppress_nst`, `set_no_context`). With a known language, the encoder context is sized to the clip (`audio_ctx` = 50 frames per second + 256, at least 768; the full 1500 near 30 s). Much below that, Large v3 Turbo repeats itself on short clips, and language detection needs the full window or it misreads the language. Until the language settings (P4), the language is the first macOS preferred language. Inference runs on the blocking pool. |
| OpenAI (P2)                   | `POST https://api.openai.com/v1/audio/transcriptions` (multipart: `file`, `model`). Default model `gpt-transcribe`, which takes `languages[]` and `keywords[]`. `whisper-1` and `gpt-4o-*-transcribe` are deprecated (removal 2027-02-26). 25 MB upload limit.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Groq (P2)                     | `POST https://api.groq.com/openai/v1/audio/transcriptions`, same multipart shape. Model `whisper-large-v3-turbo`. Each request is billed as at least 10 s.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

Model management (`stt/models.rs`, P1):

- Models are ggml files from `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-<name>.bin`.
  The catalog pins each file's size and SHA-256, taken from the Hugging Face API.
- Downloads stream to a temporary file with progress events and are renamed only after the
  hash matches.
- The default is `large-v3-turbo-q5_0` (574 MB). Measured on an M4 Mac mini with the
  3.69 s fixture (`tests/whisper_local.rs`): 0.52 s with the language set, 2.07 s with
  auto-detect (full encoder window); `small-q5_1` takes 0.11 s and 0.30 s. The turbo model
  stays the default. The catalog (`stt/models.rs`) lists tiny, base, small, medium and
  large-v3-turbo, each with its pinned size and SHA-256; switching is ENG-08 (P4).
- On first launch `ModelManager` downloads the VAD model and the default model in the
  background, reports progress through `models://status` (the main window shows it), then
  loads the engine into `EngineSlot`. Until then a dictation's audio is kept and the notice
  says why; paste last transcript retries it.
- whisper.cpp uses an optional Core ML encoder (`*-encoder.mlmodelc`) automatically when
  one sits next to the model. Shipping it is a phase 6 optimisation.
- Cloud uploads encode the clip as 16-bit WAV in memory (about 32 KB per second). Dictations
  longer than the 25 MB limit (about 13 minutes) are split at silence boundaries.

## LLM providers and the edit pipeline

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> LlmProviderId;                    // Anthropic | OpenAi | Groq | Ollama
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError>;
}
// LlmRequest { system, messages: Vec<Message { role, content }>, max_output_tokens }
// LlmResponse { text }
// LlmError: Unavailable(String) | Failed(String) | Cancelled
```

The request carries no sampling settings, because models differ in what they accept. Each
provider is constructed with its model and endpoint from settings and an API key from
`secrets/`.

| Provider            | Endpoint and auth                                                                                  | Request shape (P2)                                                                              | Text is in                                                                 |
| ------------------- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Anthropic (default) | `POST https://api.anthropic.com/v1/messages`; headers `x-api-key`, `anthropic-version: 2023-06-01` | `model`, `max_tokens`, `system`, `messages`, `output_config: {effort: "low"}`; no `temperature` | `content[]` blocks with `type == "text"` (not necessarily the first block) |
| OpenAI              | `POST https://api.openai.com/v1/responses`; `Authorization: Bearer`                                | `model`, `instructions`, `input`, `reasoning: {effort: "none"}`, `store: false`                 | `output[]` message → `content[]` with `type == "output_text"`              |
| Groq                | `POST https://api.groq.com/openai/v1/chat/completions`; Bearer                                     | `model` (`openai/gpt-oss-20b`), `messages`, `include_reasoning: false`                          | `choices[0].message.content`                                               |
| Ollama              | `POST http://localhost:11434/api/chat`; no auth                                                    | `model` (from `GET /api/tags`), `messages`, `stream: false`, `think: false`                     | `message.content`                                                          |

Anthropic has no official Rust SDK, so the provider uses reqwest against the documented REST
API. The default Anthropic model is `claude-opus-5-5`: thinking cannot be turned off on it,
so the provider sets `effort` to `low`. Settings also offer `claude-sonnet-5-5` and
`claude-haiku-5-5` (Haiku rejects non-default sampling parameters). Phase 2 measures cleanup
latency per model and reports it before any default changes. OpenAI and Groq default models
are picked from their current model lists in phase 2; at research time the cheap low-latency
options were `gpt-5.4-nano` and `openai/gpt-oss-20b`.

Edit pipeline (`edit/pipeline.rs`, P2), given the transcript, captured context and settings:

1. **Pre-pass (deterministic).** Spoken line and paragraph commands (FMT-06). Snippet
   triggers (SNP-02) and dictionary replacements (DIC-03) are swapped for placeholder tokens
   that the prompt tells the model to keep unchanged.
2. **LLM pass.** The system prompt is built from the cleanup level (FMT-01), the app category
   and its style (STY-*), the dictionary terms, the language and, when Context awareness is
   on, the app name and the text around the cursor. Output is plain text with markdown lists,
   which become rich text for pasting (INS-08). Skipped at cleanup level None, for AI-app
   targets (FMT-09) and in privacy mode when the provider is a cloud one.
3. **Post-pass (deterministic).** Placeholders are expanded. The style's casing and
   trailing-period rules (FMT-07) and the context casing and spacing (FMT-11) are applied.
4. **Fallback.** On an LLM error or timeout (about 4 s), the raw transcript goes through the
   pre- and post-passes and is inserted, and the history row is marked unformatted (FMT-10).

Command Mode and Transforms (`edit/command.rs`, P3) send the instruction and the selection
(or the field text) and paste the model's output verbatim, replacing the selection.

## Storage

All files live in Tauri's `app_data_dir()`:
`~/Library/Application Support/com.shrimp.desktop/` (dev: `com.shrimp.desktop.dev`) and
`%APPDATA%\com.shrimp.desktop\` on Windows.

| Path            | Contents                                                  |
| --------------- | --------------------------------------------------------- |
| `shrimp.db`     | SQLite in WAL mode: history, dictionary, snippets, stats. |
| `settings.json` | Settings (tauri-plugin-store).                            |
| `models/`       | Whisper and VAD model files.                              |
| `recordings/`   | WAV per history entry, kept 14 days.                      |
| `backups/`      | One daily `VACUUM INTO` copy of the database (HIS-08).    |

### SQLite

rusqlite 0.40 (`bundled`, so SQLite and FTS5 are compiled in) with rusqlite_migration 2.6
(schema version kept in `PRAGMA user_version`, migrations as `M::up` SQL with a
`Migrations::validate()` test). One `Connection` lives behind a `Mutex` in managed state and
is used from the blocking pool. tauri-plugin-sql is not used: it would expose SQL to the
webview, and the data is written by the Rust pipeline anyway.

Initial schema (each table arrives with the phase that needs it):

```sql
CREATE TABLE history (                                  -- P4
  id            INTEGER PRIMARY KEY,
  created_at    INTEGER NOT NULL,          -- unix ms
  mode          TEXT    NOT NULL,          -- 'dictation' | 'command' | 'transform'
  status        TEXT    NOT NULL,          -- 'inserted' | 'copied' | 'failed' | 'cancelled'
  app_id        TEXT,                      -- bundle id / exe name
  app_name      TEXT,
  raw_text      TEXT    NOT NULL,          -- STT output
  final_text    TEXT    NOT NULL,          -- what was inserted
  formatted     INTEGER NOT NULL,          -- 0 when AI edits were skipped or failed
  language      TEXT,
  stt_engine    TEXT    NOT NULL,
  llm_provider  TEXT,
  audio_ms      INTEGER NOT NULL,
  stt_ms        INTEGER,
  llm_ms        INTEGER,
  word_count    INTEGER NOT NULL,
  audio_path    TEXT                       -- NULL once the recording is deleted
);
CREATE INDEX history_created_at ON history(created_at DESC);
CREATE VIRTUAL TABLE history_fts USING fts5(final_text, content='history', content_rowid='id');  -- P4

CREATE TABLE dictionary (                               -- P2
  id          INTEGER PRIMARY KEY,
  term        TEXT    NOT NULL,            -- correct spelling
  misspelling TEXT,                        -- set for replacement entries
  source      TEXT    NOT NULL,            -- 'manual' | 'learned' | 'imported'
  pinned      INTEGER NOT NULL DEFAULT 0,
  uses        INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL
);
CREATE UNIQUE INDEX dictionary_term ON dictionary(term COLLATE NOCASE, misspelling COLLATE NOCASE);
CREATE TABLE dictionary_rejected (term TEXT PRIMARY KEY COLLATE NOCASE);  -- auto-learn never re-adds (DIC-07)

CREATE TABLE snippets (                                 -- P2
  id         INTEGER PRIMARY KEY,
  trigger    TEXT    NOT NULL UNIQUE COLLATE NOCASE,
  expansion  TEXT    NOT NULL,             -- markdown subset for rich text
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE daily_stats (                              -- P4; survives history deletion (STA-05)
  day         TEXT PRIMARY KEY,            -- local date, YYYY-MM-DD
  dictations  INTEGER NOT NULL,
  words       INTEGER NOT NULL,
  audio_ms    INTEGER NOT NULL,
  fixes       INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE app_usage (                                -- P4
  day TEXT NOT NULL, app_id TEXT NOT NULL, words INTEGER NOT NULL,
  PRIMARY KEY (day, app_id)
);
```

### Settings (P2)

Phase 1 has nothing user-configurable yet (the shortcuts and model are fixed defaults), so
settings arrive with the AI edit options in phase 2. A typed `Settings` struct in Rust (serde with `#[serde(default)]` on every section, plus a
`version` field for migrations) is the only source of truth. It is stored as one JSON value
under the key `settings` in `settings.json` through tauri-plugin-store 2.5 (`app.store(...)`,
`set`, `save`). The webview never writes the store directly. It calls `get_settings` and
`update_settings(patch)`; Rust validates the patch, saves it and emits `settings://changed`.

Sections: shortcuts, microphone, languages, stt (engine, local model, cloud models), ai
(provider, model per provider, cleanup level, Ollama URL), styles, sounds, media (off, mute
or pause), flowBar, privacy (privacy mode, context awareness, retention), system (launch at
login, Dock icon, UI language), notifications, experimental, vibeCoding, onboarding.

### Secrets

API keys are stored with keyring 4 (`keyring::Entry::new(service, account)`, `set_password`,
`get_password`, `delete_credential`; `Error::NoEntry` means not set). The service is the app
identifier (`com.shrimp.desktop`, or `.dev` for dev builds) and the account is the provider
(`anthropic`, `openai`, `groq`). Keys are read when a provider is constructed, are never
logged or sent to the webview, and the UI only learns whether a key is set.

## Errors, logging and notices

- Each domain module has its own `thiserror` enum (`AudioError`, `SttError`, `LlmError`,
  `PlatformError`, `StorageError`). Commands map them to a serialisable `CommandError
{ code, message }` for the UI.
- User-facing failures go through `notify/` as notices next to the Flow Bar. They never
  appear as modal dialogs (the MCP bridge also stubs `alert`, and modals would block it).
- Logging uses the `log` crate. Debug builds route it into tauri-plugin-mcp's buffer
  (`capture_rust_logs`), readable with `query_logs`. Release builds add a file logger in
  phase 4. Logs never contain transcripts, field text or keys at `info` level or above.

## Security and privacy

- Capabilities grant the webview only `core:default` and the MCP plugin's log-forwarding
  commands, which exist only in debug builds. Domain operations go through typed commands.
- The CSP restricts the webview to the app's own assets and Tauri IPC.
- Secure text fields are never read (CTX-03). With Context awareness off, no field text
  leaves the device (CTX-04). Privacy mode blocks every cloud engine and provider (PRIV-03).
- Dev builds are signed with a local self-signed identity; release builds with Developer
  ID and notarization (P5). See [DEVELOPMENT.md](DEVELOPMENT.md#stable-code-signing-identity).

## Testing

| Layer           | How                                                                                                                                                                                                                   |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pure Rust logic | Unit tests next to the code: audio conversion, spec parsing, gestures, binding validation, snippet and dictionary matching, style rules, migrations.                                                                  |
| Pipelines       | Integration tests in `src-tauri/tests/`: WAV fixture → `Recorder` → engine, the full controller with fake platform services (`controller.rs`), and model downloads against a local HTTP server (`model_download.rs`). |
| Local Whisper   | `#[ignore]` tests that need a downloaded model (`whisper_local.rs`), run with `cargo test -- --ignored`.                                                                                                              |
| Providers       | Request/response mapping tested against recorded JSON; live calls only in opt-in tests.                                                                                                                               |
| UI              | Vitest + Testing Library with `mockIPC`.                                                                                                                                                                              |
| Real app        | tauri-mcp against the signed dev bundle, with `SHRIMP_AUDIO_SOURCE=wav:<path>`.                                                                                                                                       |

`pnpm check` runs formatting, oxlint, `tsc`, Vitest, clippy with `-D warnings` and cargo
test.

## Dependencies

Versions as of 2026-10-13. Rust toolchain pinned to 1.99.0 (`rust-toolchain.toml`); Tauri 2.12
needs 1.90 or later.

| Crate / package                                                                                                                             | Version           | Phase | Purpose and notes                                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| tauri / tauri-build                                                                                                                         | 2.12 / 2.7        | built | App framework; `tray-icon` feature. The `@tauri-apps/api` npm package must stay on the same minor.                                                                                                                             |
| tauri-plugin-single-instance                                                                                                                | 2.5               | built | Second launch focuses the running app.                                                                                                                                                                                         |
| tauri-plugin-mcp (git `c7d271a`)                                                                                                            | 0.3.1             | built | Dev-only automation bridge. npm `tauri-plugin-mcp` 0.3.1 and `tauri-plugin-mcp-server` 0.3.1.                                                                                                                                  |
| hound                                                                                                                                       | 3.5.1             | built | WAV read/write.                                                                                                                                                                                                                |
| rubato                                                                                                                                      | 5.0.1             | built | Resampling to 16 kHz (`Fft` + `process_all`).                                                                                                                                                                                  |
| async-trait                                                                                                                                 | 0.1               | built | dyn-compatible async traits for engines and providers.                                                                                                                                                                         |
| thiserror, serde, serde_json, log                                                                                                           | 2 / 1 / 1 / 0.4   | built | Errors, serialisation, logging.                                                                                                                                                                                                |
| cpal                                                                                                                                        | 0.18.2            | built | Microphone capture (CoreAudio, WASAPI). Needs macOS 14.2+, hence the bundle minimum.                                                                                                                                           |
| whisper-rs                                                                                                                                  | 0.16.0            | built | whisper.cpp 1.8.3 binding; `metal` feature on macOS. Needs CMake and Xcode/libclang to build. whisper.cpp is built optimised even in debug builds.                                                                             |
| tauri-nspanel                                                                                                                               | 2.1.1             | built | Non-activating `NSPanel` for the Flow Bar (crates.io release). Needs Tauri's `macos-private-api` feature.                                                                                                                      |
| rusqlite (`bundled`) + rusqlite_migration                                                                                                   | 0.40.2 + 2.6.0    | P2    | SQLite storage and migrations (dictionary and snippets in P2, history in P4).                                                                                                                                                  |
| tauri-plugin-store                                                                                                                          | 2.5.0             | P2    | Settings persistence.                                                                                                                                                                                                          |
| reqwest                                                                                                                                     | 0.13.5            | built | Model downloads (default features, rustls; `Response::chunk` streams without the `stream` feature); cloud STT and LLM calls (P2).                                                                                              |
| sha2                                                                                                                                        | 0.10              | built | Model checksum verification.                                                                                                                                                                                                   |
| tokio                                                                                                                                       | 1                 | built | Channels and timers in the controller, on Tauri's runtime. Features `sync`, `time`, `macros`, `rt`.                                                                                                                            |
| keyring                                                                                                                                     | 4.2.0             | P2    | OS keychain (macOS Keychain, Windows Credential Manager) via the default `v1` feature.                                                                                                                                         |
| tauri-plugin-global-shortcut                                                                                                                | 2.4.1             | P4    | Keyed shortcuts while Secure Keyboard Entry is on (cannot bind Fn).                                                                                                                                                            |
| handy-keys                                                                                                                                  | 0.3.4             | built | CGEventTap / `WH_KEYBOARD_LL` key events with Fn, modifier-only chords, press and release, and blocking (MIT; used by Handy).                                                                                                  |
| objc2, block2, objc2-foundation, objc2-core-foundation, objc2-core-graphics, objc2-app-kit, objc2-application-services, objc2-av-foundation | 0.6 / 0.6 / 0.3.2 | built | Event posting, pasteboard, frontmost app, Accessibility, permissions, `NSSound`, `NSLocale`, preferences. Features per API, `default-features = false`. Text Input Sources and `UCKeyTranslate` are declared by hand (Carbon). |
| rodio                                                                                                                                       | 0.23.0            | —     | Not used: `NSSound` plays the cues without keeping an output stream open.                                                                                                                                                      |
| objc2-service-management                                                                                                                    | 0.3.2             | P4    | Launch at login on macOS (`SMAppService`).                                                                                                                                                                                     |
| tauri-plugin-autostart                                                                                                                      | 2.7.0             | P5    | Launch at login on Windows.                                                                                                                                                                                                    |
| windows                                                                                                                                     | 0.62.2            | P5    | Win32 hooks, `SendInput`, foreground window, audio sessions, SMTC (`Media_Control`).                                                                                                                                           |
| clipboard-rs                                                                                                                                | 0.3.5             | P5    | Multi-format clipboard snapshot and restore on Windows.                                                                                                                                                                        |
| uiautomation                                                                                                                                | 0.25.1            | P5    | UI Automation text access on Windows.                                                                                                                                                                                          |

Prior art: [Handy](https://github.com/cjpais/Handy) (MIT), an open-source Tauri v2 dictation
app, uses the same building blocks: cpal and rubato, a CGEventTap hotkey crate (`handy-keys`)
for Fn support, tauri-nspanel for the overlay, clipboard paste with restore, and rusqlite.
Unlike Handy, which keeps API keys in its settings store, Shrimp keeps them in the OS keychain.

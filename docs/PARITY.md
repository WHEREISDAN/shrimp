# Wispr Flow parity checklist

This file is the single source of truth for what "done" means for Shrimp and how far along it
is. Every item has:

- a status box: `[ ]` open, `[x]` accepted;
- an ID;
- a one-line acceptance criterion;
- the phase that delivers it.

Tick an item only when its criterion has been verified in the real app (tauri-mcp, a test or
a recorded manual check). Note how it was verified in the commit that ticks it.

The baseline is Wispr Flow's desktop app as documented on 2026-10-13: the help center, the
changelog at wisprflow.ai/whats-new and the product pages. Sources are listed at the end of
this file.

## Phases

| Phase | Scope                                                                                                   |
| ----- | ------------------------------------------------------------------------------------------------------- |
| 1     | Core dictation loop: hotkey, capture, STT, paste, Flow Bar, tray, sounds.                               |
| 2     | AI edits: cleanup, styles, dictionary, snippets; LLM providers, cloud STT, keychain.                    |
| 3     | Command Mode, Transforms, context awareness, coding/IDE features.                                       |
| 4     | History, insights, full settings, languages, whisper mode, media, launch at login, privacy, onboarding. |
| 5     | Windows parity, packaging (DMG, Windows installer) and updates.                                         |
| 6     | Parity audit and full regression.                                                                       |

## Rules

- Phases 1–4 are accepted on macOS. Phase 5 re-verifies every phase 1–4 item on Windows
  (`WIN-01`) and adds the Windows-only items.
- Shortcut names below are the macOS defaults. The Windows defaults are in `WIN-02`.
- Where Wispr's own documents contradict each other, the criterion states the behaviour
  Shrimp adopts.
- Wispr transcribes in the cloud. Shrimp transcribes locally by default and offers cloud
  engines as options (`ENG-*`), as the owner decided.

## Engines and providers

| Status | ID     | Feature          | Acceptance criterion                                                                                                                                 | Phase |
| ------ | ------ | ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | ENG-01 | Local Whisper    | whisper.cpp (whisper-rs, Metal on Apple Silicon) transcribes the `hello-shrimp.wav` fixture offline with the default model.                          | 1     |
| [ ]    | ENG-02 | Model download   | On first use, the default model downloads with visible progress, is checked against its pinned SHA-256 and is reused afterwards.                     | 1     |
| [ ]    | ENG-03 | Silence handling | A silence-only recording produces no text (no hallucinated words), and leading and trailing silence is trimmed before STT.                           | 1     |
| [ ]    | ENG-04 | First words kept | The first word of a dictation is never cut off: audio from the key press onwards is captured.                                                        | 1     |
| [ ]    | ENG-05 | Cloud STT        | Groq and OpenAI transcription engines work with the user's own API key and can be selected in settings.                                              | 2     |
| [ ]    | ENG-06 | LLM providers    | Anthropic (the default), OpenAI, Groq and local Ollama each complete a cleanup request; provider and model are selectable.                           | 2     |
| [ ]    | ENG-07 | Provider errors  | A missing key, no network, a rate limit or a timeout shows a clear notice, and the raw transcript is inserted instead.                               | 2     |
| [ ]    | ENG-08 | Model picker     | Settings lists the Whisper models (tiny to large-v3-turbo) with size, downloaded state and delete, and switching takes effect on the next dictation. | 4     |

## Core dictation

| Status | ID      | Feature               | Acceptance criterion                                                                                                                          | Phase |
| ------ | ------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | CORE-01 | Push to talk          | Holding the push-to-talk shortcut records; releasing it transcribes and inserts the text at the cursor of the focused app.                    | 1     |
| [ ]    | CORE-02 | Fn default            | Fn alone is the default push-to-talk shortcut, and its system action (emoji picker or dictation) is suppressed while bound.                   | 1     |
| [ ]    | CORE-03 | Short-tap guard       | A press released in under 0.5 s inserts nothing: Shrimp waits briefly for a second press, then cancels and shows the "hold the shortcut" tip. | 1     |
| [ ]    | CORE-04 | Hands-free shortcut   | Fn+Space starts recording without holding; pressing it again stops and inserts.                                                               | 1     |
| [ ]    | CORE-05 | Hands-free double-tap | Double-pressing push to talk within 0.5 s locks recording hands-free; a third quick press cancels.                                            | 1     |
| [ ]    | CORE-06 | Cancel                | Esc cancels an active dictation without inserting anything, and the Esc press does not reach the focused app.                                 | 1     |
| [ ]    | CORE-07 | Busy guard            | Shortcuts pressed while a dictation is still processing are rejected (not queued) with a "Transcript currently processing" notice.            | 1     |
| [ ]    | CORE-08 | Session limit         | A warning sound and notice come at 19 minutes; at 20 minutes recording stops, then transcribes and inserts.                                   | 1     |
| [ ]    | CORE-09 | Click to dictate      | Clicking the Flow Bar starts hands-free recording, ■ stops and inserts, ✕ cancels, and a click within 0.5 s of starting cancels.              | 1     |
| [ ]    | CORE-10 | No-Fn fallback        | On a Mac keyboard without an Apple Fn key, the defaults become Ctrl+Opt, Ctrl+Opt+Space and Cmd+Ctrl+Opt (Command Mode).                      | 4     |
| [ ]    | CORE-11 | Cancel undo           | The cancel notice offers Undo, which transcribes the cancelled audio and inserts it into the current field.                                   | 4     |
| [ ]    | CORE-12 | Interrupting shortcut | Pressing another app's shortcut within the first second of a held dictation cancels it with a notification.                                   | 4     |
| [ ]    | CORE-13 | Mouse buttons         | Middle, Mouse 4/5 and Mouse 6–10 can be bound to push to talk, hands-free or Enter, and bound buttons don't pass through to apps.             | 4     |
| [ ]    | CORE-14 | Stuck-key recovery    | A key or mouse button left in a stuck "down" state recovers automatically after 10 s.                                                         | 4     |
| [ ]    | CORE-15 | Press Enter shortcut  | An unbound-by-default "Press Enter" shortcut sends Enter, and extra modifiers pass through (Shift+Enter).                                     | 3     |
| [ ]    | CORE-16 | "press enter" command | With the experimental setting on, ending a dictation with "press enter" removes the phrase and presses Enter after a successful paste.        | 3     |

## Shortcut customisation

| Status | ID      | Feature            | Acceptance criterion                                                                                                                                               | Phase |
| ------ | ------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| [ ]    | KEYS-01 | Shortcuts dialog   | Settings and the tray open a dialog listing every action; clicking a binding records a replacement (keys save on release, mouse buttons on press); Done closes it. | 4     |
| [ ]    | KEYS-02 | Multiple bindings  | Each action takes up to 4 bindings (Cancel takes 1); a trash icon removes one; required actions keep at least one.                                                 | 4     |
| [ ]    | KEYS-03 | Validation         | Combinations need a modifier or a mouse button, at most 3 keys, no duplicates across actions and no left+right pair of one modifier, with Wispr's error texts.     | 4     |
| [ ]    | KEYS-04 | Standalone keys    | Caps Lock alone is allowed for push to talk or hands-free (and stops toggling caps); Esc alone only for Cancel.                                                    | 4     |
| [ ]    | KEYS-05 | Reserved shortcuts | Cmd+Backspace, Cmd+Delete, Cmd+Space, Cmd+Ctrl+F, Cmd+C and Cmd+V are rejected with "This shortcut is not allowed".                                                | 4     |
| [ ]    | KEYS-06 | Recording cancel   | While a shortcut is being recorded, Esc alone, a mouse click or ✕ cancels without a change; pressing Ctrl then Esc records Ctrl+Esc.                               | 4     |
| [ ]    | KEYS-07 | Overlap warning    | A hands-free combination contained in push to talk saves with a conflict warning.                                                                                  | 4     |
| [ ]    | KEYS-08 | Reset to defaults  | Reset restores the platform defaults, including clearing unbound-by-default actions, after a confirmation.                                                         | 4     |
| [ ]    | KEYS-09 | Secure input       | When another app turns on Secure Keyboard Entry, Fn-only holds keep working and Shrimp explains why key combinations are blocked.                                  | 4     |
| [ ]    | KEYS-10 | Persistence        | Shortcuts are stored per device and survive restarts and updates.                                                                                                  | 4     |

## Text insertion and clipboard

| Status | ID     | Feature               | Acceptance criterion                                                                                                                                       | Phase |
| ------ | ------ | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | INS-01 | Paste at cursor       | Text is inserted through the clipboard and Cmd+V into the focused field of native, browser and Electron apps.                                              | 1     |
| [ ]    | INS-02 | Clipboard restore     | The previous clipboard (text, rich text, images) is restored about 0.5 s after the paste; something copied during that window is kept.                     | 1     |
| [ ]    | INS-03 | Keyboard layouts      | Pasting works on non-QWERTY layouts (Dvorak, AZERTY) because Shrimp resolves which key types "v".                                                          | 1     |
| [ ]    | INS-04 | Failed insertion      | If no editable field is focused or the paste fails, the text stays on the clipboard and a notification offers Copy.                                        | 1     |
| [ ]    | INS-05 | Paste last transcript | Cmd+Ctrl+V pastes the last transcript, retrying a failed or cancelled one first, and does nothing when there is none.                                      | 1     |
| [ ]    | INS-06 | Copy last transcript  | Cmd+Ctrl+C copies the last transcript to the clipboard.                                                                                                    | 1     |
| [ ]    | INS-07 | Undo in target app    | Cmd+Z in the target app removes the inserted text in one step.                                                                                             | 1     |
| [ ]    | INS-08 | Rich text output      | One spoken line break gives a new line and two give a new paragraph; rich text pastes where the field accepts it, and plain text where it doesn't.         | 2     |
| [ ]    | INS-09 | AI coding CLIs        | In Claude Code and Codex CLI, long dictations paste in parts, so they don't collapse into "[Pasted N lines]".                                              | 3     |
| [ ]    | INS-10 | Remote desktops       | In remote-desktop clients (Windows App, Citrix, Horizon, VNC and Screen Sharing), the clipboard is held about 5 s and no context-based spacing is applied. | 3     |
| [ ]    | INS-11 | Terminals             | Pasting works in Terminal.app, iTerm2, Warp, Ghostty, kitty, Alacritty and IDE terminals.                                                                  | 3     |

## Formatting and cleanup

| Status | ID     | Feature                | Acceptance criterion                                                                                                                                                                                                                    | Phase |
| ------ | ------ | ---------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | FMT-01 | Auto Cleanup levels    | Style offers None (raw transcript), Light (fillers and grammar; the default) and Medium (rewording for clarity), applied from the next dictation.                                                                                       | 2     |
| [ ]    | FMT-02 | Filler removal         | "um", "uh", false starts and repetitions are removed at Light and above.                                                                                                                                                                | 2     |
| [ ]    | FMT-03 | Backtrack              | Clear self-corrections ("coffee at 2, actually 3", "scratch that", "never mind") keep only the corrected version.                                                                                                                       | 2     |
| [ ]    | FMT-04 | Lists                  | Spoken enumerations ("one… two…", "first… second…") become a formatted list.                                                                                                                                                            | 2     |
| [ ]    | FMT-05 | Spoken punctuation     | "period" / "full stop", "comma", "question mark", "exclamation point", "colon", "semicolon" and "em dash" insert the symbol; Shrimp never adds an em dash on its own.                                                                   | 2     |
| [ ]    | FMT-06 | Line commands          | "new line", "next line", "line break", "skip a line" and "new paragraph" insert breaks whatever the other settings are.                                                                                                                 | 2     |
| [ ]    | FMT-07 | Trailing periods       | Trailing periods are stripped per style (Formal never, Casual up to 10 sentences, Very casual always, otherwise up to 2 in messaging apps), keeping `!` and `?`.                                                                        | 2     |
| [ ]    | FMT-08 | Email formatting       | Email-category apps get a greeting, body and sign-off separated by line breaks.                                                                                                                                                         | 2     |
| [ ]    | FMT-09 | AI apps unformatted    | Dictation into AI chat apps skips style formatting.                                                                                                                                                                                     | 2     |
| [ ]    | FMT-10 | Raw fallback           | If AI editing fails or times out, the raw transcript is inserted and the history entry is marked unformatted.                                                                                                                           | 2     |
| [ ]    | FMT-11 | Context casing/spacing | Mid-sentence insertions lowercase the first word and add a leading space (unless it is a proper noun or dictionary term), drop ending punctuation when the text after the cursor continues, and add no spaces for Chinese and Japanese. | 3     |
| [ ]    | FMT-12 | Undo/Redo AI edit      | History offers Undo AI edit (restores the raw text) and Redo AI edit whenever the two differ.                                                                                                                                           | 4     |
| [ ]    | FMT-13 | Formatting tips        | The first lists, emails and backtracks show one explanation and up to 3 reminders, which the Announcements toggle mutes.                                                                                                                | 4     |
| [ ]    | FMT-14 | French typography      | French output gets narrow no-break spaces before `; : ! ?`, except in developer apps.                                                                                                                                                   | 4     |

## Styles

| Status | ID     | Feature            | Acceptance criterion                                                                                                                                                | Phase |
| ------ | ------ | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | STY-01 | App categories     | The frontmost app maps to Personal messages, Work messages, Email or Other; AI assistants and terminals are Other.                                                  | 2     |
| [ ]    | STY-02 | Style options      | Formal, Casual, Very casual (Personal only) and Excited (Work, Email, Other) change only capitalisation, punctuation and spacing, never wording.                    | 2     |
| [ ]    | STY-03 | Style setup        | The Style page walks through each category; a choice saves on selection, and every category defaults to Formal.                                                     | 2     |
| [ ]    | STY-04 | Built-in app map   | Documented apps land in the right category (Messages, WhatsApp, Discord, Signal, Instagram: Personal; Slack, Teams, LinkedIn: Work; Mail, Outlook, Missive: Email). | 2     |
| [ ]    | STY-05 | Web apps by domain | In browsers, the active tab's domain decides the category (gmail.com: Email; app.slack.com: Work).                                                                  | 3     |

## Dictionary

| Status | ID     | Feature              | Acceptance criterion                                                                                                                                                                              | Phase |
| ------ | ------ | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | DIC-01 | Add word             | Dictionary > Add word saves a term of up to 60 characters; empty, punctuation-only and duplicate entries are rejected.                                                                            | 2     |
| [ ]    | DIC-02 | Vocabulary biasing   | Dictionary terms reach the STT engine (Whisper initial prompt, cloud prompt or keywords), and a fixture term that was misrecognised is spelled correctly.                                         | 2     |
| [ ]    | DIC-03 | Replacements         | "Correct a misspelling" replaces whole words, case-insensitively, longest match first, keeping the saved casing; "n't" contractions are left alone, possessives are fixed.                        | 2     |
| [ ]    | DIC-04 | Pin and usage order  | Terms can be pinned so they take priority, and the list sorts by usage.                                                                                                                           | 2     |
| [ ]    | DIC-05 | Search, edit, delete | Entries can be searched, edited and deleted, one at a time or in bulk (Cmd+Backspace on selected rows).                                                                                           | 2     |
| [ ]    | DIC-06 | Bulk import          | CSV import (`word` or `misspelling,correction`, up to 3 MB and 1,000 rows) shows a duplicate preview and marks rows "Imported from file".                                                         | 2     |
| [ ]    | DIC-07 | Auto-learn           | With "Auto-add to dictionary" on, words the user respells after a paste are added with a badge and a notification; paths, URLs, code, numbers, emoji, common words and deleted words are skipped. | 3     |

## Snippets

| Status | ID     | Feature      | Acceptance criterion                                                                                                                                                         | Phase |
| ------ | ------ | ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | SNP-01 | Create       | Snippets > Add saves a trigger (up to 60 characters) and an expansion (up to 4,000); Cmd+Enter saves.                                                                        | 2     |
| [ ]    | SNP-02 | Matching     | A trigger spoken alone ignores case, punctuation and spacing; inside a sentence it matches case-insensitively, never next to letters, digits or `_`; the longest match wins. | 2     |
| [ ]    | SNP-03 | Rich text    | Expansions keep bold, italic, lists and links (Cmd+K inserts a link) and fall back to plain text where the app rejects formatting.                                           | 2     |
| [ ]    | SNP-04 | Edit, delete | Snippets can be edited (Save changes) and deleted; a trigger similar to an existing one saves with a warning.                                                                | 2     |
| [ ]    | SNP-05 | Bulk import  | JSON import (`[{"name", "text"}]`, up to 3 MB and 1,000 entries) shows a duplicate preview.                                                                                  | 2     |

## Command Mode

| Status | ID     | Feature         | Acceptance criterion                                                                                                                                                                        | Phase |
| ------ | ------ | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | CMD-01 | Voice edits     | Holding Fn+Ctrl, speaking an instruction and releasing applies it to the selection (or the whole field) and pastes the result without dictation formatting.                                 | 3     |
| [ ]    | CMD-02 | Reading text    | The selection and field text are read through Accessibility; if that fails, the selection is copied instead.                                                                                | 3     |
| [ ]    | CMD-03 | Questions       | With no editable field, the answer appears in a notification panel instead of being pasted.                                                                                                 | 3     |
| [ ]    | CMD-04 | Lock and cancel | A double press within 500 ms locks Command Mode hands-free; Esc or Backspace cancels; releasing before listening starts cancels.                                                            | 3     |
| [ ]    | CMD-05 | Search commands | "Search …" instructions open a web search in the default browser with the selection appended.                                                                                               | 3     |
| [ ]    | CMD-06 | Enable toggle   | Command Mode is off by default and turned on under Settings > Experimental, with a one-time intro; pressing its shortcut while off shows "Command mode is toggled off" with Go to settings. | 3     |

## Transforms

| Status | ID     | Feature           | Acceptance criterion                                                                                                                                                | Phase |
| ------ | ------ | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | TRF-01 | Polish            | Opt+1 rewrites the selection (or the field, or the last dictation within 60 s) using five toggleable rules and up to 5 custom instructions.                         | 3     |
| [ ]    | TRF-02 | Prompt Engineer   | Opt+2 turns the selection into a structured AI prompt.                                                                                                              | 3     |
| [ ]    | TRF-03 | Custom transforms | Up to 8 custom transforms, each with a name, prompt, shortcut and up to 5 writing samples; edits save automatically.                                                | 3     |
| [ ]    | TRF-04 | Result and diff   | The result replaces the text in place; "Done. See changes" and Opt+O open a diff with Copy, Retry, Accept edits and Undo.                                           | 3     |
| [ ]    | TRF-05 | Auto apply        | A chosen transform can run after every dictation, falling back to the original text on failure.                                                                     | 3     |
| [ ]    | TRF-06 | Limits and errors | Input of 1–1,000 words is accepted; otherwise the documented errors appear ("Oops, too long to polish", "Select text to apply a transform", "Transform timed out"). | 3     |

## Context awareness

| Status | ID     | Feature    | Acceptance criterion                                                                                                                                     | Phase |
| ------ | ------ | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | CTX-01 | Capture    | When a dictation starts, Shrimp reads the app, the selection and the text before and after the cursor through Accessibility, without delaying recording. | 3     |
| [ ]    | CTX-02 | Use        | Context improves name spelling, casing and style (a name visible in the field is spelled as shown).                                                      | 3     |
| [ ]    | CTX-03 | Exclusions | Password and secure fields, URL bars, placeholder text and Shrimp's own windows are never read.                                                          | 3     |
| [ ]    | CTX-04 | Toggle     | Turning off Context awareness (Data and privacy) stops text context from reaching any provider; on-device formatting still uses it.                      | 3     |

## Coding and IDE

| Status | ID     | Feature                | Acceptance criterion                                                                                                                           | Phase |
| ------ | ------ | ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | DEV-01 | File tagging           | In Cursor and Windsurf chat, saying an open file's name (or "at"/"tag" plus the name) inserts an @-mention; Settings > Vibe coding toggles it. | 3     |
| [ ]    | DEV-02 | Variable recognition   | With the IDE's screen-reader mode on, spoken identifiers ("user ID") are written as the code identifier (`userId`).                            | 3     |
| [ ]    | DEV-03 | Syntax-aware dictation | camelCase, snake_case, acronyms and CLI commands are written correctly in developer apps.                                                      | 3     |

## Flow Bar

| Status | ID     | Feature      | Acceptance criterion                                                                                                                                         | Phase |
| ------ | ------ | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| [ ]    | BAR-01 | Pill window  | A floating pill shows resting, recording (live waveform) and processing states, never takes focus and stays above all windows, including full-screen Spaces. | 1     |
| [ ]    | BAR-02 | Controls     | Stop (■), cancel (✕) and confirm (✓) work while recording.                                                                                                   | 1     |
| [ ]    | BAR-03 | Notices      | Notices (warnings, errors, cancel and failed-paste messages) appear next to the bar without taking focus.                                                    | 1     |
| [ ]    | BAR-04 | Visibility   | "Show Flow Bar at all times" works, and right-click > Hide for 1 hour hides it with an Undo.                                                                 | 4     |
| [ ]    | BAR-05 | Position     | The bar can be dragged to the bottom, left or right edge, remembers its place, avoids the menu bar and Dock, and moves to the display under the cursor.      | 4     |
| [ ]    | BAR-06 | Context menu | Right-click offers Microphone, Paste last transcript, Transcript history, Settings and Hide for 1 hour.                                                      | 4     |

## Microphone

| Status | ID     | Feature              | Acceptance criterion                                                                                                                                                    | Phase |
| ------ | ------ | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | MIC-01 | Capture              | Shrimp records from the default input device at its native format and converts the audio to 16 kHz mono for STT.                                                        | 1     |
| [ ]    | MIC-02 | Picker               | Settings and the Flow Bar menu list Auto-detect, the built-in mic and named devices, with a live level meter for the selected one.                                      | 4     |
| [ ]    | MIC-03 | Ranking and fallback | Devices can be ranked and forgotten; Auto-detect uses the highest-ranked available device and falls back mid-dictation without a break.                                 | 4     |
| [ ]    | MIC-04 | New mic prompt       | Connecting a new microphone asks Switch or Don't switch (not for AirPods).                                                                                              | 4     |
| [ ]    | MIC-05 | Clamshell            | With the MacBook lid closed, the built-in mic is unavailable ("Laptop lid is closed") and an external mic is used.                                                      | 4     |
| [ ]    | MIC-06 | Virtual mics         | Virtual devices (BlackHole, Krisp and similar) are listed under "Show other devices" and never selected automatically.                                                  | 4     |
| [ ]    | MIC-07 | Mic warnings         | Warnings appear for no sound at start, no audio after 5 s and 15 s, a mic taking over 8 s to open, and missing, unavailable or disconnected mics, each with its action. | 4     |
| [ ]    | MIC-08 | Bluetooth advice     | Using AirPods or another Bluetooth mic shows a warning with Switch microphone and Don't show again (at most 3 times, at most weekly).                                   | 4     |
| [ ]    | MIC-09 | Whisper mode         | Whispered speech transcribes reliably (input level normalisation, tuned speech detection), checked with a whispered fixture.                                            | 4     |
| [ ]    | MIC-10 | Mic advice           | A "Find the right mic for you" walkthrough recommends a microphone setup.                                                                                               | 4     |

## Sounds and media

| Status | ID     | Feature              | Acceptance criterion                                                                                                                              | Phase |
| ------ | ------ | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | SND-01 | Dictation sounds     | Distinct start, stop, lock (hands-free) and cancel sounds play at the matching moments.                                                           | 1     |
| [ ]    | SND-02 | Sound settings       | Settings > Sound turns dictation and notification sounds on or off.                                                                               | 4     |
| [ ]    | SND-03 | Mute while dictating | If audio is playing when a dictation starts, output is muted within about 300 ms and restored afterwards; the setting is off by default on macOS. | 4     |
| [ ]    | SND-04 | Pause media          | "Pause music while dictating" pauses supported players and resumes them afterwards, ignores conferencing apps, and excludes the mute option.      | 4     |

## History

| Status | ID     | Feature             | Acceptance criterion                                                                                                                                                    | Phase |
| ------ | ------ | ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | HIS-01 | Recent activity     | Home lists dictations grouped by date, with the detected language; clicking a row or pressing Enter copies it.                                                          | 4     |
| [ ]    | HIS-02 | Search and keys     | Cmd+F or / searches history; ↑/↓ or j/k move between rows.                                                                                                              | 4     |
| [ ]    | HIS-03 | Row menu            | Each row offers Retry transcript, Undo/Redo AI edit, Play audio, Extract audio (.wav) and Delete transcript.                                                            | 4     |
| [ ]    | HIS-04 | Audio retention     | Recordings are kept 14 days for retry and playback, then deleted.                                                                                                       | 4     |
| [ ]    | HIS-05 | Retry               | Failed or empty entries with at least 5 s of audio under 14 days old can be transcribed again; failures show the documented notices.                                    | 4     |
| [ ]    | HIS-06 | Recover after crash | Audio from a dictation interrupted by quitting or a crash is saved and can be recovered from History.                                                                   | 4     |
| [ ]    | HIS-07 | Local retention     | Data and privacy offers Store locally (default), Auto-delete every 24 hours, and Never store (confirmed by typing "delete all"; History then reads "History disabled"). | 4     |
| [ ]    | HIS-08 | Daily backup        | The local database is backed up once a day.                                                                                                                             | 4     |

## Insights

| Status | ID     | Feature       | Acceptance criterion                                                                                                                                                    | Phase |
| ------ | ------ | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | STA-01 | Usage stats   | Insights shows WPM over the last 100 dictations, total words, fixes made, dictionary fixes, current and longest streak, an activity heatmap, apps used and the top app. | 4     |
| [ ]    | STA-02 | Home cards    | Home shows a greeting with the push-to-talk shortcut, a stats card that opens Insights, and the week's word count.                                                      | 4     |
| [ ]    | STA-03 | Share card    | A stats card can be copied or saved as an image.                                                                                                                        | 4     |
| [ ]    | STA-04 | Voice profile | After 2,000 words, an LLM-generated voice profile (archetype, catchphrase, most-corrected word, active times) can be generated and regenerated at milestones.           | 4     |
| [ ]    | STA-05 | Stats survive | Stats keep counting when history is auto-deleted or disabled.                                                                                                           | 4     |

## Languages

| Status | ID      | Feature                 | Acceptance criterion                                                                                                                                                  | Phase |
| ------ | ------- | ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | LANG-01 | Language picker         | Settings > Languages offers Auto-detect or a searchable multi-select; saving with neither shows "Please select at least one language or turn on auto-detect".         | 4     |
| [ ]    | LANG-02 | Auto-detect             | With Auto-detect on, or several languages selected, each dictation is transcribed in its detected language.                                                           | 4     |
| [ ]    | LANG-03 | Variants                | English (US, British, Canadian), Chinese (Simplified, Traditional), Cantonese and Swiss German variants shape the output; choosing one of a pair deselects the other. | 4     |
| [ ]    | LANG-04 | Different-language card | Speech in a language that isn't selected shows "Speaking a different language?" at most once a day, with Open language settings, Add language and Don't show again.   | 4     |
| [ ]    | LANG-05 | Flow Bar switching      | With several languages enabled, hovering the Flow Bar offers one-click language switching.                                                                            | 4     |
| [ ]    | LANG-06 | App UI language         | The app's interface is available in English, German, Spanish, Italian and Portuguese.                                                                                 | 4     |

## Notifications

| Status | ID     | Feature    | Acceptance criterion                                                                                                      | Phase |
| ------ | ------ | ---------- | ------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | NOT-01 | Categories | Settings > Notifications toggles Suggestions, Announcements and Milestones; error and permission notices cannot be muted. | 4     |
| [ ]    | NOT-02 | Behaviour  | Error notices interrupt and others queue; notices never take focus, and action buttons confirm with a ✓.                  | 4     |

## Privacy and data

| Status | ID      | Feature          | Acceptance criterion                                                                                                                      | Phase |
| ------ | ------- | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | PRIV-01 | Local by default | With the default settings (local Whisper, no API key), no audio or text leaves the device, verified with a network monitor.               | 1     |
| [ ]    | PRIV-02 | Keychain         | API keys are stored only in the OS keychain, never in settings files, the database or logs.                                               | 2     |
| [ ]    | PRIV-03 | Privacy mode     | Privacy mode turns off every cloud engine and provider (only local Whisper and Ollama run), and the Flow Bar and tray show that it is on. | 4     |

## App shell and settings

| Status | ID     | Feature         | Acceptance criterion                                                                                                                                             | Phase |
| ------ | ------ | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [x]    | APP-01 | Close to tray   | Closing the main window keeps Shrimp running in the menu bar; "Open Shrimp" brings the window back and Quit exits.                                               | 1     |
| [x]    | APP-02 | Single instance | Launching Shrimp again focuses the running instance instead of starting a second one.                                                                            | 1     |
| [ ]    | APP-03 | Tray menu       | The menu-bar menu offers Open Shrimp, Shortcuts, Paste last transcript, Copy last transcript and Quit.                                                           | 1     |
| [ ]    | APP-04 | Main window     | The main window's sidebar has Home, Insights, Dictionary, Snippets, Style, Transforms, Settings and Help; Opt+↑/↓ moves between settings pages.                  | 4     |
| [ ]    | APP-05 | Settings pages  | Settings has General (shortcuts, microphone, languages, app language), System, Data and privacy, Speech engine, AI provider, Experimental and Vibe coding pages. | 4     |
| [ ]    | APP-06 | Launch at login | When turned on, Shrimp starts at login in the background without opening the main window.                                                                        | 4     |
| [ ]    | APP-07 | Dock icon       | "Show app in Dock" switches between a Dock app and a menu-bar-only app.                                                                                          | 4     |
| [ ]    | APP-08 | Reset app       | Settings > System > Reset & restart wipes local data after a confirmation and restarts Shrimp.                                                                   | 4     |
| [ ]    | APP-09 | Error screens   | A crash, corrupted local data or a full disk each show a dedicated screen with a recovery action.                                                                | 4     |

## Onboarding and permissions

| Status | ID     | Feature           | Acceptance criterion                                                                                                                                           | Phase |
| ------ | ------ | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | ONB-01 | Permissions       | First launch walks through Microphone and Accessibility (and Input Monitoring when the hotkey needs it), detects each grant live and links to System Settings. | 4     |
| [ ]    | ONB-02 | Mic test          | Onboarding shows a live level meter and lets the user change the microphone.                                                                                   | 4     |
| [ ]    | ONB-03 | Shortcut step     | Onboarding shows the push-to-talk shortcut, offers to change it and introduces hands-free.                                                                     | 4     |
| [ ]    | ONB-04 | Languages step    | Onboarding asks which languages the user speaks.                                                                                                               | 4     |
| [ ]    | ONB-05 | Engine setup      | Onboarding downloads the default Whisper model with progress and optionally takes an AI provider key.                                                          | 4     |
| [ ]    | ONB-06 | Practice          | Skippable practice dictations show what Shrimp does.                                                                                                           | 4     |
| [ ]    | ONB-07 | Gating and resume | Dictation shortcuts stay inactive until onboarding reaches a dictation step, and onboarding resumes where it stopped.                                          | 4     |
| [ ]    | ONB-08 | Re-verify         | After an update, a lost permission is detected and the user is asked to grant it again.                                                                        | 4     |

## Accessibility

| Status | ID      | Feature             | Acceptance criterion                                                                                                             | Phase |
| ------ | ------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | A11Y-01 | Keyboard navigation | Every main-window control is reachable by keyboard (Tab, Space/Enter, arrows for tabs, Cmd+[ and Cmd+]), and dialogs trap focus. | 4     |
| [ ]    | A11Y-02 | Screen readers      | VoiceOver announces every control by name, and the UI respects Reduce Motion.                                                    | 4     |

## Windows

| Status | ID     | Feature            | Acceptance criterion                                                                                                                                                                             | Phase |
| ------ | ------ | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| [ ]    | WIN-01 | Feature parity     | Every phase 1–4 item passes on Windows 10 and 11 (x64), with NVDA standing in for VoiceOver.                                                                                                     | 5     |
| [ ]    | WIN-02 | Default shortcuts  | Ctrl+Win is push to talk (Start stays closed while held), Ctrl+Win+Space hands-free, Ctrl+Win+Alt Command Mode, Shift+Alt+Z paste last, Shift+Alt+X copy last, Win+Alt+1/2/O for Transforms.     | 5     |
| [ ]    | WIN-03 | Reserved shortcuts | Standalone Ctrl, Win+E/R/L/D/Tab/I/S/X/P/Up/Down/Q, Ctrl+C and Ctrl+V are rejected.                                                                                                              | 5     |
| [ ]    | WIN-04 | Paste rules        | Shift+Insert pastes in coding editors and their terminals; the clipboard is held about 3 s for Outlook, Word and WhatsApp and about 5 s for remote desktops; text, HTML and images are restored. | 5     |
| [ ]    | WIN-05 | Clipboard history  | Shrimp's clipboard writes stay out of Win+V history and cloud clipboard sync.                                                                                                                    | 5     |
| [ ]    | WIN-06 | Elevated apps      | When a paste into an elevated app fails, Shrimp says "Run Shrimp as administrator to insert text everywhere".                                                                                    | 5     |
| [ ]    | WIN-07 | Placeholder fields | A field that shows only placeholder text counts as empty for casing and spacing.                                                                                                                 | 5     |
| [ ]    | WIN-08 | Per-app mute       | Mute while dictating mutes each playing app and restores its volume, and is on by default on Windows.                                                                                            | 5     |
| [ ]    | WIN-09 | Tray menu          | The tray's right-click menu includes Paste last transcript, greyed out when there is none.                                                                                                       | 5     |
| [ ]    | WIN-10 | Quiet start        | Launch at login is on after install and starts Shrimp in the background.                                                                                                                         | 5     |
| [ ]    | WIN-11 | UI Automation      | Context awareness, Command Mode and auto-learn read text through UI Automation.                                                                                                                  | 5     |
| [ ]    | WIN-12 | Media pause        | Pause music while dictating uses the system media transport controls.                                                                                                                            | 5     |

## Packaging and updates

| Status | ID     | Feature           | Acceptance criterion                                                                                                   | Phase |
| ------ | ------ | ----------------- | ---------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | PKG-01 | macOS DMG         | A Developer ID-signed, notarized DMG for Apple Silicon installs Shrimp into /Applications.                             | 5     |
| [ ]    | PKG-02 | Windows installer | A signed per-user installer (MSI or NSIS) installs to %LOCALAPPDATA% without admin rights.                             | 5     |
| [ ]    | PKG-03 | Check for updates | "Check for updates…" in the tray finds a newer release on the configured feed and offers "Update ready, restart now?". | 5     |
| [ ]    | PKG-04 | Idle updates      | Updates wait until no dictation is running, and the main window reopens after an update.                               | 5     |
| [ ]    | PKG-05 | Grants survive    | Updating the signed release keeps its macOS privacy grants.                                                            | 5     |

## Audit and regression

| Status | ID     | Feature      | Acceptance criterion                                                                                                                               | Phase |
| ------ | ------ | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| [ ]    | AUD-01 | Parity audit | Every item here is re-checked against Wispr Flow's then-current docs and changelog; new features are added or explicitly scoped out.               | 6     |
| [ ]    | AUD-02 | Regression   | The automated tests and a scripted tauri-mcp walkthrough of every phase 1–5 item pass on macOS and Windows.                                        | 6     |
| [ ]    | AUD-03 | Latency      | Key-release-to-insert latency for a 5-second dictation (local Whisper, with and without AI edits) is measured on both platforms and recorded here. | 6     |

## Out of scope

The owner decided these Wispr Flow features are not part of Shrimp.

- **Notetaker and notes:** Meetings and the Notetaker; "Ask anything"; Notepad; Scratchpad
  (the floating notepad that replaced Flow Notes) and its role as a fallback for failed pastes
  (Shrimp uses the clipboard and a notice instead, `INS-04`); calendar and other connectors;
  the Wispr MCP server; transcript import from Otter and Granola.
- **Accounts and sign-in:** browser sign-in (Google, Apple, Microsoft, SSO, email), email
  verification, profile, account deletion.
- **Cloud sync:** Private Cloud Sync, and syncing of dictionary, snippets, styles,
  languages, stats or Scratchpad; the "Improve the model for everyone" data-sharing opt-out;
  the HIPAA BAA.
- **Teams:** shared dictionaries and snippets, team leaderboard and usage, invites, the
  admin portal, SSO/SCIM/MDM, audit logs, organisation policy locks, "Team updates"
  notifications.
- **Billing:** plans, trials, the free-tier weekly word cap, referrals and discounts. Every
  Shrimp feature is available to every user.
- **Mobile apps:** the iOS keyboard, iOS notes, widgets and Siri; the Android bubble and its
  related features.
- **Wispr's cloud service:** service-status banners and slow-dictation incident notices,
  support and feedback forms that submit to Wispr, and Wispr's Canto speech model (replaced
  by the pluggable engines in `ENG-*`).
- **Unconfirmed or undocumented in Wispr's docs:** Focus Mode (its help article returns
  404), the "Tasks" and "Extensions" sidebar items, "Instruct" history badges, the
  "hyperlink email signature". These are revisited in `AUD-01`.
- **Linux:** Wispr has no Linux app; Shrimp targets macOS and Windows only.

## Sources

Fetched on 2026-10-13.

- Product: [wisprflow.ai](https://wisprflow.ai),
  [pricing](https://wisprflow.ai/pricing), [what's new](https://wisprflow.ai/whats-new),
  [older changelog](https://wisprflow.featurebase.app/changelog),
  [developers](https://wisprflow.ai/developers), [data controls](https://wisprflow.ai/data-controls),
  [Canto](https://wisprflow.ai/canto).
- Shortcuts and dictation:
  [shortcuts](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts),
  [first dictation](https://docs.wisprflow.ai/articles/6409258247-starting-your-first-dictation),
  [hands-free](https://docs.wisprflow.ai/articles/6391241694-use-flow-hands-free),
  [20-minute sessions](https://docs.wisprflow.ai/articles/4841123325-longer-dictation-sessions-now-up-to-20-minutes),
  [retry](https://docs.wisprflow.ai/articles/2503460374-retry-failed-transcriptions).
- Insertion:
  [pasting](https://docs.wisprflow.ai/articles/7971211038-fix-text-not-pasting-after-dictation),
  [keyboard layouts](https://docs.wisprflow.ai/articles/1621472516-flow-fails-to-detect-text-fields-or-inserts-incorrectly-on-non-qwerty-keyboard-layouts),
  [terminals](https://docs.wisprflow.ai/articles/6478598909-using-flow-with-linux-wsl-and-terminal-applications),
  [remote desktops](https://docs.wisprflow.ai/articles/7336156466-use-flow-with-remote-desktops-citrix-rdp-vdi).
- Editing:
  [formatting and backtrack](https://docs.wisprflow.ai/articles/5373093536-how-do-i-use-smart-formatting-and-backtrack),
  [Auto Cleanup](https://docs.wisprflow.ai/articles/4283510616-auto-cleanup-control-how-much-flow-edits-your-dictation-beta),
  [capitalisation](https://docs.wisprflow.ai/articles/4518230910-why-flow-capitalizes-words-mid-sentence-and-how-to-control-it),
  [styles](https://docs.wisprflow.ai/articles/2368263928-how-to-setup-flow-styles),
  [dictionary](https://docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary),
  [snippets](https://docs.wisprflow.ai/articles/5784437944-create-and-use-snippets),
  [bulk import](https://docs.wisprflow.ai/articles/8955301725-how-do-i-bulk-import-for-dictionary-and-snippets),
  [Command Mode](https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode),
  [Transforms](https://docs.wisprflow.ai/articles/8068950331-how-to-use-transforms-beta),
  [context awareness](https://docs.wisprflow.ai/articles/4678293671-feature-context-awareness),
  [IDEs](https://docs.wisprflow.ai/articles/6434410694-use-flow-with-cursor-vs-code-and-other-ides).
- App:
  [Flow Bar](https://docs.wisprflow.ai/articles/1790396454-move-and-dock-the-flow-bar-on-desktop),
  [microphones](https://docs.wisprflow.ai/articles/8884408990-connect-and-set-up-external-audio-devices),
  [mic troubleshooting](https://docs.wisprflow.ai/articles/4351452717-troubleshooting-mic-issues),
  [mute and pause](https://docs.wisprflow.ai/articles/7231650589-auto-mute-music-while-dictating-on-macos-how-audio-detection-works),
  [languages](https://docs.wisprflow.ai/articles/3191899797-use-flow-with-multiple-languages),
  [history](https://docs.wisprflow.ai/articles/4465314211-delete-transcripts-and-history-in-wispr-flow),
  [usage stats](https://docs.wisprflow.ai/articles/8760230576-your-usage-tab-track-your-dictation-stats-in-wispr-flow),
  [voice profile](https://docs.wisprflow.ai/articles/7470830521-voice-profile-updates-when-the-update-your-voice-profile-notification-appears),
  [notifications](https://docs.wisprflow.ai/articles/2250194357-customize-notification-preferences-by-category),
  [navigation](https://docs.wisprflow.ai/articles/5096240724-navigating-the-wispr-flow-app-desktop-ios-and-android),
  [accessibility](https://docs.wisprflow.ai/articles/3941699399-keyboard-and-screen-reader-accessibility-in-wispr-flow),
  [setup](https://docs.wisprflow.ai/articles/3152211871-setup-guide),
  [permissions after updates](https://docs.wisprflow.ai/articles/5510622673-re-verify-wispr-flow-permissions-after-updating),
  [reset](https://docs.wisprflow.ai/articles/2999006910-reset-and-restart-the-wispr-flow-app),
  [Windows installer](https://docs.wisprflow.ai/articles/9633655004-installing-wispr-flow-on-windows-what-to-expect-from-the-downloadable-installer).

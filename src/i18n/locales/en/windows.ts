/**
 * Texts that read differently on Windows: "this PC" instead of "this Mac",
 * Settings instead of System Settings, the notification area instead of the
 * menu bar, Ctrl and Alt instead of ⌘ and ⌥. Every key replaces one in the
 * English catalog; each translation has the same keys.
 */
export const windows = {
  settings: { pageHint: "Alt+↑ and Alt+↓ move between pages." },
  appLanguage: { system: "Same as Windows" },
  system: {
    launchAtLoginBody:
      "Shrimp starts in the background when you sign in, ready in the notification area.",
    resetBody:
      "Deletes Shrimp's local data on this PC and restarts it. Use it if Shrimp keeps misbehaving.",
    resetConfirmBody:
      "This deletes your history, recordings, dictionary, snippets, stats, settings and API keys on this PC, then restarts Shrimp. Downloaded speech models are kept.",
  },
  help: {
    about:
      "Shrimp {{version}}. Everything you dictate stays on this PC unless you choose a cloud engine or provider.",
  },
  onboarding: {
    welcomeBody:
      "Speak anywhere you can type. Hold a key, talk, and let go: Shrimp writes what you said with Whisper on this PC, tidies it up and pastes it where your cursor is.",
    permissionsBody:
      "Shrimp needs your microphone. The row turns green as soon as Windows allows it.",
    permissionsNote:
      "Windows asks no permission for shortcuts or pasting. Apps running as administrator only take text when Shrimp runs as administrator too.",
    permission: { openSettings: "Open Settings" },
    engineBody:
      "Shrimp transcribes on this PC with Whisper. The model downloads once and works offline.",
    aiOllamaBody: "A model on this PC through Ollama. Nothing leaves your PC.",
    reverifyBody:
      "Windows no longer lets Shrimp use a permission it had. Grant it again to keep dictating.",
    reverifyHint:
      "In Settings › Privacy › Microphone, make sure desktop apps may use the microphone.",
  },
  voice: {
    privacy:
      "Your dictations are only stored on this PC. Generating sends your stats and a sample of recent transcripts to {{provider}}.",
  },
  history: { searchPlaceholder: "Search (Ctrl+F or /)" },
  privacy: {
    privacyModeBody:
      "Only Whisper on this PC and Ollama run: no audio or text goes to a cloud engine or provider. The Flow Bar and the notification area icon show a dot while it's on.",
    privacyModeOn:
      "Privacy mode is on, so only Whisper on this PC and Ollama run. Your choice here applies again when you turn it off.",
    neverBody:
      "Every transcript, recording and backup on this PC is deleted now, and new dictations aren't stored. Type “delete all” to confirm.",
    deleteAllBody:
      "Deletes every transcript and recording on this PC. Insights, your dictionary, snippets and settings stay.",
  },
  aiProvider: { ollamaNote: "Ollama runs on this PC; nothing leaves it." },
  apiKey: { saved: "Key saved in Windows Credential Manager" },
  speechEngine: {
    note: "Whisper on this PC is private and works offline. Cloud engines send your audio to OpenAI or Groq; if one fails, Shrimp falls back to Whisper on this PC.",
  },
  modelCard: { ready: "Ready. Dictation runs entirely on this PC." },
  sound: {
    muteBody: "Apps that are playing when you start stay muted until you finish.",
  },
  commandModeIntro: {
    step3: "Release. The result replaces your selection; Ctrl+Z undoes it.",
  },
  vibeCoding: { step1: "Open the Command Palette (Ctrl+Shift+P)." },
  contextAwareness: {
    body: "Shrimp reads the text around your cursor and the names on screen, so names are spelled right and the style fits the app. When off, nothing from your screen reaches the speech engine or the AI provider; casing and spacing still adapt to the text around the cursor, on this PC.",
  },
  micAdvice: {
    questions: {
      computer: {
        laptop: "A laptop",
        desktop: "A desktop PC",
      },
    },
    titles: { builtIn: "Your laptop's built-in microphone" },
    why: {
      desktop:
        "Most desktop PCs have no microphone of their own, and a USB microphone gives clear, low-noise audio.",
    },
  },
};

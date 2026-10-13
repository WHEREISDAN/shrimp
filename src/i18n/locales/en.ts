/** The English interface text, the source for every other language (LANG-06). */
export const en = {
  common: {
    cancel: "Cancel",
    close: "Close",
    copy: "Copy",
    delete: "Delete",
    loading: "Loading…",
    tryAgain: "Try again",
    ok: "OK",
  },
  home: {
    tagline: "Speech to text, anywhere.",
  },
  history: {
    title: "Recent activity",
    region: "Dictation history",
    search: "Search history",
    searchPlaceholder: "Search (⌘F or /)",
    empty: "No dictations yet",
    noMatches: "No dictations match “{{query}}”",
    disabled: "History disabled",
    disabledHint:
      "Shrimp doesn't store transcripts. Choose Store data locally in Settings › Data and privacy to keep them.",
    today: "Today",
    yesterday: "Yesterday",
    copied: "Copied to the clipboard",
    statusFailed: "Transcription failed",
    statusEmpty: "No speech detected",
    statusInterrupted: "Recording interrupted. Recover it to transcribe what was said.",
    notPasted: "Not pasted",
    unformatted: "Unformatted",
    unformattedHint: "AI edits weren't applied: the raw transcript was inserted.",
    rawShown: "Raw",
    rawShownHint: "Showing what you said, before AI edits.",
    retry: "Retry",
    recover: "Recover",
    retrying: "Transcribing again…",
    moreOptions: "More options",
    retryTranscript: "Retry transcript",
    undoAiEdit: "Undo AI edit",
    redoAiEdit: "Redo AI edit",
    playAudio: "Play audio",
    stopAudio: "Stop audio",
    extractAudio: "Extract audio",
    extracted: "Saved to {{path}}",
    deleteTranscript: "Delete transcript",
    deleteTitle: "Are you sure you want to delete this transcript?",
    deleteWarning: "Deleted transcripts cannot be undone or restored.",
    deleteConfirm: "Yes, delete it",
    deleted: "Transcript deleted",
    rowLabel: "{{time}}, {{app}}: {{text}}",
    unknownApp: "Unknown app",
    recording: "Recording, {{seconds}} s",
  },
  privacy: {
    storageTitle: "Local data storage",
    storageLocal: "Store data locally",
    storageLocalBody: "Stores dictation transcripts locally. Recordings are kept for 14 days.",
    storageAutoDelete: "Auto-delete local data every 24hrs",
    storageAutoDeleteBody: "Deletes history older than 24 hours, starting immediately.",
    storageNever: "Never store data locally",
    storageNeverBody:
      "Immediately deletes all history and stops storing new transcripts. Insights keep counting.",
    autoDeleteTitle: "Auto-delete history older than 24 hours?",
    autoDeleteConfirm: "Enable auto-delete",
    neverTitle: "Delete all history?",
    neverBody:
      "Every transcript, recording and backup on this Mac is deleted now, and new dictations aren't stored. Type “delete all” to confirm.",
    neverField: "Confirmation",
    neverConfirm: "Delete all data",
  },
} as const;

type Messages<T> = { [K in keyof T]: T[K] extends string ? string : Messages<T[K]> };

/** What every translation must provide: the same keys as English. */
export type Catalog = Messages<typeof en>;

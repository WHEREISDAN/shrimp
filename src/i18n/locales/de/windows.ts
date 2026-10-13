import type { WindowsCatalog } from "../en";

export const windows: WindowsCatalog = {
  settings: { pageHint: "Alt+↑ und Alt+↓ wechseln zwischen den Seiten." },
  appLanguage: { system: "Wie in Windows" },
  system: {
    launchAtLoginBody:
      "Shrimp startet beim Anmelden im Hintergrund und wartet im Infobereich der Taskleiste.",
    resetBody:
      "Löscht die lokalen Daten von Shrimp auf diesem PC und startet die App neu. Hilft, wenn Shrimp immer wieder Probleme macht.",
    resetConfirmBody:
      "Dadurch werden Verlauf, Aufnahmen, Wörterbuch, Textbausteine, Statistiken, Einstellungen und API-Schlüssel auf diesem PC gelöscht und Shrimp neu gestartet. Heruntergeladene Sprachmodelle bleiben erhalten.",
  },
  help: {
    about:
      "Shrimp {{version}}. Alles, was du diktierst, bleibt auf diesem PC, außer du wählst eine Cloud-Engine oder einen Cloud-Anbieter.",
  },
  onboarding: {
    welcomeBody:
      "Sprich überall, wo du tippen kannst. Taste halten, sprechen, loslassen: Shrimp schreibt mit Whisper auf diesem PC auf, was du gesagt hast, bringt es in Form und fügt es an der Cursorposition ein.",
    permissionsBody:
      "Shrimp braucht dein Mikrofon. Die Zeile wird grün, sobald Windows es erlaubt.",
    permissionsNote:
      "Für Kurzbefehle und Einfügen fragt Windows keine Berechtigung ab. Apps, die als Administrator laufen, nehmen nur Text an, wenn auch Shrimp als Administrator läuft.",
    permission: { openSettings: "Einstellungen öffnen" },
    engineBody:
      "Shrimp transkribiert mit Whisper auf diesem PC. Das Modell wird einmal heruntergeladen und funktioniert offline.",
    aiOllamaBody: "Ein Modell auf diesem PC über Ollama. Nichts verlässt deinen PC.",
    reverifyBody:
      "Windows lässt Shrimp eine zuvor erteilte Berechtigung nicht mehr nutzen. Erteile sie erneut, um weiter zu diktieren.",
    reverifyHint:
      "Prüfe unter Einstellungen › Datenschutz › Mikrofon, dass Desktop-Apps das Mikrofon verwenden dürfen.",
  },
  voice: {
    privacy:
      "Deine Diktate werden nur auf diesem PC gespeichert. Zum Erstellen werden deine Statistiken und eine Auswahl neuer Transkripte an {{provider}} gesendet.",
  },
  history: { searchPlaceholder: "Suchen (Strg+F oder /)" },
  privacy: {
    privacyModeBody:
      "Nur Whisper auf diesem PC und Ollama laufen: Weder Audio noch Text gehen an eine Cloud-Engine oder einen Cloud-Anbieter. Die Flow Bar und das Symbol im Infobereich zeigen dabei einen Punkt.",
    privacyModeOn:
      "Der Datenschutzmodus ist an, daher laufen nur Whisper auf diesem PC und Ollama. Deine Auswahl hier gilt wieder, sobald du ihn ausschaltest.",
    neverBody:
      "Alle Transkripte, Aufnahmen und Backups auf diesem PC werden jetzt gelöscht, und neue Diktate werden nicht gespeichert. Gib zur Bestätigung „delete all“ ein.",
  },
  aiProvider: { ollamaNote: "Ollama läuft auf diesem PC; nichts verlässt ihn." },
  apiKey: { saved: "Schlüssel in der Windows-Anmeldeinformationsverwaltung gespeichert" },
  speechEngine: {
    note: "Whisper auf diesem PC ist privat und funktioniert offline. Cloud-Engines senden dein Audio an OpenAI oder Groq; fällt eine aus, nutzt Shrimp wieder Whisper auf diesem PC.",
  },
  modelCard: { ready: "Bereit. Das Diktieren läuft komplett auf diesem PC." },
  sound: {
    muteBody: "Apps, die beim Start etwas abspielen, bleiben stumm, bis du fertig bist.",
  },
  commandModeIntro: {
    step3: "Lass los. Das Ergebnis ersetzt deine Auswahl; Strg+Z macht es rückgängig.",
  },
  vibeCoding: { step1: "Öffne die Befehlspalette (Strg+Umschalt+P)." },
  contextAwareness: {
    body: "Shrimp liest den Text um deinen Cursor und die Namen auf dem Bildschirm, damit Namen richtig geschrieben werden und der Stil zur App passt. Ist dies aus, gelangt nichts von deinem Bildschirm zur Spracherkennung oder zum KI-Anbieter; Groß- und Kleinschreibung sowie Leerzeichen passen sich weiterhin lokal auf diesem PC an den Text um den Cursor an.",
  },
  micAdvice: {
    questions: {
      computer: {
        laptop: "Auf einem Laptop",
        desktop: "Auf einem Desktop-PC",
      },
    },
    titles: { builtIn: "Das integrierte Mikrofon deines Laptops" },
    why: {
      desktop:
        "Die meisten Desktop-PCs haben kein eigenes Mikrofon, und ein USB-Mikrofon liefert klaren, rauscharmen Klang.",
    },
  },
};

import type { SettingsSectionsCatalog } from "../en";

/** German: Settings sections: AI provider, speech engine, sound, Flow Bar, experimental, vibe coding, shortcuts. */
export const settingsSections: SettingsSectionsCatalog = {
  aiProvider: {
    title: "KI-Anbieter",
    provider: "Anbieter",
    model: "Modell",
    ollamaNote: "Ollama läuft auf diesem Mac; nichts verlässt ihn.",
    cloudNote:
      "Dein diktierter Text wird zur Bereinigung mit deinem eigenen Schlüssel an {{provider}} gesendet.",
    test: "Testen",
    testing: "Wird getestet …",
    testOk: "In {{latency, number}} ms bereinigt: „{{output}}“",
    testFailed: "Hat nicht funktioniert.",
  },
  ollama: {
    address: "Adresse",
    model: "Ollama-Modell",
    automatic: "Automatisch (empfohlen)",
    notRunning: "Ollama läuft nicht unter {{url}}.",
    retry: "Erneut versuchen",
  },
  apiKey: {
    saved: "Schlüssel in deinem Schlüsselbund gespeichert",
    none: "Noch kein Schlüssel",
    label: "API-Schlüssel für {{company}}",
    placeholder: "API-Schlüssel einfügen",
    save: "Schlüssel speichern",
    remove: "Schlüssel entfernen",
  },
  speechEngine: {
    title: "Spracherkennung",
    model: "Sprachmodell",
    note: "Whisper auf diesem Mac ist privat und funktioniert offline. Cloud-Engines senden dein Audio an OpenAI oder Groq; fällt eine aus, nutzt Shrimp wieder Whisper auf diesem Mac.",
  },
  whisperModels: {
    title: "Whisper-Modell",
    list: "Whisper-Modelle",
    size: "{{size, number}} MB",
    recommended: "Empfohlen",
    waiting: "Wartet auf Download",
    notDownloaded: "Nicht heruntergeladen",
    downloading: "Download: {{percent, number}} %",
    downloaded: "Heruntergeladen",
    inUse: "In Verwendung",
    download: "Herunterladen",
    deleteModel: "{{model}} löschen",
    note: "Größere Modelle sind genauer, aber langsamer. Eine neue Auswahl wird bei Bedarf heruntergeladen und ab deinem nächsten Diktat verwendet.",
  },
  modelCard: {
    title: "Sprachmodell",
    onDevice: "{{size, number}} MB · auf dem Gerät",
    waiting: "Wartet auf Download …",
    progress: "Download-Fortschritt des Modells",
    downloading: "Download: {{percent, number}} % ({{received, number}} von {{total, number}} MB)",
    loading: "Heruntergeladen. Wird geladen …",
    ready: "Bereit. Das Diktieren läuft komplett auf diesem Mac.",
    retry: "Erneut versuchen",
  },
  permissionsCard: {
    title: "Berechtigungen",
    microphoneWhy: "Hört dich, während du diktierst.",
    accessibilityWhy: "Erkennt den Diktier-Kurzbefehl und fügt den Text ein.",
    allow: "Erlauben …",
    openSettings: "Einstellungen öffnen",
  },
  sound: {
    title: "Ton",
    dictation: "Diktiertöne",
    dictationBody:
      "Ein Ton, wenn ein Diktat startet, endet, in den Freihandmodus wechselt, abgebrochen oder eingefügt wird.",
    notifications: "Mitteilungstöne",
    notificationsBody: "Ein Ton bei Warnungen, Fehlern und Meilensteinen.",
    mute: "Audio beim Diktieren stummschalten",
    muteBody: "Läuft beim Start etwas, bleibt dein Mac stumm, bis du fertig bist.",
    pause: "Musik beim Diktieren pausieren",
    pauseBody:
      "Pausiert Musik, Spotify und andere Player (nie Anrufe) und setzt sie fort, wenn du fertig bist. Ersetzt das Stummschalten.",
  },
  flowBarSettings: {
    title: "Flow Bar",
    alwaysShow: "Flow Bar immer anzeigen",
    alwaysShowBody:
      "Ist dies aus, erscheint die Leiste nur beim Diktieren oder wenn Shrimp dir etwas mitteilen will.",
    hiddenUntil: "Ausgeblendet bis {{time}}",
    showNow: "Jetzt anzeigen",
    note: "Zieh die Leiste an den unteren, linken oder rechten Bildschirmrand; sie folgt dir auf den Bildschirm, auf dem dein Zeiger ist. Ein Rechtsklick darauf zeigt das Mikrofon und mehr.",
  },
  experimental: {
    title: "Experimentell",
    commandMode: "Command Mode",
    commandModeBody:
      "Wähle Text aus, halte {{shortcut}} gedrückt und sag, was du ändern willst, etwa „formuliere das förmlicher“ oder „übersetze ins Englische“. Ist nichts ausgewählt, stell eine Frage.",
    pressEnter: "Sprachbefehl „press enter“",
    pressEnterBody:
      "Beende ein Diktat mit „press enter“, um es abzuschicken: Shrimp lässt die Wörter weg und drückt nach dem Einfügen Enter.",
  },
  commandModeIntro: {
    title: "Command Mode verwenden (Beta)",
    step1: "Wähle den Text aus, den du ändern willst, oder setz den Cursor in ein Textfeld.",
    step2:
      "Halte <kbd>{{shortcut}}</kbd> gedrückt und sag, was passieren soll: „mach das kürzer“, „übersetze ins Englische“, „mach eine Liste daraus“.",
    step3: "Lass los. Das Ergebnis ersetzt deine Auswahl; Cmd+Z macht es rückgängig.",
    body: "Ist nichts ausgewählt, stell eine Frage, und die Antwort erscheint neben der Flow Bar. Beginne mit „search“, um eine Websuche zu öffnen. Drück den Kurzbefehl zweimal, damit Shrimp freihändig weiter zuhört; Esc bricht ab. Deine Auswahl und das Feld, in dem du bist, werden an deinen KI-Anbieter gesendet.",
    disable: "Deaktivieren",
    gotIt: "Verstanden",
  },
  vibeCoding: {
    title: "Vibe-Coding",
    fileTagging: "Dateien im Chat erwähnen",
    fileTaggingBody:
      "Sag im Chat von Cursor und Windsurf den Namen einer Datei („index dot ts“) oder „tag“ und ihren Namen, und Shrimp schreibt eine @-Erwähnung. Funktioniert für geöffnete oder in der Seitenleiste angezeigte Dateien.",
    variables: "Variablenerkennung",
    variablesBody:
      "Sag Namen in Cursor, VS Code und Windsurf ganz natürlich, „user ID“ für <code>userId</code>: Shrimp gleicht sie mit dem Code auf dem Bildschirm ab. Sag „camel case“, „snake case“, „pascal case“ oder „kebab case“ vor Wörtern, um einen neuen Namen zu bilden.",
    step1: "Öffne die Befehlspalette (Cmd+Shift+P).",
    step2: "Führe „Toggle Screen Reader Accessibility Mode“ aus.",
    step3: "Prüfe, ob „Screen Reader Optimized“ in der Statusleiste steht.",
  },
  contextAwareness: {
    title: "Kontexterkennung",
    body: "Shrimp liest den Text um deinen Cursor und die Namen auf dem Bildschirm, damit Namen richtig geschrieben werden und der Stil zur App passt. Ist dies aus, gelangt nichts von deinem Bildschirm zur Spracherkennung oder zum KI-Anbieter; Groß- und Kleinschreibung sowie Leerzeichen passen sich weiterhin lokal auf diesem Mac an den Text um den Cursor an.",
  },
  shortcutsSection: {
    title: "Kurzbefehle",
    pushToTalk: "Push-to-Talk",
    change: "Ändern",
    note: "Jede Aktion kann bis zu vier Kurzbefehle haben, auch Maustasten. „Enter drücken“ sendet Enter an die App, in der du bist; halte dabei Shift oder eine andere Sondertaste, um Shift+Enter zu senden.",
  },
};

import type { SettingsSectionsCatalog } from "../en";

/** Italian: settings sections (AI provider, speech engine, sound, Flow Bar, experimental, vibe coding, shortcuts). */
export const settingsSections: SettingsSectionsCatalog = {
  aiProvider: {
    title: "Provider IA",
    provider: "Provider",
    model: "Modello",
    ollamaNote: "Ollama funziona su questo Mac; nulla esce dal Mac.",
    cloudNote: "Il testo dettato viene inviato a {{provider}} per la pulizia, con la tua chiave.",
    test: "Prova",
    testing: "Prova in corso…",
    testOk: "Ripulito in {{latency, number}} ms: “{{output}}”",
    testFailed: "Non ha funzionato.",
  },
  ollama: {
    address: "Indirizzo",
    model: "Modello Ollama",
    automatic: "Automatico (consigliato)",
    notRunning: "Ollama non è in esecuzione su {{url}}.",
    retry: "Riprova",
  },
  apiKey: {
    saved: "Chiave salvata nel portachiavi",
    none: "Nessuna chiave",
    label: "Chiave API {{company}}",
    placeholder: "Incolla la chiave API",
    save: "Salva chiave",
    remove: "Rimuovi chiave",
  },
  speechEngine: {
    title: "Motore vocale",
    model: "Modello vocale",
    note: "Whisper su questo Mac è privato e funziona offline. I motori cloud inviano l'audio a OpenAI o Groq; se uno non funziona, Shrimp torna a Whisper su questo Mac.",
  },
  whisperModels: {
    title: "Modello Whisper",
    list: "Modelli Whisper",
    size: "{{size, number}} MB",
    recommended: "Consigliato",
    waiting: "In attesa di download",
    notDownloaded: "Non scaricato",
    downloading: "Download {{percent, number}}%",
    downloaded: "Scaricato",
    inUse: "In uso",
    download: "Scarica",
    deleteModel: "Elimina {{model}}",
    note: "I modelli più grandi sono più precisi e più lenti. Una nuova scelta viene scaricata se serve e usata dalla prossima dettatura.",
  },
  modelCard: {
    title: "Modello vocale",
    onDevice: "{{size, number}} MB · sul dispositivo",
    waiting: "In attesa di download…",
    progress: "Avanzamento del download del modello",
    downloading: "Download {{percent, number}}% ({{received, number}} di {{total, number}} MB)",
    loading: "Scaricato. Caricamento…",
    ready: "Pronto. La dettatura avviene interamente su questo Mac.",
    retry: "Riprova",
  },
  permissionsCard: {
    title: "Permessi",
    microphoneWhy: "Ti sente mentre detti.",
    accessibilityWhy: "Rileva la scorciatoia di dettatura e incolla il testo.",
    allow: "Consenti…",
    openSettings: "Apri Impostazioni",
  },
  sound: {
    title: "Suono",
    dictation: "Suoni di dettatura",
    dictationBody:
      "Un suono quando la dettatura inizia, si ferma, passa a mani libere, viene annullata o incollata.",
    notifications: "Suoni di notifica",
    notificationsBody: "Un suono con avvisi, errori e traguardi.",
    mute: "Silenzia l'audio durante la dettatura",
    muteBody:
      "Se qualcosa è in riproduzione quando inizi, il Mac resta silenziato finché non finisci.",
    pause: "Metti in pausa la musica durante la dettatura",
    pauseBody:
      "Mette in pausa Musica, Spotify e altri player (mai le chiamate) e li riprende quando finisci. Sostituisce il silenziamento.",
  },
  flowBarSettings: {
    title: "Flow Bar",
    alwaysShow: "Mostra sempre la Flow Bar",
    alwaysShowBody:
      "Altrimenti la barra appare solo mentre detti o quando Shrimp ha qualcosa da dirti.",
    hiddenUntil: "Nascosta fino alle ore {{time}}",
    showNow: "Mostra ora",
    note: "Trascina la barra sul bordo inferiore, sinistro o destro dello schermo; ti segue sul monitor dove si trova il puntatore. Fai clic destro sulla barra per il microfono e altro.",
  },
  experimental: {
    title: "Sperimentale",
    commandMode: "Command Mode",
    commandModeBody:
      "Seleziona del testo, tieni premuto {{shortcut}} e di' cosa cambiare, come “rendilo più formale” o “traducilo in inglese”. Senza selezione, fai una domanda.",
    pressEnter: "Comando vocale “press enter”",
    pressEnterBody:
      "Termina una dettatura con “press enter” per inviarla: Shrimp omette le parole e preme Invio dopo aver incollato.",
  },
  commandModeIntro: {
    title: "Usare Command Mode (beta)",
    step1: "Seleziona il testo da modificare o metti il cursore in un campo di testo.",
    step2:
      "Tieni premuto <kbd>{{shortcut}}</kbd> e di' cosa fare: “rendilo più breve”, “traducilo in inglese”, “trasformalo in un elenco”.",
    step3: "Rilascia. Il risultato sostituisce la selezione; ⌘Z lo annulla.",
    body: "Senza selezione, fai una domanda e la risposta appare accanto alla Flow Bar. Inizia con “search” per aprire una ricerca web. Premi due volte la scorciatoia per continuare ad ascoltare a mani libere; Esc annulla. La selezione e il campo in cui ti trovi vengono inviati al tuo provider IA.",
    disable: "Disattiva",
    gotIt: "Ho capito",
  },
  vibeCoding: {
    title: "Vibe coding",
    fileTagging: "Tag dei file nella chat",
    fileTaggingBody:
      "Nella chat di Cursor e Windsurf, di' il nome di un file (“index dot ts”) oppure “tag” e il suo nome, e Shrimp scrive una @menzione. Funziona con i file aperti o visibili nella barra laterale.",
    variables: "Riconoscimento delle variabili",
    variablesBody:
      "In Cursor, VS Code e Windsurf, pronuncia i nomi in modo naturale, “user ID” per <code>userId</code>: Shrimp li abbina al codice sullo schermo. Di' “camel case”, “snake case”, “pascal case” o “kebab case” prima delle parole per creare un nuovo nome.",
    step1: "Apri la Command Palette (⇧⌘P).",
    step2: "Esegui “Toggle Screen Reader Accessibility Mode”.",
    step3: "Verifica che nella barra di stato compaia “Screen Reader Optimized”.",
  },
  contextAwareness: {
    title: "Riconoscimento del contesto",
    body: "Shrimp legge il testo intorno al cursore e i nomi sullo schermo, così i nomi sono scritti correttamente e lo stile si adatta all'app. Se disattivato, nulla dello schermo arriva al motore vocale o al provider IA; maiuscole e spaziatura si adattano comunque al testo intorno al cursore, su questo Mac.",
  },
  shortcutsSection: {
    title: "Scorciatoie",
    pushToTalk: "Premi per parlare",
    change: "Modifica",
    note: "Ogni azione accetta fino a quattro scorciatoie, inclusi i tasti del mouse. “Premi Invio” invia Invio all'app in uso; aggiungi Maiuscole o un altro modificatore per inviare Maiuscole+Invio.",
  },
};

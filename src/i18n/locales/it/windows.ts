import type { WindowsCatalog } from "../en";

export const windows: WindowsCatalog = {
  settings: { pageHint: "Alt+↑ e Alt+↓ passano da una pagina all'altra." },
  appLanguage: { system: "Stessa lingua di Windows" },
  system: {
    launchAtLoginBody: "Shrimp si avvia in background quando accedi, pronto nell'area di notifica.",
    resetBody:
      "Elimina i dati locali di Shrimp su questo PC e lo riavvia. Usalo se Shrimp continua a non funzionare bene.",
    resetConfirmBody:
      "Verranno eliminati cronologia, registrazioni, dizionario, snippet, statistiche, impostazioni e chiavi API su questo PC, poi Shrimp si riavvierà. I modelli vocali scaricati vengono mantenuti.",
  },
  help: {
    about:
      "Shrimp {{version}}. Tutto ciò che detti resta su questo PC, a meno che tu non scelga un motore o un provider cloud.",
  },
  onboarding: {
    welcomeBody:
      "Parla ovunque puoi scrivere. Tieni premuto un tasto, parla e rilascia: Shrimp trascrive ciò che hai detto con Whisper su questo PC, lo sistema e lo incolla dove si trova il cursore.",
    permissionsBody:
      "Shrimp ha bisogno del microfono. La riga diventa verde appena Windows lo consente.",
    permissionsNote:
      "Windows non chiede permessi per le scorciatoie o per incollare. Le app eseguite come amministratore ricevono testo solo se anche Shrimp è eseguito come amministratore.",
    permission: { openSettings: "Apri Impostazioni" },
    engineBody:
      "Shrimp trascrive su questo PC con Whisper. Il modello si scarica una volta sola e funziona offline.",
    aiOllamaBody: "Un modello su questo PC tramite Ollama. Nulla lascia il tuo PC.",
    reverifyBody:
      "Windows non permette più a Shrimp di usare un permesso che aveva. Concedilo di nuovo per continuare a dettare.",
    reverifyHint:
      "In Impostazioni › Privacy › Microfono, verifica che le app desktop possano usare il microfono.",
  },
  voice: {
    privacy:
      "Le tue dettature restano solo su questo PC. La generazione invia le tue statistiche e un campione di trascrizioni recenti a {{provider}}.",
  },
  history: { searchPlaceholder: "Cerca (Ctrl+F o /)" },
  privacy: {
    privacyModeBody:
      "Funzionano solo Whisper su questo PC e Ollama: nessun audio o testo va a motori o provider cloud. La Flow Bar e l'icona nell'area di notifica mostrano un punto quando è attiva.",
    privacyModeOn:
      "La modalità privacy è attiva, quindi funzionano solo Whisper su questo PC e Ollama. La tua scelta qui torna valida quando la disattivi.",
    neverBody:
      "Ogni trascrizione, registrazione e backup su questo PC viene eliminato subito, e le nuove dettature non vengono salvate. Digita “delete all” per confermare.",
  },
  aiProvider: { ollamaNote: "Ollama funziona su questo PC; nulla esce dal PC." },
  apiKey: { saved: "Chiave salvata in Gestione credenziali di Windows" },
  speechEngine: {
    note: "Whisper su questo PC è privato e funziona offline. I motori cloud inviano l'audio a OpenAI o Groq; se uno non funziona, Shrimp torna a Whisper su questo PC.",
  },
  modelCard: { ready: "Pronto. La dettatura avviene interamente su questo PC." },
  sound: {
    muteBody: "Le app in riproduzione quando inizi restano silenziate finché non finisci.",
  },
  commandModeIntro: {
    step3: "Rilascia. Il risultato sostituisce la selezione; Ctrl+Z lo annulla.",
  },
  vibeCoding: { step1: "Apri la Command Palette (Ctrl+Maiusc+P)." },
  contextAwareness: {
    body: "Shrimp legge il testo intorno al cursore e i nomi sullo schermo, così i nomi sono scritti correttamente e lo stile si adatta all'app. Se disattivato, nulla dello schermo arriva al motore vocale o al provider IA; maiuscole e spaziatura si adattano comunque al testo intorno al cursore, su questo PC.",
  },
  micAdvice: {
    questions: {
      computer: {
        laptop: "Un portatile",
        desktop: "Un PC desktop",
      },
    },
    titles: { builtIn: "Il microfono integrato del tuo portatile" },
    why: {
      desktop:
        "La maggior parte dei PC desktop non ha un microfono integrato, e un microfono USB offre un audio nitido e con poco rumore.",
    },
  },
};

import type { ControlsCatalog } from "../en";

/** Italian: microphone, languages, shortcuts, shared dialog buttons and the Flow Bar's cards and menu. */
export const controls: ControlsCatalog = {
  microphone: {
    title: "Microfono",
    autoDetect: "Rilevamento automatico",
    autoDetectHint: "usa il microfono collegato con la priorità più alta",
    list: "Microfoni",
    builtIn: "{{name}} (integrato, consigliato)",
    notConnected: "Non collegato",
    inUse: "In uso",
    connected: "Collegato",
    moveUp: "Sposta su {{name}}",
    moveDown: "Sposta giù {{name}}",
    forget: "Dimentica",
    forgetDevice: "Dimentica {{name}}",
    none: "Nessun microfono rilevato.",
    showOthers: "Mostra altri dispositivi",
    showOthersHint: "(ingressi virtuali e loopback, mai scelti automaticamente)",
    inputLevel: "Livello di input",
    level: "Livello del microfono",
    notSure: "Non sai quale microfono usare?",
  },
  micAdvice: {
    title: "Trova il microfono giusto per te",
    progress: "Domanda {{current, number}} di {{total, number}}",
    startOver: "Ricomincia",
    questions: {
      place: {
        label: "Dove detti più spesso?",
        quiet: "In una stanza silenziosa",
        shared: "In un ufficio condiviso o al bar",
        moving: "In movimento",
      },
      computer: {
        label: "Su cosa detti?",
        laptop: "Un MacBook",
        desktop: "Un Mac desktop (mini, iMac, Studio)",
      },
      headphones: {
        label: "Indossi le cuffie mentre detti?",
        none: "No",
        wired: "Cuffie a filo, con o senza microfono",
        bluetooth: "AirPods o altre cuffie Bluetooth",
      },
    },
    titles: {
      wiredHeadset: "Cuffie a filo con microfono",
      wiredHeadsetOrEarbuds: "Cuffie o auricolari a filo con microfono",
      yourWiredHeadset: "Le tue cuffie a filo con microfono",
      headsetOrUsb: "Cuffie con microfono o un microfono USB vicino a te",
      usbMicrophone: "Un microfono USB sulla scrivania",
      builtIn: "Il microfono integrato del tuo MacBook",
    },
    why: {
      moving:
        "Un microfono che si muove con te resta vicino alla bocca ovunque tu sia, così il rumore di fondo resta basso.",
      shared: "In una stanza rumorosa, un microfono vicino alla bocca sente te e non la stanza.",
      desktop:
        "I Mac desktop, tranne l'iMac, non hanno un microfono integrato, e un microfono USB offre un audio nitido e con poco rumore.",
      quiet: "In una stanza silenziosa è nitido, sempre disponibile e si attiva subito.",
    },
    tips: {
      bluetooth:
        "Usa le cuffie Bluetooth per ascoltare: il loro microfono suona ovattato, è lento ad attivarsi e porta l'audio in una modalità a bassa qualità.",
      whisper: "Puoi parlare a bassa voce: Shrimp amplifica la voce sommessa e i sussurri.",
      moving:
        "Scegli questo microfono nell'elenco, o mettilo al primo posto così Rilevamento automatico lo usa.",
      shared:
        "Metti questo microfono al primo posto, così Rilevamento automatico lo sceglie ogni volta che è collegato.",
      desktop: "Posizionalo a 15–30 cm dalla bocca, un po' di lato.",
      quiet:
        "Con il coperchio chiuso non ti sente, quindi in quel caso collega un altro microfono.",
    },
  },
  languages: {
    title: "Lingue",
    autoDetect: "Rilevamento automatico",
    autoDetectBody: "Trascrive qualsiasi lingua parli.",
    change: "Modifica",
    body: "Con più lingue, ogni dettatura viene trascritta in quella che parli. Le varianti di inglese, cinese e tedesco determinano grafia e caratteri. Passa il puntatore sulla Flow Bar per cambiare lingua.",
    search: "Cerca lingue",
    noneSelected: "Seleziona almeno una lingua o attiva il rilevamento automatico",
  },
  shortcuts: {
    title: "Scorciatoie",
    busy: "Prima termina o annulla la registrazione.",
    secureInput:
      "“Input da tastiera sicuro” è attivo (macOS indica {{app}}), quindi le scorciatoie con lettere o altri tasti non funzionano finché non viene disattivato. fn e le altre scorciatoie con soli modificatori funzionano comunque.",
    resetConfirm:
      "Ripristinare tutte le scorciatoie ai valori predefiniti? L'operazione non può essere annullata.",
    reset: "Ripristina",
    resetToDefault: "Ripristina predefinita",
    change: "{{action}}: {{shortcut}}. Modifica",
    remove: "Rimuovi la scorciatoia {{shortcut}} da {{action}}",
    removeTitle: "Rimuovi scorciatoia",
    add: "Fai clic per aggiungere una scorciatoia",
    addAnother: "Aggiungi un'altra scorciatoia per {{action}}",
    addAnotherTitle: "Aggiungi altra",
    pressKey: "Premi un tasto o un pulsante del mouse",
    cancelRecording: "Annulla registrazione",
    saved: "Scorciatoia salvata",
    notSet: "Non impostata",
    recorder: "Scorciatoia {{label}}: {{shortcut}}",
    removeOne: "Rimuovi",
    removeOneLabel: "Rimuovi scorciatoia {{label}}",
  },
  dialog: {
    done: "Fine",
    save: "Salva",
  },
  wand: {
    label: "Bacchetta delle trasformazioni",
    run: "Applica {{name}} alla selezione",
    choose: "Scegli una trasformazione",
    menu: "Menu della bacchetta delle trasformazioni",
    autoApply: "Applica automaticamente dopo la dettatura",
    configure: "Configura le trasformazioni",
  },
  flowBarMenu: {
    label: "Menu della Flow Bar",
    microphone: "Microfono ›",
    pasteLast: "Incolla l'ultima trascrizione",
    history: "Cronologia trascrizioni",
    settings: "Impostazioni",
    hide: "Nascondi per 1 ora",
  },
  answer: {
    region: "Risposta di Command Mode",
    dismiss: "Chiudi",
    copied: "Copiato ✓",
  },
  changes: {
    region: "Modifiche di {{name}}",
    retry: "Riprova",
    undo: "Annulla",
    accept: "Accetta modifiche",
    quick: {
      label: "Trasforma di nuovo",
      promptEngineer: "Prompt Engineer",
      turnToList: "Trasforma in elenco",
      translateToEnglish: "Traduci in inglese",
      empathize: "Rendi più empatico",
    },
  },
  languageSwitch: {
    label: "Lingua di dettatura",
    auto: "Auto",
    detectAmong: "Rileva tra {{languages}}",
  },
};

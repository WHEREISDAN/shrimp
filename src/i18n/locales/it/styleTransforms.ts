import type { StyleTransformsCatalog } from "../en";

/** Italian: the Style and Transforms pages. */
export const styleTransforms: StyleTransformsCatalog = {
  style: {
    title: "Stile",
    cleanup: {
      title: "Pulizia automatica",
      none: "Nessuna",
      noneBody: "Incolla la trascrizione grezza, con intercalari, false partenze ed errori.",
      light: "Leggera",
      lightBody: "Rimuove gli intercalari e corregge la grammatica.",
      medium: "Media",
      mediumBody:
        "Può riformulare le frasi per renderle più chiare e concise, mantenendo il significato.",
    },
    styles: {
      title: "Stili",
      tabs: "Stili",
      appliesTo: "<label>Si applica a </label>{{apps}}",
      group: "Stile per {{category}}",
      next: "Avanti",
      footnote:
        "Gli stili valgono per le nuove dettature. Gli assistenti IA come ChatGPT e Claude non ricevono formattazione di stile.",
    },
    appFormats: {
      title: "Formattazione per app",
      body: "Scegli come vengono formattate le dettature in un'app. Le app non elencate ricevono la formattazione automatica.",
      app: "App",
      format: "Formato",
      add: "Aggiungi app",
      remove: "Rimuovi",
      removeApp: "Rimuovi {{app}}",
      formatFor: "Formato per {{app}}",
      noApps: "Qui compaiono le app in cui detti.",
      footnote:
        "La trascrizione non formattata salta la modifica con l'IA, la punteggiatura dettata e le maiuscole; a capo, correzioni del dizionario e snippet restano attivi. Tutto minuscolo vale per l'intera dettatura, inizi di frase compresi; gli snippet mantengono il loro testo.",
      formats: {
        automatic: "Formattazione automatica",
        unformatted: "Trascrizione non formattata",
        properNouns: "Maiuscole solo per i nomi propri",
        lowercase: "Tutto minuscolo",
      },
    },
    // Each title is written in its own style: capitals and punctuation show the difference.
    options: {
      formal: { title: "Formale.", description: "Maiuscole + punteggiatura." },
      casual: { title: "Informale", description: "Maiuscole + meno punteggiatura." },
      veryCasual: {
        title: "molto informale",
        description: "Niente maiuscole + meno punteggiatura.",
      },
      excited: { title: "Entusiasta!", description: "Più punti esclamativi." },
    },
    // Within a category the examples use the same words; only capitals and punctuation change.
    categories: {
      personal: {
        label: "Messaggi personali",
        apps: "Messaggi, WhatsApp, Discord, Signal, Telegram, Instagram",
        formal: "Ciao, hai tempo domani per pranzo? Facciamo alle 12 se ti va bene.",
        casual: "Ciao hai tempo domani per pranzo? Facciamo alle 12 se ti va bene",
        veryCasual: "ciao hai tempo domani per pranzo? facciamo alle 12 se ti va bene",
      },
      work: {
        label: "Messaggi di lavoro",
        apps: "Slack, Microsoft Teams, LinkedIn",
        formal:
          "Grazie per l'aggiornamento. I numeri sembrano buoni. Procediamo con la prima opzione.",
        casual:
          "Grazie per l'aggiornamento, i numeri sembrano buoni. Procediamo con la prima opzione",
        excited:
          "Grazie per l'aggiornamento! I numeri sembrano buoni! Procediamo con la prima opzione!",
      },
      email: {
        label: "Email",
        apps: "Mail, Outlook, Missive, Superhuman, Spark, Thunderbird, Mimestream",
        formal:
          "Ciao Dana, grazie per avermi inviato il contratto. Lo leggo oggi e ti rispondo entro venerdì.",
        casual:
          "Ciao Dana grazie per avermi inviato il contratto, lo leggo oggi e ti rispondo entro venerdì",
        excited:
          "Ciao Dana! Grazie per avermi inviato il contratto! Lo leggo oggi e ti rispondo entro venerdì!",
      },
      other: {
        label: "Altro",
        apps: "Tutto il resto, inclusi assistenti IA e terminali.",
        formal: "La build è finita e tutti i test sono passati, quindi faccio il merge adesso.",
        casual: "La build è finita e tutti i test sono passati quindi faccio il merge adesso",
        excited: "La build è finita e tutti i test sono passati quindi faccio il merge adesso!",
      },
    },
  },
  transforms: {
    title: "Trasformazioni",
    heading: "Riscrivi qualsiasi testo con l'IA",
    toggle: "Trasformazioni",
    toggleBody:
      "Seleziona del testo ovunque e premi la scorciatoia di una trasformazione: il risultato lo sostituisce e “Vedi modifiche” mostra cosa è cambiato. Il testo viene inviato al tuo provider IA.",
    newTransform: "Nuova trasformazione",
    afterDictation: "Dopo la dettatura",
    autoApply: "Applica una trasformazione dopo ogni dettatura",
    off: "Nessuna",
    autoApplyBody: "Se la trasformazione non riesce, la dettatura viene incollata così com'era.",
    seeChanges: "Vedi modifiche",
    reset: "Ripristina",
    polish: {
      title: "Rifinisci",
      body: "Riscrive il testo selezionato, il campo di testo o la tua ultima dettatura dell'ultimo minuto.",
      concise: "Rendi più conciso",
      conciseBody: "Elimina riempitivi, ripetizioni e parti superflue.",
      clarity: "Riformula per chiarezza",
      clarityBody: "Sostituisce le formulazioni vaghe o goffe.",
      reorder: "Riordina per leggibilità",
      reorderBody: "Mette prima il punto principale e raggruppa i punti correlati.",
      structure: "Aggiungi struttura per leggibilità",
      structureBody: "Paragrafi brevi ed elenchi per le enumerazioni.",
      tone: "Mantieni il tuo tono",
      toneBody: "Lascia la tua voce così com'è.",
      instructions: "Istruzioni personalizzate",
      remove: "Rimuovi",
      removeInstruction: "Rimuovi {{instruction}}",
      newInstruction: "Nuova istruzione",
      placeholder: "Ad esempio: evita gli anglicismi",
      add: "Aggiungi",
      tooLong_one: "Ogni istruzione deve avere meno di {{count, number}} parola.",
      tooLong_many: "Ogni istruzione deve avere meno di {{count, number}} parole.",
      tooLong_other: "Ogni istruzione deve avere meno di {{count, number}} parole.",
      tooLong_zero: "Ogni istruzione deve avere meno di {{count, number}} parole.",
      duplicate: "Questa istruzione è già nell'elenco.",
    },
    promptEngineer: {
      title: "Prompt Engineer",
      body: "Trasforma pensieri abbozzati in un prompt strutturato per un assistente IA.",
      prompt: "Prompt",
    },
    custom: {
      fallbackName: "Trasformazione {{position, number}}",
      name: "Nome",
      prompt: "Prompt",
      placeholder: "Ad esempio: riscrivilo come una risposta cordiale a un cliente",
      incomplete: "Assegnale un nome e un prompt per usarla.",
      samples: "Esempi di scrittura",
      samplesBody: "Fino a 5 testi da 50 a 500 parole, di cui il risultato imita lo stile.",
      sample: "Esempio {{position, number}}",
      words_one: "{{count, number}} parola",
      words_many: "{{count, number}} parole",
      words_other: "{{count, number}} parole",
      words_zero: "{{count, number}} parole",
      removeSample: "Rimuovi esempio",
      addSample: "Aggiungi esempio",
    },
  },
};

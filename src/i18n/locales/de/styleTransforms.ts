import type { StyleTransformsCatalog } from "../en";

/** German: the Style and Transforms pages. */
export const styleTransforms: StyleTransformsCatalog = {
  style: {
    title: "Stil",
    cleanup: {
      title: "Automatische Bereinigung",
      none: "Keine",
      noneBody:
        "Fügt das Rohtranskript ein, mit Füllwörtern, abgebrochenen Ansätzen und Versprechern.",
      light: "Leicht",
      lightBody: "Entfernt Füllwörter und korrigiert die Grammatik.",
      medium: "Mittel",
      mediumBody: "Formuliert bei Bedarf klarer und knapper um, ohne die Bedeutung zu ändern.",
    },
    styles: {
      title: "Stile",
      tabs: "Stile",
      appliesTo: "<label>Gilt für </label>{{apps}}",
      group: "Stil: {{category}}",
      next: "Weiter",
      footnote:
        "Stile gelten für neue Diktate. KI-Assistenten wie ChatGPT und Claude erhalten keine Stilformatierung.",
    },
    // Each title is written in its own style: capitals and punctuation show the difference.
    options: {
      formal: { title: "Formell.", description: "Großschreibung + Satzzeichen." },
      casual: { title: "Locker", description: "Großschreibung + weniger Satzzeichen." },
      veryCasual: { title: "sehr locker", description: "Kleinschreibung + weniger Satzzeichen." },
      excited: { title: "Begeistert!", description: "Mehr Ausrufezeichen." },
    },
    // Within a category the examples use the same words; only capitals and punctuation change.
    categories: {
      personal: {
        label: "Persönliche Nachrichten",
        apps: "Nachrichten, WhatsApp, Discord, Signal, Telegram, Instagram",
        formal: "Hey, hast du morgen Zeit zum Mittagessen? Sagen wir um 12, wenn es dir passt.",
        casual: "Hey hast du morgen Zeit zum Mittagessen? Sagen wir um 12 wenn es dir passt",
        veryCasual: "hey hast du morgen zeit zum mittagessen? sagen wir um 12 wenn es dir passt",
      },
      work: {
        label: "Arbeitsnachrichten",
        apps: "Slack, Microsoft Teams, LinkedIn",
        formal: "Danke für das Update. Die Zahlen sehen gut aus. Nehmen wir die erste Option.",
        casual: "Danke für das Update, die Zahlen sehen gut aus. Nehmen wir die erste Option",
        excited: "Danke für das Update! Die Zahlen sehen gut aus! Nehmen wir die erste Option!",
      },
      email: {
        label: "E-Mail",
        apps: "Mail, Outlook, Missive, Superhuman, Spark, Thunderbird, Mimestream",
        formal:
          "Hallo Dana, danke für den Vertrag. Ich sehe ihn mir heute an und melde mich bis Freitag bei dir.",
        casual:
          "Hallo Dana danke für den Vertrag, ich sehe ihn mir heute an und melde mich bis Freitag bei dir",
        excited:
          "Hallo Dana! Danke für den Vertrag! Ich sehe ihn mir heute an und melde mich bis Freitag bei dir!",
      },
      other: {
        label: "Sonstiges",
        apps: "Alles andere, auch KI-Assistenten und Terminals.",
        formal: "Der Build ist fertig und alle Tests sind grün, also merge ich jetzt.",
        casual: "Der Build ist fertig und alle Tests sind grün also merge ich jetzt",
        excited: "Der Build ist fertig und alle Tests sind grün also merge ich jetzt!",
      },
    },
  },
  transforms: {
    title: "Transformationen",
    heading: "Beliebigen Text mit KI umschreiben",
    toggle: "Transformationen",
    toggleBody:
      "Wähle irgendwo Text aus und drück den Kurzbefehl einer Transformation: Das Ergebnis ersetzt ihn, und „Änderungen ansehen“ zeigt, was sich geändert hat. Dein Text wird an deinen KI-Anbieter gesendet.",
    newTransform: "Neue Transformation",
    afterDictation: "Nach dem Diktat",
    autoApply: "Nach jedem Diktat eine Transformation anwenden",
    off: "Aus",
    autoApplyBody: "Schlägt die Transformation fehl, wird dein Diktat unverändert eingefügt.",
    seeChanges: "Änderungen ansehen",
    reset: "Zurücksetzen",
    polish: {
      title: "Feinschliff",
      body: "Überarbeitet den ausgewählten Text, das Textfeld oder dein letztes Diktat der vergangenen Minute.",
      concise: "Kürzer fassen",
      conciseBody: "Streicht Füllwörter, Wiederholungen und Überflüssiges.",
      clarity: "Klarer formulieren",
      clarityBody: "Ersetzt vage oder holprige Formulierungen.",
      reorder: "Für bessere Lesbarkeit umstellen",
      reorderBody: "Stellt das Wichtigste an den Anfang und Zusammengehöriges zusammen.",
      structure: "Für bessere Lesbarkeit gliedern",
      structureBody: "Kurze Absätze und Listen für Aufzählungen.",
      tone: "Deinen Ton beibehalten",
      toneBody: "Lässt deine Ausdrucksweise, wie sie ist.",
      instructions: "Eigene Anweisungen",
      remove: "Entfernen",
      removeInstruction: "„{{instruction}}“ entfernen",
      newInstruction: "Neue Anweisung",
      placeholder: "Zum Beispiel: Verwende Schweizer Rechtschreibung",
      add: "Hinzufügen",
      tooLong_one: "Halte jede Anweisung unter {{count, number}} Wort.",
      tooLong_many: "Halte jede Anweisung unter {{count, number}} Wörtern.",
      tooLong_other: "Halte jede Anweisung unter {{count, number}} Wörtern.",
      tooLong_zero: "Halte jede Anweisung unter {{count, number}} Wörtern.",
      duplicate: "Diese Anweisung steht schon in der Liste.",
    },
    promptEngineer: {
      title: "Prompt Engineer",
      body: "Macht aus groben Gedanken einen strukturierten Prompt für einen KI-Assistenten.",
      prompt: "Prompt",
    },
    custom: {
      fallbackName: "Transformation {{position, number}}",
      name: "Name",
      prompt: "Prompt",
      placeholder: "Zum Beispiel: Formuliere das als freundliche Antwort an einen Kunden",
      incomplete: "Gib ihr einen Namen und einen Prompt, um sie zu verwenden.",
      samples: "Textbeispiele",
      samplesBody: "Bis zu 5 Beispiele mit 50 bis 500 Wörtern, deren Stil das Ergebnis übernimmt.",
      sample: "Beispiel {{position, number}}",
      words_one: "{{count, number}} Wort",
      words_many: "{{count, number}} Wörter",
      words_other: "{{count, number}} Wörter",
      words_zero: "{{count, number}} Wörter",
      removeSample: "Beispiel entfernen",
      addSample: "Beispiel hinzufügen",
    },
  },
};

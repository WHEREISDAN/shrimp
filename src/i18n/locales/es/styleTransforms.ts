import type { StyleTransformsCatalog } from "../en";

/** Spanish: the Style and Transforms pages. */
export const styleTransforms: StyleTransformsCatalog = {
  style: {
    title: "Estilo",
    cleanup: {
      title: "Limpieza automática",
      none: "Ninguna",
      noneBody:
        "Pega la transcripción sin editar, con muletillas, arranques en falso y errores al hablar.",
      light: "Ligera",
      lightBody: "Quita las muletillas y corrige la gramática.",
      medium: "Media",
      mediumBody: "Puede reformular frases para ganar claridad y concisión sin cambiar el sentido.",
    },
    styles: {
      title: "Estilos",
      tabs: "Estilos",
      appliesTo: "<label>Se aplica a </label>{{apps}}",
      group: "Estilo para {{category}}",
      next: "Siguiente",
      footnote:
        "Los estilos se aplican a los dictados nuevos. Los asistentes de IA como ChatGPT y Claude no reciben formato de estilo.",
    },
    appFormats: {
      title: "Formato por app",
      body: "Elige cómo se da formato a los dictados en una app. Las apps que no estén en la lista reciben formato automático.",
      app: "App",
      format: "Formato",
      add: "Añadir app",
      remove: "Quitar",
      removeApp: "Quitar {{app}}",
      formatFor: "Formato para {{app}}",
      noApps: "Aquí aparecen las apps en las que dictas.",
      footnote:
        "La transcripción sin formato omite la edición con IA, la puntuación dictada y las mayúsculas; los saltos de línea, las correcciones del diccionario y los fragmentos se siguen aplicando. Todo en minúsculas afecta a todo el dictado, incluidos los inicios de frase; los fragmentos conservan su propio texto.",
      formats: {
        automatic: "Formato automático",
        unformatted: "Transcripción sin formato",
        properNouns: "Mayúsculas solo en nombres propios",
        lowercase: "Todo en minúsculas",
      },
    },
    // Each title is written in its own style: capitals and punctuation show the difference.
    options: {
      formal: { title: "Formal.", description: "Mayúsculas + puntuación." },
      casual: { title: "Informal", description: "Mayúsculas + menos puntuación." },
      veryCasual: { title: "muy informal", description: "Sin mayúsculas + menos puntuación." },
      excited: { title: "¡Entusiasta!", description: "Más exclamaciones." },
    },
    // Within a category the examples use the same words; only capitals and punctuation change.
    categories: {
      personal: {
        label: "Mensajes personales",
        apps: "Mensajes, WhatsApp, Discord, Signal, Telegram, Instagram",
        formal: "Hola, ¿estás libre para comer mañana? Podemos vernos a las 12 si puedes.",
        casual: "Hola ¿estás libre para comer mañana? Podemos vernos a las 12 si puedes",
        veryCasual: "hola ¿estás libre para comer mañana? podemos vernos a las 12 si puedes",
      },
      work: {
        label: "Mensajes de trabajo",
        apps: "Slack, Microsoft Teams, LinkedIn",
        formal: "Gracias por el informe. Las cifras son buenas. Vamos con la primera opción.",
        casual: "Gracias por el informe, las cifras son buenas. Vamos con la primera opción",
        excited: "¡Gracias por el informe! ¡Las cifras son buenas! ¡Vamos con la primera opción!",
      },
      email: {
        label: "Correo",
        apps: "Mail, Outlook, Missive, Superhuman, Spark, Thunderbird, Mimestream",
        formal:
          "Hola, Dana. Gracias por enviarme el contrato. Lo reviso hoy y te respondo antes del viernes.",
        casual:
          "Hola Dana gracias por enviarme el contrato, lo reviso hoy y te respondo antes del viernes",
        excited:
          "¡Hola, Dana! ¡Gracias por enviarme el contrato! ¡Lo reviso hoy y te respondo antes del viernes!",
      },
      other: {
        label: "Otros",
        apps: "Todo lo demás, incluidos los asistentes de IA y los terminales.",
        formal: "La compilación terminó y pasaron todas las pruebas, así que lo fusiono ya.",
        casual: "La compilación terminó y pasaron todas las pruebas así que lo fusiono ya",
        excited: "¡La compilación terminó y pasaron todas las pruebas así que lo fusiono ya!",
      },
    },
  },
  transforms: {
    title: "Transformaciones",
    heading: "Reescribe cualquier texto con IA",
    toggle: "Transformaciones",
    toggleBody:
      "Selecciona texto en cualquier lugar y pulsa el atajo de una transformación: el resultado lo sustituye y “Ver cambios” muestra qué ha cambiado. Tu texto se envía a tu proveedor de IA.",
    newTransform: "Nueva transformación",
    afterDictation: "Después de dictar",
    autoApply: "Aplicar una transformación después de cada dictado",
    off: "Ninguna",
    autoApplyBody: "Si la transformación falla, tu dictado se pega tal cual.",
    seeChanges: "Ver cambios",
    reset: "Restablecer",
    polish: {
      title: "Pulir",
      body: "Reescribe el texto seleccionado, el del cuadro de texto o tu último dictado si es del último minuto.",
      concise: "Hacerlo más conciso",
      conciseBody: "Quita relleno, repeticiones y rodeos.",
      clarity: "Reformular para ganar claridad",
      clarityBody: "Sustituye expresiones vagas o torpes.",
      reorder: "Reordenar para leerlo mejor",
      reorderBody: "Pone primero la idea principal y agrupa las relacionadas.",
      structure: "Añadir estructura para leerlo mejor",
      structureBody: "Párrafos cortos y listas para las enumeraciones.",
      tone: "Mantener tu tono",
      toneBody: "Conserva tu voz tal como es.",
      instructions: "Instrucciones personalizadas",
      remove: "Quitar",
      removeInstruction: "Quitar {{instruction}}",
      newInstruction: "Nueva instrucción",
      placeholder: "Por ejemplo: usa “usted” en vez de “tú”",
      add: "Añadir",
      tooLong_one: "Cada instrucción debe tener menos de {{count, number}} palabra.",
      tooLong_many: "Cada instrucción debe tener menos de {{count, number}} palabras.",
      tooLong_other: "Cada instrucción debe tener menos de {{count, number}} palabras.",
      tooLong_zero: "Cada instrucción debe tener menos de {{count, number}} palabras.",
      duplicate: "Esa instrucción ya está en la lista.",
    },
    promptEngineer: {
      title: "Prompt Engineer",
      body: "Convierte ideas sueltas en un prompt estructurado para un asistente de IA.",
      prompt: "Prompt",
    },
    custom: {
      fallbackName: "Transformación {{position, number}}",
      name: "Nombre",
      prompt: "Prompt",
      placeholder: "Por ejemplo: reescríbelo como una respuesta amable a un cliente",
      incomplete: "Ponle un nombre y un prompt para poder usarla.",
      samples: "Muestras de escritura",
      samplesBody: "Hasta 5 muestras de 50 a 500 palabras cuyo estilo imitará el resultado.",
      sample: "Muestra {{position, number}}",
      words_one: "{{count, number}} palabra",
      words_many: "{{count, number}} palabras",
      words_other: "{{count, number}} palabras",
      words_zero: "{{count, number}} palabras",
      removeSample: "Quitar muestra",
      addSample: "Añadir muestra",
    },
  },
};

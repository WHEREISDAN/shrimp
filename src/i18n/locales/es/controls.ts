import type { ControlsCatalog } from "../en";

/** Spanish: microphone, languages, shortcuts, shared dialog buttons and the Flow Bar's cards and menu. */
export const controls: ControlsCatalog = {
  microphone: {
    title: "Micrófono",
    autoDetect: "Detección automática",
    autoDetectHint: "usa el micrófono conectado con mayor prioridad",
    list: "Micrófonos",
    builtIn: "{{name}} (integrado, recomendado)",
    notConnected: "No conectado",
    inUse: "En uso",
    connected: "Conectado",
    moveUp: "Subir {{name}}",
    moveDown: "Bajar {{name}}",
    forget: "Olvidar",
    forgetDevice: "Olvidar {{name}}",
    none: "No se detectó ningún micrófono.",
    showOthers: "Mostrar otros dispositivos",
    showOthersHint: "(entradas virtuales y loopback; nunca se eligen automáticamente)",
    inputLevel: "Nivel de entrada",
    level: "Nivel del micrófono",
    notSure: "¿No sabes qué micrófono usar?",
  },
  micAdvice: {
    title: "Encuentra el micrófono adecuado",
    progress: "Pregunta {{current, number}} de {{total, number}}",
    startOver: "Volver a empezar",
    questions: {
      place: {
        label: "¿Dónde dictas más?",
        quiet: "En una habitación tranquila",
        shared: "En una oficina compartida o una cafetería",
        moving: "En movimiento",
      },
      computer: {
        label: "¿En qué Mac dictas?",
        laptop: "Un MacBook",
        desktop: "Un Mac de escritorio (mini, iMac, Studio)",
      },
      headphones: {
        label: "¿Usas auriculares mientras dictas?",
        none: "No",
        wired: "Auriculares con cable o con micrófono",
        bluetooth: "AirPods u otros auriculares Bluetooth",
      },
    },
    titles: {
      wiredHeadset: "Unos auriculares con micrófono y cable",
      wiredHeadsetOrEarbuds: "Auriculares con micrófono y cable, de diadema o de botón",
      yourWiredHeadset: "Tus auriculares con cable",
      headsetOrUsb: "Unos auriculares con micrófono o un micrófono USB cerca de ti",
      usbMicrophone: "Un micrófono USB en tu mesa",
      builtIn: "El micrófono integrado de tu MacBook",
    },
    why: {
      moving:
        "Un micrófono que va contigo se mantiene cerca de tu boca estés donde estés, y el ruido de fondo sigue siendo bajo.",
      shared: "En una sala ruidosa, un micrófono cerca de tu boca te oye a ti y no a la sala.",
      desktop:
        "Los Mac de escritorio, salvo el iMac, no tienen micrófono propio, y un micrófono USB da un audio claro y con poco ruido.",
      quiet:
        "En una habitación tranquila suena claro, siempre está disponible y se activa al instante.",
    },
    tips: {
      bluetooth:
        "Usa los auriculares Bluetooth para escuchar: sus micrófonos suenan apagados, tardan en activarse y pasan tu audio a un modo de baja calidad.",
      whisper: "Puedes hablar en voz baja: Shrimp realza la voz suave y los susurros.",
      moving:
        "Elige ese micrófono en el selector, o ponlo el primero de la lista para que la detección automática lo use.",
      shared:
        "Pon ese micrófono el primero de la lista para que la detección automática lo elija siempre que esté conectado.",
      desktop: "Colócalo a 15–30 cm de tu boca, un poco hacia un lado.",
      quiet: "Con la tapa cerrada no te oye, así que en ese caso conecta otro micrófono.",
    },
  },
  languages: {
    title: "Idiomas",
    autoDetect: "Detección automática",
    autoDetectBody: "Transcribe cualquier idioma que hables.",
    change: "Cambiar",
    body: "Con varios idiomas, cada dictado se transcribe en el que hables. Las variantes de inglés, chino y alemán definen la ortografía y los caracteres. Pasa el puntero sobre la Flow Bar para cambiar de idioma.",
    search: "Buscar idiomas",
    noneSelected: "Selecciona al menos un idioma o activa la detección automática",
  },
  shortcuts: {
    title: "Atajos",
    busy: "Primero termina o cancela el atajo que estás registrando.",
    secureInput:
      "La entrada de teclado segura está activada (según macOS, por {{app}}), así que los atajos con letras u otras teclas no funcionan hasta que se desactive. fn y los atajos solo con teclas modificadoras siguen funcionando.",
    resetConfirm:
      "¿Restablecer todos los atajos a sus valores predeterminados? No se puede deshacer.",
    reset: "Restablecer",
    resetToDefault: "Restablecer valores predeterminados",
    change: "{{action}}: {{shortcut}}. Cambiar",
    remove: "Quitar el atajo {{shortcut}} de {{action}}",
    removeTitle: "Quitar atajo",
    add: "Haz clic para añadir un atajo",
    addAnother: "Añadir otro atajo para {{action}}",
    addAnotherTitle: "Añadir otro",
    pressKey: "Pulsa una tecla o un botón del ratón",
    cancelRecording: "Cancelar el registro del atajo",
    saved: "Atajo guardado",
    notSet: "Sin asignar",
    recorder: "Atajo para {{label}}: {{shortcut}}",
    removeOne: "Quitar",
    removeOneLabel: "Quitar el atajo para {{label}}",
  },
  dialog: {
    done: "Listo",
    save: "Guardar",
  },
  wand: {
    label: "Varita de transformaciones",
    run: "Aplicar {{name}} a la selección",
    choose: "Elegir una transformación",
    menu: "Menú de la varita de transformaciones",
    autoApply: "Aplicar automáticamente tras el dictado",
    configure: "Configurar transformaciones",
  },
  flowBarMenu: {
    label: "Menú de la Flow Bar",
    microphone: "Micrófono ›",
    pasteLast: "Pegar la última transcripción",
    history: "Historial de transcripciones",
    settings: "Ajustes",
    hide: "Ocultar durante 1 hora",
  },
  answer: {
    region: "Respuesta de Command Mode",
    dismiss: "Cerrar",
    copied: "Copiado ✓",
  },
  changes: {
    region: "Cambios de {{name}}",
    retry: "Reintentar",
    undo: "Deshacer",
    accept: "Aceptar cambios",
    quick: {
      label: "Transformar de nuevo",
      promptEngineer: "Prompt Engineer",
      turnToList: "Convertir en lista",
      translateToEnglish: "Traducir al inglés",
      empathize: "Hacerlo más empático",
    },
  },
  languageSwitch: {
    label: "Idioma de dictado",
    auto: "Auto",
    detectAmong: "Detectar entre {{languages}}",
  },
};

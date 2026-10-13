import type { SettingsSectionsCatalog } from "../en";

/** Spanish: settings sections (AI provider, speech engine, sound, Flow Bar, experimental, vibe coding, shortcuts). */
export const settingsSections: SettingsSectionsCatalog = {
  aiProvider: {
    title: "Proveedor de IA",
    provider: "Proveedor",
    model: "Modelo",
    ollamaNote: "Ollama funciona en este Mac; nada sale de él.",
    cloudNote:
      "El texto de tus dictados se envía a {{provider}} para limpiarlo, con tu propia clave.",
    test: "Probar",
    testing: "Probando…",
    testOk: "Limpiado en {{latency, number}} ms: “{{output}}”",
    testFailed: "No funcionó.",
  },
  ollama: {
    address: "Dirección",
    model: "Modelo de Ollama",
    automatic: "Automático (recomendado)",
    notRunning: "Ollama no se está ejecutando en {{url}}.",
    retry: "Reintentar",
  },
  apiKey: {
    saved: "Clave guardada en tu llavero",
    none: "Aún no hay clave",
    label: "Clave de API de {{company}}",
    placeholder: "Pega la clave de API",
    save: "Guardar clave",
    remove: "Eliminar clave",
  },
  speechEngine: {
    title: "Motor de voz",
    model: "Modelo de voz",
    note: "Whisper en este Mac es privado y funciona sin conexión. Los motores en la nube envían tu audio a OpenAI o Groq; si uno falla, Shrimp recurre a Whisper en este Mac.",
  },
  whisperModels: {
    title: "Modelo de Whisper",
    list: "Modelos de Whisper",
    size: "{{size, number}} MB",
    recommended: "Recomendado",
    waiting: "Esperando para descargar",
    notDownloaded: "No descargado",
    downloading: "Descargando: {{percent, number}} %",
    downloaded: "Descargado",
    inUse: "En uso",
    download: "Descargar",
    deleteModel: "Eliminar {{model}}",
    note: "Los modelos más grandes son más precisos y más lentos. Si eliges otro, se descarga si hace falta y se usa a partir de tu próximo dictado.",
  },
  modelCard: {
    title: "Modelo de voz",
    onDevice: "{{size, number}} MB · en este Mac",
    waiting: "Esperando para descargar…",
    progress: "Progreso de la descarga del modelo",
    downloading:
      "Descargando: {{percent, number}} % ({{received, number}} de {{total, number}} MB)",
    loading: "Descargado. Cargando…",
    ready: "Listo. El dictado funciona por completo en este Mac.",
    retry: "Reintentar",
  },
  permissionsCard: {
    title: "Permisos",
    microphoneWhy: "Te oye mientras dictas.",
    accessibilityWhy: "Detecta el atajo de dictado y pega el texto.",
    allow: "Permitir…",
    openSettings: "Abrir Ajustes",
  },
  sound: {
    title: "Sonido",
    dictation: "Sonidos de dictado",
    dictationBody:
      "Un sonido cuando el dictado empieza, termina, pasa a manos libres o se cancela.",
    notifications: "Sonidos de notificación",
    notificationsBody: "Un sonido con los avisos y los errores.",
    mute: "Silenciar el audio al dictar",
    muteBody:
      "Si algo se está reproduciendo al empezar, el Mac queda en silencio hasta que termines.",
    pause: "Pausar la música al dictar",
    pauseBody:
      "Pausa Música, Spotify y otros reproductores (nunca las llamadas) y los reanuda al terminar. Sustituye a silenciar el audio.",
  },
  flowBarSettings: {
    title: "Flow Bar",
    alwaysShow: "Mostrar siempre la Flow Bar",
    alwaysShowBody:
      "Si está desactivado, la barra solo aparece mientras dictas o cuando Shrimp tiene algo que decirte.",
    hiddenUntil: "Oculta hasta las {{time}}",
    showNow: "Mostrar ahora",
    note: "Arrastra la barra al borde inferior, izquierdo o derecho de la pantalla; te sigue a la pantalla en la que esté el puntero. Haz clic derecho en ella para elegir el micrófono y más.",
  },
  experimental: {
    title: "Experimental",
    commandMode: "Command Mode",
    commandModeBody:
      "Selecciona texto, mantén pulsado {{shortcut}} y di qué cambiar, como “hazlo más formal” o “tradúcelo al inglés”. Sin nada seleccionado, haz una pregunta.",
    pressEnter: "Comando de voz “press enter”",
    pressEnterBody:
      "Termina un dictado con “press enter” para enviarlo: Shrimp omite esas palabras y pulsa Enter después de pegar.",
  },
  commandModeIntro: {
    title: "Cómo usar Command Mode (beta)",
    step1: "Selecciona el texto que quieras cambiar o pon el cursor en un campo de texto.",
    step2:
      "Mantén pulsado <kbd>{{shortcut}}</kbd> y di qué hacer: “hazlo más corto”, “tradúcelo al inglés”, “conviértelo en una lista”.",
    step3: "Suelta. El resultado sustituye la selección; Cmd+Z lo deshace.",
    body: "Sin nada seleccionado, haz una pregunta y la respuesta aparecerá junto a la Flow Bar. Empieza con “search” para abrir una búsqueda web. Pulsa dos veces el atajo para seguir escuchando en modo manos libres; Esc cancela. Tu selección y el campo en el que estás se envían a tu proveedor de IA.",
    disable: "Desactivar",
    gotIt: "Entendido",
  },
  vibeCoding: {
    title: "Vibe coding",
    fileTagging: "Etiquetar archivos en el chat",
    fileTaggingBody:
      "En el chat de Cursor y Windsurf, di el nombre de un archivo (“index dot ts”) o “tag” y su nombre, y Shrimp escribe una mención con @. Funciona con archivos abiertos o visibles en la barra lateral.",
    variables: "Reconocimiento de variables",
    variablesBody:
      "En Cursor, VS Code y Windsurf, di los nombres con naturalidad, “user ID” para <code>userId</code>: Shrimp los asocia con el código en pantalla. Di “camel case”, “snake case”, “pascal case” o “kebab case” antes de las palabras para crear un nombre nuevo.",
    step1: "Abre la paleta de comandos (Cmd+Mayúsculas+P).",
    step2: "Ejecuta “Toggle Screen Reader Accessibility Mode”.",
    step3: "Comprueba que aparece “Screen Reader Optimized” en la barra de estado.",
  },
  contextAwareness: {
    title: "Reconocimiento del contexto",
    body: "Shrimp lee el texto alrededor del cursor y los nombres en pantalla, para escribir bien los nombres y adaptar el estilo a la app. Si está desactivado, nada de tu pantalla llega al motor de voz ni al proveedor de IA; las mayúsculas y los espacios se siguen adaptando al texto junto al cursor, en este Mac.",
  },
  shortcutsSection: {
    title: "Atajos",
    pushToTalk: "Pulsar para hablar",
    change: "Cambiar",
    note: "Cada acción admite hasta cuatro atajos, incluidos botones del ratón. “Pulsar Enter” envía Enter a la app en la que estás; mantén pulsado Mayúsculas u otra tecla modificadora a la vez para enviar Mayúsculas+Enter.",
  },
};

import type { WindowsCatalog } from "../en";

export const windows: WindowsCatalog = {
  settings: { pageHint: "Alt+↑ y Alt+↓ cambian de página." },
  appLanguage: { system: "El mismo que Windows" },
  system: {
    launchAtLoginBody:
      "Shrimp se abre en segundo plano al iniciar sesión y queda disponible en el área de notificación.",
    resetBody:
      "Elimina los datos locales de Shrimp en este PC y reinicia la app. Úsalo si Shrimp sigue dando problemas.",
    resetConfirmBody:
      "Se eliminarán el historial, las grabaciones, el diccionario, los fragmentos, las estadísticas, los ajustes y las claves de API de este PC, y Shrimp se reiniciará. Los modelos de voz descargados se conservan.",
  },
  help: {
    about:
      "Shrimp {{version}}. Todo lo que dictas se queda en este PC, salvo que elijas un motor o proveedor en la nube.",
  },
  onboarding: {
    welcomeBody:
      "Habla dondequiera que puedas escribir. Mantén pulsada una tecla, habla y suelta: Shrimp escribe lo que dices con Whisper en este PC, lo pule y lo pega donde está el cursor.",
    permissionsBody:
      "Shrimp necesita tu micrófono. La fila se pone en verde en cuanto Windows lo permite.",
    permissionsNote:
      "Windows no pide permisos para los atajos ni para pegar. Las apps que se ejecutan como administrador solo reciben texto si Shrimp también se ejecuta como administrador.",
    permission: { openSettings: "Abrir Configuración" },
    engineBody:
      "Shrimp transcribe en este PC con Whisper. El modelo se descarga una vez y funciona sin conexión.",
    aiOllamaBody: "Un modelo en este PC a través de Ollama. Nada sale de tu PC.",
    reverifyBody:
      "Windows ya no deja que Shrimp use un permiso que tenía. Vuelve a concederlo para seguir dictando.",
    reverifyHint:
      "En Configuración › Privacidad › Micrófono, comprueba que las aplicaciones de escritorio puedan usar el micrófono.",
  },
  voice: {
    privacy:
      "Tus dictados solo se guardan en este PC. Al generarlo, se envían tus estadísticas y una muestra de transcripciones recientes a {{provider}}.",
  },
  history: { searchPlaceholder: "Buscar (Ctrl+F o /)" },
  privacy: {
    privacyModeBody:
      "Solo funcionan Whisper en este PC y Ollama: ningún audio ni texto va a un motor o proveedor en la nube. La Flow Bar y el icono del área de notificación muestran un punto mientras está activado.",
    privacyModeOn:
      "El modo de privacidad está activado, así que solo funcionan Whisper en este PC y Ollama. Lo que elijas aquí se volverá a aplicar cuando lo desactives.",
    neverBody:
      "Se eliminan ahora todas las transcripciones, grabaciones y copias de seguridad de este PC, y los dictados nuevos no se guardan. Escribe “delete all” para confirmar.",
    deleteAllBody:
      "Elimina todas las transcripciones y grabaciones de este PC. Las estadísticas, tu diccionario, los fragmentos y los ajustes se conservan.",
  },
  aiProvider: { ollamaNote: "Ollama funciona en este PC; nada sale de él." },
  apiKey: { saved: "Clave guardada en el Administrador de credenciales de Windows" },
  speechEngine: {
    note: "Whisper en este PC es privado y funciona sin conexión. Los motores en la nube envían tu audio a OpenAI o Groq; si uno falla, Shrimp recurre a Whisper en este PC.",
  },
  modelCard: { ready: "Listo. El dictado funciona por completo en este PC." },
  sound: {
    muteBody:
      "Las apps que estén reproduciendo algo al empezar quedan en silencio hasta que termines.",
  },
  commandModeIntro: {
    step3: "Suelta. El resultado sustituye la selección; Ctrl+Z lo deshace.",
  },
  vibeCoding: { step1: "Abre la paleta de comandos (Ctrl+Mayús+P)." },
  contextAwareness: {
    body: "Shrimp lee el texto alrededor del cursor y los nombres en pantalla, para escribir bien los nombres y adaptar el estilo a la app. Si está desactivado, nada de tu pantalla llega al motor de voz ni al proveedor de IA; las mayúsculas y los espacios se siguen adaptando al texto junto al cursor, en este PC.",
  },
  micAdvice: {
    questions: {
      computer: {
        laptop: "Un portátil",
        desktop: "Un PC de escritorio",
      },
    },
    titles: { builtIn: "El micrófono integrado de tu portátil" },
    why: {
      desktop:
        "La mayoría de los PC de escritorio no tienen micrófono propio, y un micrófono USB da un audio claro y con poco ruido.",
    },
  },
};

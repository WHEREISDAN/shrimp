import type { WindowsCatalog } from "../en";

export const windows: WindowsCatalog = {
  settings: { pageHint: "Alt+↑ e Alt+↓ alternam entre as páginas." },
  appLanguage: { system: "Igual ao do Windows" },
  system: {
    launchAtLoginBody:
      "O Shrimp abre em segundo plano quando você entra na sessão, pronto na área de notificação.",
    resetBody:
      "Apaga os dados locais do Shrimp neste PC e o reinicia. Use se o Shrimp continuar com problemas.",
    resetConfirmBody:
      "Isso apaga seu histórico, gravações, dicionário, trechos, estatísticas, ajustes e chaves de API neste PC e depois reinicia o Shrimp. Os modelos de fala baixados são mantidos.",
  },
  help: {
    about:
      "Shrimp {{version}}. Tudo o que você dita fica neste PC, a menos que você escolha um mecanismo ou provedor na nuvem.",
  },
  onboarding: {
    welcomeBody:
      "Fale em qualquer lugar onde você digita. Mantenha uma tecla pressionada, fale e solte: o Shrimp escreve o que você disse com o Whisper neste PC, dá uma arrumada e cola onde está o cursor.",
    permissionsBody:
      "O Shrimp precisa do seu microfone. A linha fica verde assim que o Windows permitir.",
    permissionsNote:
      "O Windows não pede permissão para atalhos nem para colar. Apps executados como administrador só recebem texto se o Shrimp também for executado como administrador.",
    permission: { openSettings: "Abrir Configurações" },
    engineBody:
      "O Shrimp transcreve neste PC com o Whisper. O modelo é baixado uma vez e funciona offline.",
    aiOllamaBody: "Um modelo neste PC, via Ollama. Nada sai do seu PC.",
    reverifyBody:
      "O Windows não deixa mais o Shrimp usar uma permissão que ele tinha. Conceda-a de novo para continuar ditando.",
    reverifyHint:
      "Em Configurações › Privacidade › Microfone, confira se os apps da área de trabalho podem usar o microfone.",
  },
  voice: {
    privacy:
      "Seus ditados ficam guardados só neste PC. Gerar o perfil envia suas estatísticas e uma amostra de transcrições recentes para {{provider}}.",
  },
  history: { searchPlaceholder: "Buscar (Ctrl+F ou /)" },
  privacy: {
    privacyModeBody:
      "Só o Whisper neste PC e o Ollama funcionam: nenhum áudio ou texto vai para um mecanismo ou provedor na nuvem. A Flow Bar e o ícone na área de notificação mostram um ponto enquanto ele está ativado.",
    privacyModeOn:
      "O modo de privacidade está ativado, então só o Whisper neste PC e o Ollama funcionam. Sua escolha aqui volta a valer quando você desativá-lo.",
    neverBody:
      "Todas as transcrições, gravações e backups neste PC serão apagados agora, e novos ditados não serão guardados. Digite “delete all” para confirmar.",
  },
  aiProvider: { ollamaNote: "O Ollama roda neste PC; nada sai dele." },
  apiKey: { saved: "Chave salva no Gerenciador de Credenciais do Windows" },
  speechEngine: {
    note: "O Whisper neste PC é privado e funciona offline. Os mecanismos na nuvem enviam seu áudio para a OpenAI ou a Groq; se um deles falhar, o Shrimp volta a usar o Whisper neste PC.",
  },
  modelCard: { ready: "Pronto. O ditado roda inteiramente neste PC." },
  sound: {
    muteBody: "Os apps que estiverem tocando quando você começar ficam sem som até você terminar.",
  },
  commandModeIntro: {
    step3: "Solte. O resultado substitui a seleção; Ctrl+Z desfaz.",
  },
  vibeCoding: { step1: "Abra a Paleta de Comandos (Ctrl+Shift+P)." },
  contextAwareness: {
    body: "O Shrimp lê o texto ao redor do cursor e os nomes na tela, para escrever os nomes corretamente e adequar o estilo ao app. Quando desativado, nada da sua tela chega ao mecanismo de fala nem ao provedor de IA; maiúsculas e espaçamento continuam se adaptando ao texto ao redor do cursor, neste PC.",
  },
  micAdvice: {
    questions: {
      computer: {
        laptop: "Um notebook",
        desktop: "Um PC de mesa",
      },
    },
    titles: { builtIn: "O microfone integrado do seu notebook" },
    why: {
      desktop:
        "A maioria dos PCs de mesa não tem microfone próprio, e um microfone USB oferece um áudio claro e com pouco ruído.",
    },
  },
};

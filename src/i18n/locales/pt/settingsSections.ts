import type { SettingsSectionsCatalog } from "../en";

/** Brazilian Portuguese: the Settings sections (AI provider, speech engine, sound, Flow Bar, experimental, vibe coding, shortcuts). */
export const settingsSections: SettingsSectionsCatalog = {
  aiProvider: {
    title: "Provedor de IA",
    provider: "Provedor",
    model: "Modelo",
    ollamaNote: "O Ollama roda neste Mac; nada sai dele.",
    cloudNote:
      "O texto do seu ditado é enviado para {{provider}} para a limpeza, com a sua própria chave.",
    test: "Testar",
    testing: "Testando…",
    testOk: "Limpo em {{latency, number}} ms: “{{output}}”",
    testFailed: "Não funcionou.",
  },
  ollama: {
    address: "Endereço",
    model: "Modelo do Ollama",
    automatic: "Automático (recomendado)",
    notRunning: "O Ollama não está rodando em {{url}}.",
    retry: "Tentar de novo",
  },
  apiKey: {
    saved: "Chave salva no Acesso às Chaves",
    none: "Nenhuma chave ainda",
    label: "Chave de API da {{company}}",
    placeholder: "Cole a chave de API",
    save: "Salvar chave",
    remove: "Remover chave",
  },
  speechEngine: {
    title: "Mecanismo de fala",
    model: "Modelo de fala",
    note: "O Whisper neste Mac é privado e funciona offline. Os mecanismos na nuvem enviam seu áudio para a OpenAI ou a Groq; se um deles falhar, o Shrimp volta a usar o Whisper neste Mac.",
  },
  whisperModels: {
    title: "Modelo do Whisper",
    list: "Modelos do Whisper",
    size: "{{size, number}} MB",
    recommended: "Recomendado",
    waiting: "Aguardando download",
    notDownloaded: "Não baixado",
    downloading: "Baixando {{percent, number}}%",
    downloaded: "Baixado",
    inUse: "Em uso",
    download: "Baixar",
    deleteModel: "Apagar {{model}}",
    note: "Modelos maiores são mais precisos e mais lentos. Uma nova escolha é baixada se necessário e passa a ser usada no seu próximo ditado.",
  },
  modelCard: {
    title: "Modelo de fala",
    onDevice: "{{size, number}} MB · no dispositivo",
    waiting: "Aguardando download…",
    progress: "Progresso do download do modelo",
    downloading: "Baixando {{percent, number}}% ({{received, number}} de {{total, number}} MB)",
    loading: "Baixado. Carregando…",
    ready: "Pronto. O ditado roda inteiramente neste Mac.",
    retry: "Tentar de novo",
  },
  permissionsCard: {
    title: "Permissões",
    microphoneWhy: "Ouve você enquanto você dita.",
    accessibilityWhy: "Detecta o atalho de ditado e cola o texto.",
    allow: "Permitir…",
    openSettings: "Abrir Ajustes",
  },
  sound: {
    title: "Som",
    dictation: "Sons de ditado",
    dictationBody: "Um som quando o ditado começa, para, fica no modo mãos livres ou é cancelado.",
    notifications: "Sons de notificação",
    notificationsBody: "Um som com avisos e erros.",
    mute: "Silenciar o áudio ao ditar",
    muteBody: "Se algo estiver tocando quando você começar, o Mac fica sem som até você terminar.",
    pause: "Pausar a música ao ditar",
    pauseBody:
      "Pausa o app Música, o Spotify e outros players (nunca chamadas) e os retoma quando você termina. Substitui o silenciamento.",
  },
  flowBarSettings: {
    title: "Flow Bar",
    alwaysShow: "Mostrar a Flow Bar o tempo todo",
    alwaysShowBody:
      "Quando desativado, a barra só aparece enquanto você dita ou quando o Shrimp tem algo para dizer.",
    hiddenUntil: "Oculta até {{time}}",
    showNow: "Mostrar agora",
    note: "Arraste a barra para a borda inferior, esquerda ou direita da tela; ela vai junto para a tela onde está o ponteiro. Clique nela com o botão direito para ver o microfone e mais opções.",
  },
  experimental: {
    title: "Experimental",
    commandMode: "Command Mode",
    commandModeBody:
      "Selecione um texto, mantenha {{shortcut}} pressionado e diga o que mudar, como “deixe isto mais formal” ou “traduza para o inglês”. Sem nada selecionado, faça uma pergunta.",
    pressEnter: "Comando de voz “press enter”",
    pressEnterBody:
      "Termine um ditado com “press enter” para enviá-lo: o Shrimp omite essas palavras e pressiona Enter depois de colar.",
  },
  commandModeIntro: {
    title: "Usando o Command Mode (beta)",
    step1: "Selecione o texto que quer mudar ou coloque o cursor em um campo de texto.",
    step2:
      "Mantenha <kbd>{{shortcut}}</kbd> pressionado e diga o que fazer: “deixe isto mais curto”, “traduza para o inglês”, “transforme isto em uma lista”.",
    step3: "Solte. O resultado substitui a seleção; Cmd+Z desfaz.",
    body: "Sem nada selecionado, faça uma pergunta e a resposta aparece ao lado da Flow Bar. Comece com “search” para abrir uma pesquisa na web. Pressione o atalho duas vezes para continuar ouvindo sem as mãos; Esc cancela. Sua seleção e o campo em que você está são enviados ao seu provedor de IA.",
    disable: "Desativar",
    gotIt: "Entendi",
  },
  vibeCoding: {
    title: "Vibe coding",
    fileTagging: "Marcar arquivos no chat",
    fileTaggingBody:
      "No chat do Cursor e do Windsurf, diga o nome de um arquivo (“index dot ts”) ou “tag” e o nome dele, e o Shrimp escreve uma menção com @. Funciona com arquivos abertos ou visíveis na barra lateral.",
    variables: "Reconhecimento de variáveis",
    variablesBody:
      "No Cursor, no VS Code e no Windsurf, diga os nomes naturalmente, “user ID” para <code>userId</code>: o Shrimp os associa ao código na tela. Diga “camel case”, “snake case”, “pascal case” ou “kebab case” antes das palavras para criar um novo nome.",
    step1: "Abra a Paleta de Comandos (Cmd+Shift+P).",
    step2: "Execute “Toggle Screen Reader Accessibility Mode”.",
    step3: "Confira se “Screen Reader Optimized” aparece na barra de status.",
  },
  contextAwareness: {
    title: "Percepção de contexto",
    body: "O Shrimp lê o texto ao redor do cursor e os nomes na tela, para escrever os nomes corretamente e adequar o estilo ao app. Quando desativado, nada da sua tela chega ao mecanismo de fala nem ao provedor de IA; maiúsculas e espaçamento continuam se adaptando ao texto ao redor do cursor, neste Mac.",
  },
  shortcutsSection: {
    title: "Atalhos",
    pushToTalk: "Pressionar para falar",
    change: "Alterar",
    note: "Cada ação aceita até quatro atalhos, incluindo botões do mouse. Pressionar Enter envia Enter para o app em que você está; segure Shift ou outro modificador junto para enviar Shift+Enter.",
  },
};

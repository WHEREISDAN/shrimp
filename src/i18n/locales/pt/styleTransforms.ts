import type { StyleTransformsCatalog } from "../en";

/** Brazilian Portuguese: the Style and Transforms pages. */
export const styleTransforms: StyleTransformsCatalog = {
  style: {
    title: "Estilo",
    cleanup: {
      title: "Limpeza automática",
      none: "Nenhuma",
      noneBody:
        "Cola a transcrição bruta, com vícios de linguagem, frases recomeçadas e erros de fala.",
      light: "Leve",
      lightBody: "Remove vícios de linguagem e corrige a gramática.",
      medium: "Média",
      mediumBody:
        "Pode reformular frases para deixá-las mais claras e concisas, mantendo o sentido.",
    },
    styles: {
      title: "Estilos",
      tabs: "Estilos",
      appliesTo: "<label>Aplica-se a </label>{{apps}}",
      group: "Estilo: {{category}}",
      next: "Próximo",
      footnote:
        "Os estilos valem para novos ditados. Assistentes de IA como ChatGPT e Claude não recebem formatação de estilo.",
    },
    // Each title is written in its own style: capitals and punctuation show the difference.
    options: {
      formal: { title: "Formal.", description: "Maiúsculas + pontuação." },
      casual: { title: "Casual", description: "Maiúsculas + menos pontuação." },
      veryCasual: { title: "bem casual", description: "Sem maiúsculas + menos pontuação." },
      excited: { title: "Animado!", description: "Mais exclamações." },
    },
    // Within a category the examples use the same words; only capitals and punctuation change.
    categories: {
      personal: {
        label: "Mensagens pessoais",
        apps: "Mensagens, WhatsApp, Discord, Signal, Telegram, Instagram",
        formal: "Oi, você está livre para almoçar amanhã? Pode ser ao meio-dia, se der.",
        casual: "Oi você está livre para almoçar amanhã? Pode ser ao meio-dia se der",
        veryCasual: "oi você está livre para almoçar amanhã? pode ser ao meio-dia se der",
      },
      work: {
        label: "Mensagens de trabalho",
        apps: "Slack, Microsoft Teams, LinkedIn",
        formal: "Obrigado pelo retorno. Os números estão bons. Vamos seguir com a primeira opção.",
        casual: "Obrigado pelo retorno, os números estão bons. Vamos seguir com a primeira opção",
        excited: "Obrigado pelo retorno! Os números estão bons! Vamos seguir com a primeira opção!",
      },
      email: {
        label: "E-mail",
        apps: "Mail, Outlook, Missive, Superhuman, Spark, Thunderbird, Mimestream",
        formal:
          "Oi, Dana, obrigado por enviar o contrato. Vou revisá-lo hoje e te dou um retorno até sexta.",
        casual:
          "Oi Dana obrigado por enviar o contrato, vou revisá-lo hoje e te dou um retorno até sexta",
        excited:
          "Oi, Dana! Obrigado por enviar o contrato! Vou revisá-lo hoje e te dou um retorno até sexta!",
      },
      other: {
        label: "Outros",
        apps: "Todo o resto, incluindo assistentes de IA e terminais.",
        formal: "O build terminou e todos os testes passaram, então vou fazer o merge agora.",
        casual: "O build terminou e todos os testes passaram então vou fazer o merge agora",
        excited: "O build terminou e todos os testes passaram então vou fazer o merge agora!",
      },
    },
  },
  transforms: {
    title: "Transformações",
    heading: "Reescreva qualquer texto com IA",
    toggle: "Transformações",
    toggleBody:
      "Selecione um texto em qualquer lugar e pressione o atalho de uma transformação, ou passe o cursor pela Flow Bar e clique na varinha: o resultado substitui o texto, e “Ver alterações” mostra o que mudou. Seu texto é enviado ao seu provedor de IA.",
    newTransform: "Nova transformação",
    afterDictation: "Depois do ditado",
    autoApply: "Aplicar uma transformação após cada ditado",
    off: "Nenhuma",
    autoApplyBody: "Se a transformação falhar, seu ditado é colado como estava.",
    seeChanges: "Ver alterações",
    reset: "Redefinir",
    polish: {
      title: "Refinar",
      body: "Reescreve o texto selecionado, o campo de texto ou o ditado feito no último minuto.",
      concise: "Deixar mais conciso",
      conciseBody: "Corta rodeios, repetições e excessos.",
      clarity: "Reformular para mais clareza",
      clarityBody: "Substitui frases vagas ou confusas.",
      reorder: "Reordenar para facilitar a leitura",
      reorderBody: "Coloca a ideia principal primeiro e agrupa os pontos relacionados.",
      structure: "Estruturar para facilitar a leitura",
      structureBody: "Parágrafos curtos e listas para itens enumerados.",
      tone: "Manter seu tom",
      toneBody: "Mantém a sua voz como está.",
      mainPoint: "Esclarecer a ideia principal",
      mainPointBody: "Diz claramente do que o texto trata ou o que ele pede.",
      impact: "Refinar a redação para causar mais impacto",
      impactBody: "Prefere verbos fortes e palavras específicas, sem exagerar.",
      instructions: "Instruções personalizadas",
      remove: "Remover",
      removeInstruction: "Remover {{instruction}}",
      newInstruction: "Nova instrução",
      placeholder: "Por exemplo: evite gírias",
      add: "Adicionar",
      tooLong_one: "Cada instrução deve ter menos de {{count, number}} palavra.",
      tooLong_many: "Cada instrução deve ter menos de {{count, number}} palavras.",
      tooLong_other: "Cada instrução deve ter menos de {{count, number}} palavras.",
      tooLong_zero: "Cada instrução deve ter menos de {{count, number}} palavras.",
      duplicate: "Já existe uma regra personalizada com este texto",
    },
    promptEngineer: {
      title: "Prompt Engineer",
      body: "Transforma ideias soltas em um prompt estruturado para um assistente de IA.",
      prompt: "Prompt",
    },
    custom: {
      fallbackName: "Transformação {{position, number}}",
      name: "Nome",
      prompt: "Prompt",
      placeholder: "Por exemplo: reescreva isto como uma resposta simpática a um cliente",
      incomplete: "Dê um nome e um prompt para usá-la.",
      samples: "Exemplos de escrita",
      samplesBody: "Até 5 exemplos de 50 a 500 palavras cujo estilo o resultado segue.",
      sample: "Exemplo {{position, number}}",
      words_one: "{{count, number}} palavra",
      words_many: "{{count, number}} palavras",
      words_other: "{{count, number}} palavras",
      words_zero: "{{count, number}} palavras",
      removeSample: "Remover exemplo",
      addSample: "Adicionar exemplo",
    },
  },
};

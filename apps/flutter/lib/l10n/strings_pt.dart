const Map<String, String> stringsPt = {
  // App
  'appTitle': 'Tesseras',

  // Memory types
  'memoryTypeMoment': 'Momento',
  'memoryTypeReflection': 'Reflexao',
  'memoryTypeDaily': 'Diario',
  'memoryTypeRelation': 'Relacao',
  'memoryTypeObject': 'Objeto',

  // Visibility
  'visibilityPrivate': 'Privado',
  'visibilityCircle': 'Circulo',
  'visibilityPublic': 'Publico',
  'visibilityPublicAfterDeath': 'Publico Apos Morte',
  'visibilitySealed': 'Lacrado',

  // Visibility badges
  'badgePrivate': 'Privado',
  'badgeCircle': 'Circulo',
  'badgePublic': 'Publico',
  'badgeAfterDeath': 'Apos Morte',
  'badgeSealed': 'Lacrado',

  // Network event types
  'eventPeerConnected': 'Par conectado',
  'eventPeerDisconnected': 'Par desconectado',
  'eventAttestationReceived': 'Atestacao recebida',
  'eventReplicationComplete': 'Replicacao completa',
  'eventRepairTriggered': 'Reparo iniciado',
  'eventBootstrapComplete': 'Bootstrap completo',

  // Welcome screen
  'welcomeTitle': 'Tesseras',
  'welcomeTagline': 'Preserve suas memorias atraves dos milenios.',
  'welcomeBody':
      'Uma rede ponto a ponto onde suas fotos, audios e historias '
          'sao preservados para sempre \u2014 sem empresa, sem nuvem, sem '
          'ponto unico de falha.',
  'welcomeGetStarted': 'Comecar',

  // Identity screen
  'identityHeadline': 'Crie sua identidade',
  'identityTapToChangeColor': 'Toque para mudar a cor',
  'identityNameLabel': 'Seu nome',
  'identityKeysNote':
      'Sua identidade e protegida com chaves criptograficas geradas automaticamente.',
  'identityOrImport': 'Ou importar uma identidade existente',
  'identityImportButton': 'Importar Identidade',
  'identityImportHint': 'Selecione um arquivo .tessera-identity',
  'identityImported': 'Identidade importada com sucesso',
  'identityAvatarChoose': 'Escolher avatar',
  'identityAvatarCamera': 'Tirar foto',
  'identityAvatarGallery': 'Escolher da galeria',
  'identityAvatarColor': 'Cor solida',
  'identityAvatarRemove': 'Remover foto',
  'identityBack': 'Voltar',
  'identityContinue': 'Continuar',

  // Ready screen
  'readyWelcome': 'Bem-vindo, {name}!',
  'readyNodePrefix': 'No: {nodeId}...',
  'readyCopyNodeId': 'Copiar ID do No',
  'readyKeysNote':
      'Suas chaves foram geradas. Mantenha seu dispositivo seguro \u2014 ele guarda sua identidade.',
  'readyOpenButton': 'Abrir Tesseras',

  // Sidebar
  'sidebarTimeline': 'Linha do Tempo',
  'sidebarTimelineTooltip': 'Linha do Tempo (Ctrl+1)',
  'sidebarNetwork': 'Rede',
  'sidebarNetworkTooltip': 'Rede (Ctrl+2)',
  'sidebarSettings': 'Configuracoes',
  'sidebarSettingsTooltip': 'Configuracoes (Ctrl+3)',
  'sidebarNewMemory': 'Nova Memoria',
  'sidebarNewMemoryTooltip': 'Nova Memoria (Ctrl+N)',

  // Copy button
  'copyDefault': 'Copiar',
  'copiedToClipboard': 'Copiado para a area de transferencia',

  // Timeline screen
  'timelineTitle': 'Linha do Tempo',
  'timelineSearchHint': 'Buscar memorias...',
  'timelineSortTooltip': 'Ordenar',
  'timelineSortNewest': 'Mais recentes',
  'timelineSortOldest': 'Mais antigas',
  'timelineSortByType': 'Por tipo',

  // Empty timeline
  'emptyTimelineHeading': 'Nenhuma memoria ainda',
  'emptyTimelineSubtitle': 'Crie sua primeira memoria',
  'emptyTimelineHint': 'Pressione Ctrl+N para comecar',

  // Memory detail dialog
  'memoryDetailTitle': 'Detalhe da Memoria',
  'memoryDetailAudioUnavailable':
      'Reproducao de audio nao disponivel no mockup',
  'memoryDetailContextLabel': 'Contexto',
  'memoryDetailTypeLabel': 'Tipo',
  'memoryDetailCreatedLabel': 'Criado',
  'memoryDetailLanguageLabel': 'Idioma',
  'memoryDetailMediaLabel': 'Midia',
  'memoryDetailOpensAfter': 'Abre apos: {date}',
  'memoryDetailPublicAfterDeath':
      'Publico apos {n} anos de inatividade',
  'memoryDetailTesseraLabel': 'Tessera: ',
  'memoryDetailExported': 'Exportado para ~/Downloads',
  'memoryDetailExport': 'Exportar',
  'memoryDetailVerified': 'Tessera verificada com sucesso',
  'memoryDetailVerify': 'Verificar',
  'memoryDetailClose': 'Fechar',

  // Create memory dialog
  'createMemoryTitle': 'Nova Memoria',
  'createMemoryDropZone':
      'Arraste arquivo ou pasta aqui, ou clique para procurar',
  'createMemoryBrowseFiles': 'Procurar arquivos',
  'createMemoryOpenFolder': 'Abrir pasta',
  'createMemorySupportedFormats':
      'Suportados: JPEG, PNG, WAV, WebM, TXT, ZIP, TAR.GZ',
  'createMemoryFilesFound': '{n} arquivos suportados encontrados',
  'createMemoryContextLabel': 'Contexto (opcional)',
  'createMemoryContextHint': 'Sobre o que e esta memoria?',
  'createMemoryTypeLabel': 'Tipo',
  'createMemoryVisibilityLabel': 'Visibilidade',
  'createMemoryOpenAfterDate': 'Abrir apos data',
  'createMemoryInactiveYears': 'Anos de inatividade',
  'createMemoryCircleNote':
      'Membros do circulo serao configuraveis em uma atualizacao futura.',
  'createMemoryLanguageLabel': 'Idioma',
  'createMemoryLangEnglish': 'Ingles',
  'createMemoryLangPortuguese': 'Portugues',
  'createMemoryLangSpanish': 'Espanhol',
  'createMemoryLangFrench': 'Frances',
  'createMemoryLangGerman': 'Alemao',
  'createMemoryLangJapanese': 'Japones',
  'createMemoryTagsLabel': 'Tags (separadas por virgula)',
  'createMemoryTagsHint': 'familia, ferias, 2026',
  'createMemoryLocationLabel': 'Localizacao (opcional)',
  'createMemoryLocationHint': 'Paraty, RJ',
  'createMemoryPeopleLabel': 'Pessoas (opcional, uma por linha)',
  'createMemoryPeopleHint': 'Maria - esposa\nLucas - filho',
  'createMemoryCancel': 'Cancelar',
  'createMemoryCreate': 'Criar Memoria',

  // Network screen
  'networkTitle': 'Rede',
  'networkRefreshTooltip': 'Atualizar',
  'networkRefreshed': 'Dados da rede atualizados',
  'networkNodeStatus': 'Status do No',
  'networkStatPeers': 'Pares',
  'networkStatDhtEntries': 'Entradas DHT',
  'networkStatBootstrapped': 'Bootstrap',
  'networkStatUptime': 'Tempo ativo',
  'networkStatNat': 'NAT',
  'networkStatYes': 'Sim',
  'networkStatNo': 'Nao',
  'networkReplication': 'Replicacao',
  'networkStatFragments': 'Fragmentos',
  'networkStatHealthy': 'Saudaveis',
  'networkStatRepairing': 'Reparando',
  'networkStatFactor': 'Fator',
  'networkStatStorage': 'Armazenamento',
  'networkConnectedPeers': 'Pares Conectados',
  'networkColNodeId': 'ID do No',
  'networkColAddress': 'Endereco',
  'networkColLastSeen': 'Visto por Ultimo',
  'networkRecentEvents': 'Eventos Recentes',

  // Settings screen
  'settingsTitle': 'Configuracoes',
  'settingsIdentity': 'Identidade',
  'settingsNodePrefix': 'No: {nodeId}...',
  'settingsCreatedPrefix': 'Criado: {date}',
  'settingsScanToConnect': 'Escaneie para conectar',
  'settingsCopyEd25519': 'Copiar Chave Publica Ed25519',
  'settingsAppearance': 'Aparencia',
  'settingsTheme': 'Tema',
  'settingsThemeLight': 'Claro',
  'settingsThemeDark': 'Escuro',
  'settingsThemeSystem': 'Sistema',
  'settingsLanguage': 'Idioma',
  'settingsLangEnglish': 'English',
  'settingsLangPortuguese': 'Portugues',
  'settingsLangSystem': 'Sistema',
  'settingsStorage': 'Armazenamento',
  'settingsMemories': 'Memorias: {n}',
  'settingsFragments': 'Fragmentos: {n}',
  'settingsDataDir': 'Diretorio: ~/.local/share/tesseras',
  'settingsNetwork': 'Rede',
  'settingsBootstrapNodes': 'Nos de bootstrap:',
  'settingsListenPort': 'Porta: 4433',
  'settingsMaxStorage': 'Armazenamento maximo: 10 GB',
  'settingsHeirs': 'Herdeiros',
  'settingsNoHeirs': 'Nenhum herdeiro configurado',
  'settingsComingSoon': 'Disponivel em uma atualizacao futura',
  'settingsConfigureHeirs': 'Configurar Herdeiros',
  'settingsAbout': 'Sobre',
  'settingsVersion': 'Tesseras v0.1.0',
  'settingsDescription': 'Rede P2P de preservacao de memorias',
  'settingsWebsite': 'tesseras.net',
  'settingsIrc': '#tesseras no Libera.Chat',
};

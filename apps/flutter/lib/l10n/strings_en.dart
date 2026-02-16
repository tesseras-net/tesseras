const Map<String, String> stringsEn = {
  // App
  'appTitle': 'Tesseras',

  // Memory types
  'memoryTypeMoment': 'Moment',
  'memoryTypeReflection': 'Reflection',
  'memoryTypeDaily': 'Daily',
  'memoryTypeRelation': 'Relation',
  'memoryTypeObject': 'Object',

  // Visibility
  'visibilityPrivate': 'Private',
  'visibilityCircle': 'Circle',
  'visibilityPublic': 'Public',
  'visibilityPublicAfterDeath': 'Public After Death',
  'visibilitySealed': 'Sealed',

  // Visibility badges
  'badgePrivate': 'Private',
  'badgeCircle': 'Circle',
  'badgePublic': 'Public',
  'badgeAfterDeath': 'After Death',
  'badgeSealed': 'Sealed',

  // Network event types
  'eventPeerConnected': 'Peer connected',
  'eventPeerDisconnected': 'Peer disconnected',
  'eventAttestationReceived': 'Attestation received',
  'eventReplicationComplete': 'Replication complete',
  'eventRepairTriggered': 'Repair triggered',
  'eventBootstrapComplete': 'Bootstrap complete',

  // Welcome screen
  'welcomeTitle': 'Tesseras',
  'welcomeTagline': 'Preserve your memories across millennia.',
  'welcomeBody':
      'A peer-to-peer network where your photos, audio, and stories '
          'are preserved forever \u2014 no company, no cloud, no single '
          'point of failure.',
  'welcomeGetStarted': 'Get Started',

  // Identity screen
  'identityHeadline': 'Create your identity',
  'identityTapToChangeColor': 'Tap to change color',
  'identityNameLabel': 'Your name',
  'identityKeysNote':
      'Your identity is secured with cryptographic keys generated automatically.',
  'identityOrImport': 'Or import an existing identity',
  'identityImportButton': 'Import Identity',
  'identityImportHint': 'Select a .tessera-identity file',
  'identityImported': 'Identity imported successfully',
  'identityAvatarChoose': 'Choose avatar',
  'identityAvatarCamera': 'Take photo',
  'identityAvatarGallery': 'Choose from gallery',
  'identityAvatarColor': 'Solid color',
  'identityAvatarRemove': 'Remove photo',
  'identityBack': 'Back',
  'identityContinue': 'Continue',

  // Ready screen
  'readyWelcome': 'Welcome, {name}!',
  'readyNodePrefix': 'Node: {nodeId}...',
  'readyCopyNodeId': 'Copy Node ID',
  'readyKeysNote':
      'Your keys have been generated. Keep your device safe \u2014 it holds your identity.',
  'readyOpenButton': 'Open Tesseras',

  // Sidebar
  'sidebarTimeline': 'Timeline',
  'sidebarTimelineTooltip': 'Timeline (Ctrl+1)',
  'sidebarNetwork': 'Network',
  'sidebarNetworkTooltip': 'Network (Ctrl+2)',
  'sidebarSettings': 'Settings',
  'sidebarSettingsTooltip': 'Settings (Ctrl+3)',
  'sidebarNewMemory': 'New Memory',
  'sidebarNewMemoryTooltip': 'New Memory (Ctrl+N)',

  // Copy button
  'copyDefault': 'Copy',
  'copiedToClipboard': 'Copied to clipboard',

  // Timeline screen
  'timelineTitle': 'Timeline',
  'timelineSearchHint': 'Search memories...',
  'timelineSortTooltip': 'Sort',
  'timelineSortNewest': 'Newest first',
  'timelineSortOldest': 'Oldest first',
  'timelineSortByType': 'By type',

  // Empty timeline
  'emptyTimelineHeading': 'No memories yet',
  'emptyTimelineSubtitle': 'Create your first memory',
  'emptyTimelineHint': 'Press Ctrl+N to get started',

  // Memory detail dialog
  'memoryDetailTitle': 'Memory Detail',
  'memoryDetailAudioUnavailable': 'Audio playback not available in mockup',
  'memoryDetailContextLabel': 'Context',
  'memoryDetailTypeLabel': 'Type',
  'memoryDetailCreatedLabel': 'Created',
  'memoryDetailLanguageLabel': 'Language',
  'memoryDetailMediaLabel': 'Media',
  'memoryDetailOpensAfter': 'Opens after: {date}',
  'memoryDetailPublicAfterDeath':
      'Public after {n} years of inactivity',
  'memoryDetailTesseraLabel': 'Tessera: ',
  'memoryDetailExported': 'Exported to ~/Downloads',
  'memoryDetailExport': 'Export',
  'memoryDetailVerified': 'Tessera verified successfully',
  'memoryDetailVerify': 'Verify',
  'memoryDetailClose': 'Close',

  // Create memory dialog
  'createMemoryTitle': 'New Memory',
  'createMemoryDropZone': 'Drop file or folder here, or click to browse',
  'createMemoryBrowseFiles': 'Browse files',
  'createMemoryOpenFolder': 'Open folder',
  'createMemorySupportedFormats':
      'Supported: JPEG, PNG, WAV, WebM, TXT, ZIP, TAR.GZ',
  'createMemoryFilesFound': '{n} supported files found',
  'createMemoryContextLabel': 'Context (optional)',
  'createMemoryContextHint': 'What is this memory about?',
  'createMemoryTypeLabel': 'Type',
  'createMemoryVisibilityLabel': 'Visibility',
  'createMemoryOpenAfterDate': 'Open after date',
  'createMemoryInactiveYears': 'Years of inactivity',
  'createMemoryCircleNote':
      'Circle members will be configurable in a future update.',
  'createMemoryLanguageLabel': 'Language',
  'createMemoryLangEnglish': 'English',
  'createMemoryLangPortuguese': 'Portuguese',
  'createMemoryLangSpanish': 'Spanish',
  'createMemoryLangFrench': 'French',
  'createMemoryLangGerman': 'German',
  'createMemoryLangJapanese': 'Japanese',
  'createMemoryTagsLabel': 'Tags (comma separated)',
  'createMemoryTagsHint': 'family, vacation, 2026',
  'createMemoryLocationLabel': 'Location (optional)',
  'createMemoryLocationHint': 'Paraty, RJ',
  'createMemoryPeopleLabel': 'People (optional, one per line)',
  'createMemoryPeopleHint': 'Maria - spouse\nLucas - son',
  'createMemoryCancel': 'Cancel',
  'createMemoryCreate': 'Create Memory',

  // Network screen
  'networkTitle': 'Network',
  'networkRefreshTooltip': 'Refresh',
  'networkRefreshed': 'Network data refreshed',
  'networkNodeStatus': 'Node Status',
  'networkStatPeers': 'Peers',
  'networkStatDhtEntries': 'DHT Entries',
  'networkStatBootstrapped': 'Bootstrapped',
  'networkStatUptime': 'Uptime',
  'networkStatNat': 'NAT',
  'networkStatYes': 'Yes',
  'networkStatNo': 'No',
  'networkReplication': 'Replication',
  'networkStatFragments': 'Fragments',
  'networkStatHealthy': 'Healthy',
  'networkStatRepairing': 'Repairing',
  'networkStatFactor': 'Factor',
  'networkStatStorage': 'Storage',
  'networkConnectedPeers': 'Connected Peers',
  'networkColNodeId': 'Node ID',
  'networkColAddress': 'Address',
  'networkColLastSeen': 'Last Seen',
  'networkRecentEvents': 'Recent Events',

  // Settings screen
  'settingsTitle': 'Settings',
  'settingsIdentity': 'Identity',
  'settingsNodePrefix': 'Node: {nodeId}...',
  'settingsCreatedPrefix': 'Created: {date}',
  'settingsScanToConnect': 'Scan to connect',
  'settingsCopyEd25519': 'Copy Ed25519 Public Key',
  'settingsAppearance': 'Appearance',
  'settingsTheme': 'Theme',
  'settingsThemeLight': 'Light',
  'settingsThemeDark': 'Dark',
  'settingsThemeSystem': 'System',
  'settingsLanguage': 'Language',
  'settingsLangEnglish': 'English',
  'settingsLangPortuguese': 'Portugues',
  'settingsLangSystem': 'System',
  'settingsStorage': 'Storage',
  'settingsMemories': 'Memories: {n}',
  'settingsFragments': 'Fragments: {n}',
  'settingsDataDir': 'Data dir: ~/.local/share/tesseras',
  'settingsNetwork': 'Network',
  'settingsBootstrapNodes': 'Bootstrap nodes:',
  'settingsListenPort': 'Listen port: 4433',
  'settingsMaxStorage': 'Max storage: 10 GB',
  'settingsHeirs': 'Heirs',
  'settingsNoHeirs': 'No heirs configured',
  'settingsComingSoon': 'Coming in a future update',
  'settingsConfigureHeirs': 'Configure Heirs',
  'settingsAbout': 'About',
  'settingsVersion': 'Tesseras v0.1.0',
  'settingsDescription': 'P2P memory preservation network',
  'settingsWebsite': 'tesseras.net',
  'settingsIrc': '#tesseras on Libera.Chat',
};

import 'package:flutter/widgets.dart';
import 'strings_en.dart';
import 'strings_pt.dart';

class AppLocalizations {
  final Map<String, String> _strings;

  AppLocalizations(this._strings);

  static AppLocalizations of(BuildContext context) =>
      Localizations.of<AppLocalizations>(context, AppLocalizations)!;

  String _get(String key) => _strings[key] ?? key;

  // App
  String get appTitle => _get('appTitle');

  // Memory types
  String get memoryTypeMoment => _get('memoryTypeMoment');
  String get memoryTypeReflection => _get('memoryTypeReflection');
  String get memoryTypeDaily => _get('memoryTypeDaily');
  String get memoryTypeRelation => _get('memoryTypeRelation');
  String get memoryTypeObject => _get('memoryTypeObject');

  // Visibility
  String get visibilityPrivate => _get('visibilityPrivate');
  String get visibilityCircle => _get('visibilityCircle');
  String get visibilityPublic => _get('visibilityPublic');
  String get visibilityPublicAfterDeath => _get('visibilityPublicAfterDeath');
  String get visibilitySealed => _get('visibilitySealed');

  // Visibility badges
  String get badgePrivate => _get('badgePrivate');
  String get badgeCircle => _get('badgeCircle');
  String get badgePublic => _get('badgePublic');
  String get badgeAfterDeath => _get('badgeAfterDeath');
  String get badgeSealed => _get('badgeSealed');

  // Network event types
  String get eventPeerConnected => _get('eventPeerConnected');
  String get eventPeerDisconnected => _get('eventPeerDisconnected');
  String get eventAttestationReceived => _get('eventAttestationReceived');
  String get eventReplicationComplete => _get('eventReplicationComplete');
  String get eventRepairTriggered => _get('eventRepairTriggered');
  String get eventBootstrapComplete => _get('eventBootstrapComplete');

  // Welcome screen
  String get welcomeTitle => _get('welcomeTitle');
  String get welcomeTagline => _get('welcomeTagline');
  String get welcomeBody => _get('welcomeBody');
  String get welcomeGetStarted => _get('welcomeGetStarted');

  // Identity screen
  String get identityHeadline => _get('identityHeadline');
  String get identityTapToChangeColor => _get('identityTapToChangeColor');
  String get identityNameLabel => _get('identityNameLabel');
  String get identityKeysNote => _get('identityKeysNote');
  String get identityOrImport => _get('identityOrImport');
  String get identityImportButton => _get('identityImportButton');
  String get identityImportHint => _get('identityImportHint');
  String get identityImported => _get('identityImported');
  String get identityAvatarChoose => _get('identityAvatarChoose');
  String get identityAvatarCamera => _get('identityAvatarCamera');
  String get identityAvatarGallery => _get('identityAvatarGallery');
  String get identityAvatarColor => _get('identityAvatarColor');
  String get identityAvatarRemove => _get('identityAvatarRemove');
  String get identityBack => _get('identityBack');
  String get identityContinue => _get('identityContinue');

  // Ready screen
  String readyWelcome(String name) =>
      _get('readyWelcome').replaceAll('{name}', name);
  String readyNodePrefix(String nodeId) =>
      _get('readyNodePrefix').replaceAll('{nodeId}', nodeId);
  String get readyCopyNodeId => _get('readyCopyNodeId');
  String get readyKeysNote => _get('readyKeysNote');
  String get readyOpenButton => _get('readyOpenButton');

  // Sidebar
  String get sidebarTimeline => _get('sidebarTimeline');
  String get sidebarTimelineTooltip => _get('sidebarTimelineTooltip');
  String get sidebarNetwork => _get('sidebarNetwork');
  String get sidebarNetworkTooltip => _get('sidebarNetworkTooltip');
  String get sidebarSettings => _get('sidebarSettings');
  String get sidebarSettingsTooltip => _get('sidebarSettingsTooltip');
  String get sidebarNewMemory => _get('sidebarNewMemory');
  String get sidebarNewMemoryTooltip => _get('sidebarNewMemoryTooltip');

  // Copy button
  String get copyDefault => _get('copyDefault');
  String get copiedToClipboard => _get('copiedToClipboard');

  // Timeline screen
  String get timelineTitle => _get('timelineTitle');
  String get timelineSearchHint => _get('timelineSearchHint');
  String get timelineSortTooltip => _get('timelineSortTooltip');
  String get timelineSortNewest => _get('timelineSortNewest');
  String get timelineSortOldest => _get('timelineSortOldest');
  String get timelineSortByType => _get('timelineSortByType');

  // Empty timeline
  String get emptyTimelineHeading => _get('emptyTimelineHeading');
  String get emptyTimelineSubtitle => _get('emptyTimelineSubtitle');
  String get emptyTimelineHint => _get('emptyTimelineHint');

  // Memory detail dialog
  String get memoryDetailTitle => _get('memoryDetailTitle');
  String get memoryDetailAudioUnavailable =>
      _get('memoryDetailAudioUnavailable');
  String get memoryDetailContextLabel => _get('memoryDetailContextLabel');
  String get memoryDetailTypeLabel => _get('memoryDetailTypeLabel');
  String get memoryDetailCreatedLabel => _get('memoryDetailCreatedLabel');
  String get memoryDetailLanguageLabel => _get('memoryDetailLanguageLabel');
  String get memoryDetailMediaLabel => _get('memoryDetailMediaLabel');
  String memoryDetailOpensAfter(String date) =>
      _get('memoryDetailOpensAfter').replaceAll('{date}', date);
  String memoryDetailPublicAfterDeath(int n) =>
      _get('memoryDetailPublicAfterDeath').replaceAll('{n}', n.toString());
  String get memoryDetailTesseraLabel => _get('memoryDetailTesseraLabel');
  String get memoryDetailExported => _get('memoryDetailExported');
  String get memoryDetailExport => _get('memoryDetailExport');
  String get memoryDetailVerified => _get('memoryDetailVerified');
  String get memoryDetailVerify => _get('memoryDetailVerify');
  String get memoryDetailClose => _get('memoryDetailClose');

  // Create memory dialog
  String get createMemoryTitle => _get('createMemoryTitle');
  String get createMemoryDropZone => _get('createMemoryDropZone');
  String get createMemoryBrowseFiles => _get('createMemoryBrowseFiles');
  String get createMemoryOpenFolder => _get('createMemoryOpenFolder');
  String get createMemorySupportedFormats =>
      _get('createMemorySupportedFormats');
  String createMemoryFilesFound(int n) =>
      _get('createMemoryFilesFound').replaceAll('{n}', n.toString());
  String get createMemoryContextLabel => _get('createMemoryContextLabel');
  String get createMemoryContextHint => _get('createMemoryContextHint');
  String get createMemoryTypeLabel => _get('createMemoryTypeLabel');
  String get createMemoryVisibilityLabel =>
      _get('createMemoryVisibilityLabel');
  String get createMemoryOpenAfterDate => _get('createMemoryOpenAfterDate');
  String get createMemoryInactiveYears => _get('createMemoryInactiveYears');
  String get createMemoryCircleNote => _get('createMemoryCircleNote');
  String get createMemoryLanguageLabel => _get('createMemoryLanguageLabel');
  String get createMemoryLangEnglish => _get('createMemoryLangEnglish');
  String get createMemoryLangPortuguese => _get('createMemoryLangPortuguese');
  String get createMemoryLangSpanish => _get('createMemoryLangSpanish');
  String get createMemoryLangFrench => _get('createMemoryLangFrench');
  String get createMemoryLangGerman => _get('createMemoryLangGerman');
  String get createMemoryLangJapanese => _get('createMemoryLangJapanese');
  String get createMemoryTagsLabel => _get('createMemoryTagsLabel');
  String get createMemoryTagsHint => _get('createMemoryTagsHint');
  String get createMemoryLocationLabel => _get('createMemoryLocationLabel');
  String get createMemoryLocationHint => _get('createMemoryLocationHint');
  String get createMemoryPeopleLabel => _get('createMemoryPeopleLabel');
  String get createMemoryPeopleHint => _get('createMemoryPeopleHint');
  String get createMemoryCancel => _get('createMemoryCancel');
  String get createMemoryCreate => _get('createMemoryCreate');

  // Network screen
  String get networkTitle => _get('networkTitle');
  String get networkRefreshTooltip => _get('networkRefreshTooltip');
  String get networkRefreshed => _get('networkRefreshed');
  String get networkNodeStatus => _get('networkNodeStatus');
  String get networkStatPeers => _get('networkStatPeers');
  String get networkStatDhtEntries => _get('networkStatDhtEntries');
  String get networkStatBootstrapped => _get('networkStatBootstrapped');
  String get networkStatUptime => _get('networkStatUptime');
  String get networkStatNat => _get('networkStatNat');
  String get networkStatYes => _get('networkStatYes');
  String get networkStatNo => _get('networkStatNo');
  String get networkReplication => _get('networkReplication');
  String get networkStatFragments => _get('networkStatFragments');
  String get networkStatHealthy => _get('networkStatHealthy');
  String get networkStatRepairing => _get('networkStatRepairing');
  String get networkStatFactor => _get('networkStatFactor');
  String get networkStatStorage => _get('networkStatStorage');
  String get networkConnectedPeers => _get('networkConnectedPeers');
  String get networkColNodeId => _get('networkColNodeId');
  String get networkColAddress => _get('networkColAddress');
  String get networkColLastSeen => _get('networkColLastSeen');
  String get networkRecentEvents => _get('networkRecentEvents');

  // Settings screen
  String get settingsTitle => _get('settingsTitle');
  String get settingsIdentity => _get('settingsIdentity');
  String settingsNodePrefix(String nodeId) =>
      _get('settingsNodePrefix').replaceAll('{nodeId}', nodeId);
  String settingsCreatedPrefix(String date) =>
      _get('settingsCreatedPrefix').replaceAll('{date}', date);
  String get settingsScanToConnect => _get('settingsScanToConnect');
  String get settingsCopyEd25519 => _get('settingsCopyEd25519');
  String get settingsAppearance => _get('settingsAppearance');
  String get settingsTheme => _get('settingsTheme');
  String get settingsThemeLight => _get('settingsThemeLight');
  String get settingsThemeDark => _get('settingsThemeDark');
  String get settingsThemeSystem => _get('settingsThemeSystem');
  String get settingsLanguage => _get('settingsLanguage');
  String get settingsLangEnglish => _get('settingsLangEnglish');
  String get settingsLangPortuguese => _get('settingsLangPortuguese');
  String get settingsLangSystem => _get('settingsLangSystem');
  String get settingsStorage => _get('settingsStorage');
  String settingsMemories(int n) =>
      _get('settingsMemories').replaceAll('{n}', n.toString());
  String settingsFragments(int n) =>
      _get('settingsFragments').replaceAll('{n}', n.toString());
  String get settingsDataDir => _get('settingsDataDir');
  String get settingsNetwork => _get('settingsNetwork');
  String get settingsBootstrapNodes => _get('settingsBootstrapNodes');
  String get settingsListenPort => _get('settingsListenPort');
  String get settingsMaxStorage => _get('settingsMaxStorage');
  String get settingsHeirs => _get('settingsHeirs');
  String get settingsNoHeirs => _get('settingsNoHeirs');
  String get settingsComingSoon => _get('settingsComingSoon');
  String get settingsConfigureHeirs => _get('settingsConfigureHeirs');
  String get settingsAbout => _get('settingsAbout');
  String get settingsVersion => _get('settingsVersion');
  String get settingsDescription => _get('settingsDescription');
  String get settingsWebsite => _get('settingsWebsite');
  String get settingsIrc => _get('settingsIrc');
}

class AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const AppLocalizationsDelegate();

  static const supportedLocales = [
    Locale('en'),
    Locale('pt'),
  ];

  @override
  bool isSupported(Locale locale) =>
      ['en', 'pt'].contains(locale.languageCode);

  @override
  Future<AppLocalizations> load(Locale locale) async {
    final strings = switch (locale.languageCode) {
      'pt' => stringsPt,
      _ => stringsEn,
    };
    return AppLocalizations(strings);
  }

  @override
  bool shouldReload(AppLocalizationsDelegate old) => false;
}

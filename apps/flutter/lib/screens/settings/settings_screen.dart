import 'dart:io';

import 'package:flutter/material.dart' hide ThemeMode;
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart' show ThemeMode;
import '../../l10n/app_localizations.dart';
import '../../providers/locale_provider.dart';
import '../../providers/mock_data.dart';
import '../../providers/mock_identity_provider.dart';
import '../../providers/theme_provider.dart';
import '../../widgets/copy_button.dart';
import '../../widgets/node_qr_code.dart';
import '../../widgets/storage_bar.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final identity = ref.watch(mockIdentityProvider) ?? mockIdentity;
    final themeMode = ref.watch(themeProvider);
    final localeOverride = ref.watch(localeProvider);
    final colorScheme = Theme.of(context).colorScheme;
    final l = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.settingsTitle,
              style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 16),
          Expanded(
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 700),
                child: ListView(
                  children: [
                    // 1. Identity
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsIdentity,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                CircleAvatar(
                                  radius: 28,
                                  backgroundColor: identity.avatarImagePath == null
                                      ? identity.avatarColor
                                      : null,
                                  backgroundImage: identity.avatarImagePath != null
                                      ? FileImage(File(identity.avatarImagePath!))
                                      : null,
                                  child: identity.avatarImagePath == null
                                      ? Text(
                                          identity.name.isNotEmpty
                                              ? identity.name[0].toUpperCase()
                                              : '?',
                                          style: const TextStyle(
                                              color: Colors.white,
                                              fontSize: 24,
                                              fontWeight: FontWeight.bold),
                                        )
                                      : null,
                                ),
                                const SizedBox(width: 16),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Text(identity.name,
                                          style: Theme.of(context)
                                              .textTheme
                                              .titleLarge),
                                      Text(
                                        l.settingsNodePrefix(
                                            identity.nodeIdHex
                                                .substring(0, 16)),
                                        style: Theme.of(context)
                                            .textTheme
                                            .bodySmall
                                            ?.copyWith(
                                                fontFamily: 'monospace'),
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            _KeyRow(
                              label: 'Ed25519',
                              value: identity.ed25519PublicKeyHex,
                            ),
                            _KeyRow(
                              label: 'ML-DSA',
                              value: identity.mldsaPublicKeyHex,
                            ),
                            const SizedBox(height: 8),
                            Text(
                              l.settingsCreatedPrefix(
                                  _formatDate(identity.createdAt)),
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                            const SizedBox(height: 8),
                            const SizedBox(height: 12),
                            // QR-like pattern for easy peer connection
                            Center(
                              child: Column(
                                children: [
                                  NodeQrCode(
                                      hexData: identity.nodeIdHex),
                                  const SizedBox(height: 8),
                                  Text(
                                    l.settingsScanToConnect,
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall
                                        ?.copyWith(
                                            color: colorScheme
                                                .onSurfaceVariant),
                                  ),
                                ],
                              ),
                            ),
                            const SizedBox(height: 8),
                            Row(
                              mainAxisAlignment: MainAxisAlignment.end,
                              children: [
                                CopyButton(
                                  text: identity.ed25519PublicKeyHex,
                                  tooltip: l.settingsCopyEd25519,
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 2. Appearance
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsAppearance,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text(l.settingsTheme),
                                const SizedBox(width: 16),
                                SegmentedButton<ThemeMode>(
                                  segments: [
                                    ButtonSegment(
                                        value: ThemeMode.light,
                                        label: Text(l.settingsThemeLight),
                                        icon: const Icon(Icons.light_mode)),
                                    ButtonSegment(
                                        value: ThemeMode.dark,
                                        label: Text(l.settingsThemeDark),
                                        icon: const Icon(Icons.dark_mode)),
                                    ButtonSegment(
                                        value: ThemeMode.system,
                                        label: Text(l.settingsThemeSystem),
                                        icon: const Icon(
                                            Icons.settings_suggest)),
                                  ],
                                  selected: {themeMode},
                                  onSelectionChanged: (selected) {
                                    ref
                                        .read(themeProvider.notifier)
                                        .setTheme(selected.first);
                                  },
                                ),
                              ],
                            ),
                            const SizedBox(height: 16),
                            Row(
                              children: [
                                Text(l.settingsLanguage),
                                const SizedBox(width: 16),
                                SegmentedButton<Locale?>(
                                  segments: [
                                    ButtonSegment(
                                        value: const Locale('en'),
                                        label:
                                            Text(l.settingsLangEnglish)),
                                    ButtonSegment(
                                        value: const Locale('pt'),
                                        label:
                                            Text(l.settingsLangPortuguese)),
                                    ButtonSegment(
                                        value: null,
                                        label:
                                            Text(l.settingsLangSystem),
                                        icon: const Icon(
                                            Icons.settings_suggest)),
                                  ],
                                  selected: {localeOverride},
                                  onSelectionChanged: (selected) {
                                    ref
                                        .read(localeProvider.notifier)
                                        .setLocale(selected.first);
                                  },
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 3. Storage
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsStorage,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            const StorageBar(
                                usedMB: 142, totalMB: 10 * 1024),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text(l.settingsMemories(47),
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall),
                                const SizedBox(width: 24),
                                Text(l.settingsFragments(847),
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall),
                              ],
                            ),
                            const SizedBox(height: 4),
                            Text(
                              l.settingsDataDir,
                              style: Theme.of(context)
                                  .textTheme
                                  .bodySmall
                                  ?.copyWith(fontFamily: 'monospace'),
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 4. Network
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsNetwork,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text(l.settingsBootstrapNodes,
                                style:
                                    Theme.of(context).textTheme.bodySmall),
                            const SizedBox(height: 4),
                            Text('  boot1.tesseras.net:4433',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(fontFamily: 'monospace')),
                            Text('  boot2.tesseras.net:4433',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(fontFamily: 'monospace')),
                            const SizedBox(height: 8),
                            Text(l.settingsListenPort,
                                style:
                                    Theme.of(context).textTheme.bodySmall),
                            Text(l.settingsMaxStorage,
                                style:
                                    Theme.of(context).textTheme.bodySmall),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 5. Heirs
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsHeirs,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text(l.settingsNoHeirs,
                                style: Theme.of(context)
                                    .textTheme
                                    .bodyMedium
                                    ?.copyWith(
                                        color:
                                            colorScheme.onSurfaceVariant)),
                            const SizedBox(height: 8),
                            OutlinedButton(
                              onPressed: () {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  SnackBar(
                                      content: Text(l.settingsComingSoon)),
                                );
                              },
                              child: Text(l.settingsConfigureHeirs),
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 6. About
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsAbout,
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text(l.settingsVersion,
                                style:
                                    Theme.of(context).textTheme.bodyLarge),
                            const SizedBox(height: 4),
                            Text(l.settingsDescription,
                                style:
                                    Theme.of(context).textTheme.bodyMedium),
                            const SizedBox(height: 8),
                            Text(l.settingsWebsite,
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: colorScheme.primary)),
                            Text(l.settingsIrc,
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: colorScheme.primary)),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 16),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  String _formatDate(String isoDate) {
    final dt = DateTime.tryParse(isoDate);
    if (dt == null) return isoDate;
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }
}

class _KeyRow extends StatelessWidget {
  final String label;
  final String value;

  const _KeyRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          SizedBox(
            width: 60,
            child: Text(label,
                style: Theme.of(context)
                    .textTheme
                    .bodySmall
                    ?.copyWith(fontWeight: FontWeight.bold)),
          ),
          Expanded(
            child: Text(
              '${value.substring(0, 24)}...',
              style: Theme.of(context)
                  .textTheme
                  .bodySmall
                  ?.copyWith(fontFamily: 'monospace'),
            ),
          ),
        ],
      ),
    );
  }
}

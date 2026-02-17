import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../providers/identity_provider.dart';
import '../../providers/locale_provider.dart';
import '../../providers/network_provider.dart';
import '../../providers/storage_provider.dart';
import '../../providers/theme_provider.dart';
import '../../widgets/copy_button.dart';
import '../../widgets/node_qr_code.dart';
import '../../widgets/storage_bar.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final identityAsync = ref.watch(identityProvider);
    final storageAsync = ref.watch(storageProvider);
    final networkAsync = ref.watch(networkProvider);
    final themeMode = ref.watch(themeProvider);
    final localeOverride = ref.watch(localeProvider);
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);

    final identity = identityAsync.value;
    final storage = storageAsync.value;
    final network = networkAsync.value;

    if (identity == null) {
      return const Center(child: CircularProgressIndicator());
    }

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(l.settingsTitle).h3.semiBold,
          const SizedBox(height: 16),
          Expanded(
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 700),
                child: ListView(
                  children: [
                    // 1. Identity
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsIdentity).semiBold,
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Container(
                                  width: 56,
                                  height: 56,
                                  decoration: BoxDecoration(
                                    shape: BoxShape.circle,
                                    color: identity.avatarPath == null
                                        ? const Color(0xFF3F51B5)
                                        : null,
                                    image: identity.avatarPath != null
                                        ? DecorationImage(
                                            image: FileImage(
                                                File(identity.avatarPath!)),
                                            fit: BoxFit.cover,
                                          )
                                        : null,
                                  ),
                                  child: identity.avatarPath == null
                                      ? Center(
                                          child: Text(
                                            identity.name.isNotEmpty
                                                ? identity.name[0].toUpperCase()
                                                : '?',
                                            style: const TextStyle(
                                                color: Color(0xFFFFFFFF),
                                                fontSize: 24,
                                                fontWeight: FontWeight.bold),
                                          ),
                                        )
                                      : null,
                                ),
                                const SizedBox(width: 16),
                                Expanded(
                                  child: Column(
                                    crossAxisAlignment:
                                        CrossAxisAlignment.start,
                                    children: [
                                      Text(identity.name).large.semiBold,
                                      Text(l.settingsNodePrefix(
                                              identity.nodeIdHex
                                                  .substring(0, 16)))
                                          .small
                                          .mono
                                          .muted,
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 12),
                            _KeyRow(
                              label: 'Ed25519',
                              value: identity.publicKeyHex,
                            ),
                            const SizedBox(height: 8),
                            Text(l.settingsCreatedPrefix(
                                    _formatDate(identity.createdAt)))
                                .small
                                .muted,
                            const SizedBox(height: 12),
                            // QR-like pattern
                            Center(
                              child: Column(
                                children: [
                                  NodeQrCode(
                                      hexData: identity.nodeIdHex),
                                  const SizedBox(height: 8),
                                  Text(l.settingsScanToConnect).small.muted,
                                ],
                              ),
                            ),
                            const SizedBox(height: 8),
                            Row(
                              mainAxisAlignment: MainAxisAlignment.end,
                              children: [
                                CopyButton(
                                  text: identity.publicKeyHex,
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
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsAppearance).semiBold,
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text(l.settingsTheme),
                                const SizedBox(width: 16),
                                Expanded(
                                  child: Row(
                                    children: [
                                      _ThemeButton(
                                        label: l.settingsThemeLight,
                                        icon: Icons.light_mode,
                                        selected: themeMode == ThemeMode.light,
                                        onTap: () => ref
                                            .read(themeProvider.notifier)
                                            .setTheme(ThemeMode.light),
                                      ),
                                      const SizedBox(width: 4),
                                      _ThemeButton(
                                        label: l.settingsThemeDark,
                                        icon: Icons.dark_mode,
                                        selected: themeMode == ThemeMode.dark,
                                        onTap: () => ref
                                            .read(themeProvider.notifier)
                                            .setTheme(ThemeMode.dark),
                                      ),
                                      const SizedBox(width: 4),
                                      _ThemeButton(
                                        label: l.settingsThemeSystem,
                                        icon: Icons.settings_suggest,
                                        selected: themeMode == ThemeMode.system,
                                        onTap: () => ref
                                            .read(themeProvider.notifier)
                                            .setTheme(ThemeMode.system),
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 16),
                            Row(
                              children: [
                                Text(l.settingsLanguage),
                                const SizedBox(width: 16),
                                Expanded(
                                  child: Row(
                                    children: [
                                      _ThemeButton(
                                        label: l.settingsLangEnglish,
                                        selected: localeOverride ==
                                            const Locale('en'),
                                        onTap: () => ref
                                            .read(localeProvider.notifier)
                                            .setLocale(const Locale('en')),
                                      ),
                                      const SizedBox(width: 4),
                                      _ThemeButton(
                                        label: l.settingsLangPortuguese,
                                        selected: localeOverride ==
                                            const Locale('pt'),
                                        onTap: () => ref
                                            .read(localeProvider.notifier)
                                            .setLocale(const Locale('pt')),
                                      ),
                                      const SizedBox(width: 4),
                                      _ThemeButton(
                                        label: l.settingsLangSystem,
                                        icon: Icons.settings_suggest,
                                        selected: localeOverride == null,
                                        onTap: () => ref
                                            .read(localeProvider.notifier)
                                            .setLocale(null),
                                      ),
                                    ],
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 3. Storage
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsStorage).semiBold,
                            const SizedBox(height: 12),
                            StorageBar(
                                usedMB: storage != null
                                    ? (storage.totalBytes.toInt() /
                                            (1024 * 1024))
                                        .round()
                                    : 0,
                                totalMB: 10 * 1024),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text(l.settingsMemories(
                                        storage?.tesseraCount ?? 0))
                                    .small,
                                const SizedBox(width: 24),
                                Text(l.settingsFragments(
                                        network?.replication.totalFragments ??
                                            0))
                                    .small,
                              ],
                            ),
                            const SizedBox(height: 4),
                            Text(l.settingsDataDir).small.mono.muted,
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 4. Network
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsNetwork).semiBold,
                            const SizedBox(height: 12),
                            Text(l.settingsBootstrapNodes).small,
                            const SizedBox(height: 4),
                            const Text('  boot1.tesseras.net:4433')
                                .small
                                .mono
                                .muted,
                            const Text('  boot2.tesseras.net:4433')
                                .small
                                .mono
                                .muted,
                            const SizedBox(height: 8),
                            Text(l.settingsListenPort).small,
                            Text(l.settingsMaxStorage).small,
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 5. Heirs
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsHeirs).semiBold,
                            const SizedBox(height: 12),
                            Text(l.settingsNoHeirs).muted,
                            const SizedBox(height: 8),
                            Button.outline(
                              onPressed: () {
                                showToast(
                                  context: context,
                                  builder: (context, overlay) => SurfaceCard(
                                    child: Basic(
                                      title: Text(l.settingsComingSoon),
                                      trailing: Button(
                                        style: const ButtonStyle.ghost(
                                            density: ButtonDensity.icon),
                                        onPressed: overlay.close,
                                        child:
                                            const Icon(Icons.close, size: 16),
                                      ),
                                    ),
                                  ),
                                  showDuration: const Duration(seconds: 2),
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
                    SurfaceCard(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(l.settingsAbout).semiBold,
                            const SizedBox(height: 12),
                            Text(l.settingsVersion).large,
                            const SizedBox(height: 4),
                            Text(l.settingsDescription),
                            const SizedBox(height: 8),
                            Text(l.settingsWebsite).small.muted,
                            Text(l.settingsIrc).small.muted,
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

class _ThemeButton extends StatelessWidget {
  final String label;
  final IconData? icon;
  final bool selected;
  final VoidCallback onTap;

  const _ThemeButton({
    required this.label,
    this.icon,
    required this.selected,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Button(
      style: ButtonStyle(
        variance: selected ? ButtonVariance.secondary : ButtonVariance.outline,
      ),
      onPressed: onTap,
      leading: icon != null ? Icon(icon!, size: 16) : null,
      child: Text(label),
    );
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
            child: Text(label).small.semiBold,
          ),
          Expanded(
            child: Text('${value.substring(0, 24)}...').small.mono.muted,
          ),
        ],
      ),
    );
  }
}

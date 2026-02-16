import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
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
    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text('Settings', style: Theme.of(context).textTheme.headlineSmall),
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
                            Text('Identity',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                CircleAvatar(
                                  radius: 28,
                                  backgroundColor: identity.avatarColor,
                                  child: Text(
                                    identity.name.isNotEmpty
                                        ? identity.name[0].toUpperCase()
                                        : '?',
                                    style: const TextStyle(
                                        color: Colors.white,
                                        fontSize: 24,
                                        fontWeight: FontWeight.bold),
                                  ),
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
                                        'Node: ${identity.nodeIdHex.substring(0, 16)}...',
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
                              'Created: ${_formatDate(identity.createdAt)}',
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
                                    'Scan to connect',
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
                                  tooltip: 'Copy Ed25519 Public Key',
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
                            Text('Appearance',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                const Text('Theme'),
                                const SizedBox(width: 16),
                                SegmentedButton<ThemeMode>(
                                  segments: const [
                                    ButtonSegment(
                                        value: ThemeMode.light,
                                        label: Text('Light'),
                                        icon: Icon(Icons.light_mode)),
                                    ButtonSegment(
                                        value: ThemeMode.dark,
                                        label: Text('Dark'),
                                        icon: Icon(Icons.dark_mode)),
                                    ButtonSegment(
                                        value: ThemeMode.system,
                                        label: Text('System'),
                                        icon: Icon(Icons.settings_suggest)),
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
                            Text('Storage',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            const StorageBar(
                                usedMB: 142, totalMB: 10 * 1024),
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text('Memories: 47',
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall),
                                const SizedBox(width: 24),
                                Text('Fragments: 847',
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall),
                              ],
                            ),
                            const SizedBox(height: 4),
                            Text(
                              'Data dir: ~/.local/share/tesseras',
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
                            Text('Network',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text('Bootstrap nodes:',
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
                            Text('Listen port: 4433',
                                style:
                                    Theme.of(context).textTheme.bodySmall),
                            Text('Max storage: 10 GB',
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
                            Text('Heirs',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text('No heirs configured',
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
                                  const SnackBar(
                                      content: Text(
                                          'Coming in a future update')),
                                );
                              },
                              child: const Text('Configure Heirs'),
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
                            Text('About',
                                style:
                                    Theme.of(context).textTheme.titleMedium),
                            const SizedBox(height: 12),
                            Text('Tesseras v0.1.0',
                                style:
                                    Theme.of(context).textTheme.bodyLarge),
                            const SizedBox(height: 4),
                            Text(
                                'P2P memory preservation network',
                                style:
                                    Theme.of(context).textTheme.bodyMedium),
                            const SizedBox(height: 8),
                            Text('tesseras.net',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: colorScheme.primary)),
                            Text('#tesseras on Libera.Chat',
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

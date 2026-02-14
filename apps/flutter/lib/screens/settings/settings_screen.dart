import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/identity_provider.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final identity = ref.watch(identityProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Settings')),
      body: ListView(
        children: [
          // Identity section
          identity.when(
            loading: () => const ListTile(
              title: Text('Loading identity...'),
            ),
            error: (e, _) => ListTile(
              title: Text('Error: $e'),
            ),
            data: (id) => id != null
                ? Column(
                    children: [
                      ListTile(
                        leading: const CircleAvatar(
                          child: Icon(Icons.person),
                        ),
                        title: Text(id.name),
                        subtitle: Text('Node: ${id.nodeIdHex.substring(0, 16)}...'),
                      ),
                      ListTile(
                        leading: const Icon(Icons.key),
                        title: const Text('Public Key'),
                        subtitle: Text(
                          '${id.publicKeyHex.substring(0, 24)}...',
                          style: const TextStyle(fontFamily: 'monospace'),
                        ),
                      ),
                      ListTile(
                        leading: const Icon(Icons.calendar_today),
                        title: const Text('Created'),
                        subtitle: Text(id.createdAt),
                      ),
                    ],
                  )
                : const ListTile(
                    title: Text('No identity'),
                  ),
          ),
          const Divider(),

          // About section
          ListTile(
            leading: const Icon(Icons.info_outline),
            title: const Text('About Tesseras'),
            subtitle: const Text('P2P memory preservation network'),
            onTap: () {
              showAboutDialog(
                context: context,
                applicationName: 'Tesseras',
                applicationVersion: '0.1.0',
                children: [
                  const Text(
                    'Preserve your memories across millennia. '
                    'No cloud. No company. Just you and the network.',
                  ),
                ],
              );
            },
          ),
        ],
      ),
    );
  }
}

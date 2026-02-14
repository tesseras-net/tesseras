import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../src/rust/api/simple.dart' as rust;
import '../../src/rust/types.dart';
import '../../providers/node_provider.dart';

final networkStatsProvider = FutureProvider<NetworkStats>((ref) {
  final nodeRunning = ref.watch(nodeProvider);
  if (nodeRunning is! AsyncData || nodeRunning.value != true) {
    return NetworkStats(
      peerCount: 0,
      dhtSize: 0,
      isBootstrapped: false,
      uptimeSecs: BigInt.zero,
    );
  }
  return rust.getNetworkStats();
});

final replicationStatusProvider = FutureProvider<ReplicationStatus>((ref) {
  final nodeRunning = ref.watch(nodeProvider);
  if (nodeRunning is! AsyncData || nodeRunning.value != true) {
    return ReplicationStatus(
      totalFragments: 0,
      healthyFragments: 0,
      repairingFragments: 0,
      replicationFactor: 0,
    );
  }
  return rust.getReplicationStatus();
});

class NetworkScreen extends ConsumerWidget {
  const NetworkScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final stats = ref.watch(networkStatsProvider);
    final replication = ref.watch(replicationStatusProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Network'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: () {
              ref.invalidate(networkStatsProvider);
              ref.invalidate(replicationStatusProvider);
            },
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Node status card
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Node Status',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 12),
                  stats.when(
                    loading: () => const CircularProgressIndicator(),
                    error: (e, _) => Text('Error: $e'),
                    data: (s) => Column(
                      children: [
                        _StatRow('Peers', '${s.peerCount}'),
                        _StatRow('DHT Size', '${s.dhtSize}'),
                        _StatRow(
                            'Bootstrapped', s.isBootstrapped ? 'Yes' : 'No'),
                        _StatRow('Uptime', _formatUptime(s.uptimeSecs)),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),

          // Replication card
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Replication',
                      style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 12),
                  replication.when(
                    loading: () => const CircularProgressIndicator(),
                    error: (e, _) => Text('Error: $e'),
                    data: (r) => Column(
                      children: [
                        _StatRow('Total Fragments', '${r.totalFragments}'),
                        _StatRow('Healthy', '${r.healthyFragments}'),
                        _StatRow('Repairing', '${r.repairingFragments}'),
                        _StatRow(
                            'Replication Factor', '${r.replicationFactor}'),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  String _formatUptime(BigInt secs) {
    final s = secs.toInt();
    if (s < 60) return '${s}s';
    if (s < 3600) return '${s ~/ 60}m ${s % 60}s';
    return '${s ~/ 3600}h ${(s % 3600) ~/ 60}m';
  }
}

class _StatRow extends StatelessWidget {
  final String label;
  final String value;

  const _StatRow(this.label, this.value);

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label),
          Text(value, style: const TextStyle(fontWeight: FontWeight.bold)),
        ],
      ),
    );
  }
}

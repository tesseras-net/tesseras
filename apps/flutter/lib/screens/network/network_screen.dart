import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/network_event.dart';
import '../../providers/mock_data.dart';
import '../../providers/mock_network_provider.dart';
import 'network_event_tile.dart';

class NetworkScreen extends ConsumerWidget {
  const NetworkScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final events = ref.watch(mockNetworkEventsProvider);
    final peers = ref.watch(mockConnectedPeersProvider);
    final stats = mockNetworkStats;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Toolbar
          Row(
            children: [
              Text('Network',
                  style: Theme.of(context).textTheme.headlineSmall),
              const Spacer(),
              Tooltip(
                message: 'Refresh',
                child: IconButton(
                  icon: const Icon(Icons.refresh),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                          content: Text('Network data refreshed'),
                          duration: Duration(seconds: 1)),
                    );
                  },
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          // Scrollable content
          Expanded(
            child: ListView(
              children: [
                // Top row: two stat cards
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: Card(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text('Node Status',
                                  style: Theme.of(context)
                                      .textTheme
                                      .titleMedium),
                              const SizedBox(height: 12),
                              _StatRow('Peers', '${stats.connectedPeers}'),
                              _StatRow('DHT Entries', '${stats.dhtEntries}'),
                              _StatRow('Bootstrapped',
                                  stats.bootstrapped ? 'Yes' : 'No'),
                              _StatRow('Uptime', stats.uptime),
                              _StatRow('NAT', stats.natStatus),
                            ],
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Card(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text('Replication',
                                  style: Theme.of(context)
                                      .textTheme
                                      .titleMedium),
                              const SizedBox(height: 12),
                              _StatRow(
                                  'Fragments', '${stats.totalFragments}'),
                              _StatRow(
                                  'Healthy', '${stats.healthyFragments}'),
                              _StatRow(
                                  'Repairing', '${stats.repairingFragments}'),
                              _StatRow(
                                  'Factor', '${stats.replicationFactor}x'),
                              _StatRow(
                                  'Storage', '${stats.storageUsedMB} MB'),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                // Connected Peers table
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Connected Peers',
                            style: Theme.of(context).textTheme.titleMedium),
                        const SizedBox(height: 12),
                        Table(
                          columnWidths: const {
                            0: FlexColumnWidth(2),
                            1: FlexColumnWidth(3),
                            2: FlexColumnWidth(1.5),
                          },
                          children: [
                            TableRow(
                              decoration: BoxDecoration(
                                border: Border(
                                  bottom: BorderSide(
                                      color: Theme.of(context).dividerColor),
                                ),
                              ),
                              children: [
                                _tableHeader(context, 'Node ID'),
                                _tableHeader(context, 'Address'),
                                _tableHeader(context, 'Last Seen'),
                              ],
                            ),
                            ...peers.map((peer) => TableRow(
                                  children: [
                                    _tableCell(context, peer.nodeId),
                                    _tableCell(context, peer.address),
                                    _tableCell(context, peer.lastSeen),
                                  ],
                                )),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 16),
                // Recent Events
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text('Recent Events',
                            style: Theme.of(context).textTheme.titleMedium),
                        const SizedBox(height: 12),
                        ...events.map((e) => NetworkEventTile(event: e)),
                      ],
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _tableHeader(BuildContext context, String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(text,
          style: Theme.of(context)
              .textTheme
              .labelMedium
              ?.copyWith(fontWeight: FontWeight.bold)),
    );
  }

  Widget _tableCell(BuildContext context, String text) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Text(text,
          style: Theme.of(context)
              .textTheme
              .bodySmall
              ?.copyWith(fontFamily: 'monospace')),
    );
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

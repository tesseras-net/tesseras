import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
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
    final l = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Toolbar
          Row(
            children: [
              Text(l.networkTitle,
                  style: Theme.of(context).textTheme.headlineSmall),
              const Spacer(),
              Tooltip(
                message: l.networkRefreshTooltip,
                child: IconButton(
                  icon: const Icon(Icons.refresh),
                  onPressed: () {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                          content: Text(l.networkRefreshed),
                          duration: const Duration(seconds: 1)),
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
                              Text(l.networkNodeStatus,
                                  style: Theme.of(context)
                                      .textTheme
                                      .titleMedium),
                              const SizedBox(height: 12),
                              _StatRow(l.networkStatPeers,
                                  '${stats.connectedPeers}'),
                              _StatRow(l.networkStatDhtEntries,
                                  '${stats.dhtEntries}'),
                              _StatRow(
                                  l.networkStatBootstrapped,
                                  stats.bootstrapped
                                      ? l.networkStatYes
                                      : l.networkStatNo),
                              _StatRow(l.networkStatUptime, stats.uptime),
                              _StatRow(l.networkStatNat, stats.natStatus),
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
                              Text(l.networkReplication,
                                  style: Theme.of(context)
                                      .textTheme
                                      .titleMedium),
                              const SizedBox(height: 12),
                              _StatRow(l.networkStatFragments,
                                  '${stats.totalFragments}'),
                              _StatRow(l.networkStatHealthy,
                                  '${stats.healthyFragments}'),
                              _StatRow(l.networkStatRepairing,
                                  '${stats.repairingFragments}'),
                              _StatRow(l.networkStatFactor,
                                  '${stats.replicationFactor}x'),
                              _StatRow(l.networkStatStorage,
                                  '${stats.storageUsedMB} MB'),
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
                        Text(l.networkConnectedPeers,
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
                                _tableHeader(context, l.networkColNodeId),
                                _tableHeader(context, l.networkColAddress),
                                _tableHeader(context, l.networkColLastSeen),
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
                        Text(l.networkRecentEvents,
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

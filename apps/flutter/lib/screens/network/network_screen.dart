import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../providers/network_provider.dart';

class NetworkScreen extends ConsumerWidget {
  const NetworkScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final networkAsync = ref.watch(networkProvider);
    final networkState = networkAsync.value;
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);

    if (networkState == null) {
      return const Center(child: CircularProgressIndicator());
    }

    final stats = networkState.stats;
    final replication = networkState.replication;
    final peers = networkState.peers;

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Toolbar
          Row(
            children: [
              Text(l.networkTitle).h3.semiBold,
              const Spacer(),
              Button(
                style: const ButtonStyle.ghost(density: ButtonDensity.icon),
                onPressed: () {
                  ref.read(networkProvider.notifier).refresh();
                  showToast(
                    context: context,
                    builder: (context, overlay) => SurfaceCard(
                      child: Basic(
                        title: Text(l.networkRefreshed),
                        trailing: Button(
                          style: const ButtonStyle.ghost(
                              density: ButtonDensity.icon),
                          onPressed: overlay.close,
                          child: const Icon(Icons.close, size: 16),
                        ),
                      ),
                    ),
                    showDuration: const Duration(seconds: 1),
                  );
                },
                child: const Icon(Icons.refresh),
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
                      child: SurfaceCard(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(l.networkNodeStatus).semiBold,
                              const SizedBox(height: 12),
                              _StatRow(l.networkStatPeers,
                                  '${stats.peerCount}'),
                              _StatRow(l.networkStatDhtEntries,
                                  '${stats.dhtSize}'),
                              _StatRow(
                                  l.networkStatBootstrapped,
                                  stats.isBootstrapped
                                      ? l.networkStatYes
                                      : l.networkStatNo),
                              _StatRow(l.networkStatUptime,
                                  _formatUptime(stats.uptimeSecs)),
                              _StatRow(
                                  'Upload', _formatBytes(stats.bytesTx)),
                              _StatRow(
                                  'Download', _formatBytes(stats.bytesRx)),
                              _StatRow(l.networkStatNat, '-'),
                            ],
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: SurfaceCard(
                        child: Padding(
                          padding: const EdgeInsets.all(16),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(l.networkReplication).semiBold,
                              const SizedBox(height: 12),
                              _StatRow(l.networkStatFragments,
                                  '${replication.totalFragments}'),
                              _StatRow(l.networkStatHealthy,
                                  '${replication.healthyFragments}'),
                              _StatRow(l.networkStatRepairing,
                                  '${replication.repairingFragments}'),
                              _StatRow(l.networkStatFactor,
                                  '${replication.replicationFactor}x'),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),
                // Connected Peers table
                SurfaceCard(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(l.networkConnectedPeers).semiBold,
                        const SizedBox(height: 12),
                        // Header row
                        Row(
                          children: [
                            Expanded(flex: 2, child: _tableHeader(context, l.networkColNodeId)),
                            Expanded(flex: 3, child: _tableHeader(context, l.networkColAddress)),
                          ],
                        ),
                        Divider(color: theme.colorScheme.border),
                        // Peer rows
                        ...peers.map((peer) => Row(
                              children: [
                                Expanded(flex: 2, child: _tableCell(context, peer.nodeId.length > 12
                                    ? '${peer.nodeId.substring(0, 12)}...'
                                    : peer.nodeId)),
                                Expanded(flex: 3, child: _tableCell(context, peer.addr)),
                              ],
                            )),
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

  String _formatBytes(BigInt bytes) {
    final b = bytes.toInt();
    if (b < 1024) return '$b B';
    if (b < 1024 * 1024) return '${(b / 1024).toStringAsFixed(1)} KB';
    if (b < 1024 * 1024 * 1024) {
      return '${(b / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(b / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  String _formatUptime(BigInt secs) {
    final s = secs.toInt();
    final hours = s ~/ 3600;
    final minutes = (s % 3600) ~/ 60;
    if (hours > 0) return '${hours}h ${minutes}m';
    return '${minutes}m';
  }

  Widget _tableHeader(BuildContext context, String text) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Text(text).small.semiBold,
    );
  }

  Widget _tableCell(BuildContext context, String text) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Text(text).small.mono,
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
          Text(label).small,
          Text(value).small.semiBold,
        ],
      ),
    );
  }
}

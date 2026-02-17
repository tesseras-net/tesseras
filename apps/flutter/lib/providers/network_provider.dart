import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../src/rust/api/simple.dart' as rust;
import '../src/rust/types.dart';
import 'node_provider.dart';

/// Combined network state from real Rust node.
class NetworkState {
  final NetworkStats stats;
  final ReplicationStatus replication;
  final List<PeerInfo> peers;

  const NetworkState({
    required this.stats,
    required this.replication,
    required this.peers,
  });
}

final networkProvider =
    AsyncNotifierProvider<NetworkNotifier, NetworkState>(() {
  return NetworkNotifier();
});

class NetworkNotifier extends AsyncNotifier<NetworkState> {
  Timer? _pollTimer;

  @override
  Future<NetworkState> build() async {
    final nodeRunning = ref.watch(nodeProvider);
    if (nodeRunning is! AsyncData || nodeRunning.value != true) {
      return NetworkState(
        stats: NetworkStats(
          peerCount: 0,
          dhtSize: 0,
          isBootstrapped: false,
          uptimeSecs: BigInt.zero,
          bytesTx: BigInt.zero,
          bytesRx: BigInt.zero,
        ),
        replication: const ReplicationStatus(
          totalFragments: 0,
          healthyFragments: 0,
          repairingFragments: 0,
          replicationFactor: 7,
        ),
        peers: const [],
      );
    }

    // Start polling every 5 seconds
    _pollTimer?.cancel();
    _pollTimer = Timer.periodic(const Duration(seconds: 5), (_) => _poll());

    ref.onDispose(() => _pollTimer?.cancel());

    return _fetchState();
  }

  NetworkState _fetchState() {
    final stats = rust.getNetworkStats();
    final replication = rust.getReplicationStatus();
    final peers = rust.getConnectedPeers();
    return NetworkState(
      stats: stats,
      replication: replication,
      peers: peers,
    );
  }

  void _poll() {
    try {
      state = AsyncData(_fetchState());
    } catch (_) {
      // Ignore poll errors silently
    }
  }

  Future<void> refresh() async {
    state = AsyncData(_fetchState());
  }
}

import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/network_event.dart';
import 'mock_data.dart';

final mockNetworkEventsProvider = StateProvider<List<NetworkEvent>>((ref) {
  return mockNetworkEvents;
});

final mockConnectedPeersProvider = StateProvider<List<ConnectedPeer>>((ref) {
  return mockConnectedPeers;
});

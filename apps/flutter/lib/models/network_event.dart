import 'package:flutter/material.dart';

/// Display model for network events.
enum NetworkEventType {
  peerConnected('Peer connected', Icons.person_add),
  peerDisconnected('Peer disconnected', Icons.person_remove),
  attestationReceived('Attestation received', Icons.verified),
  replicationComplete('Replication complete', Icons.check_circle),
  repairTriggered('Repair triggered', Icons.build),
  bootstrapComplete('Bootstrap complete', Icons.rocket_launch);

  final String label;
  final IconData icon;
  const NetworkEventType(this.label, this.icon);

  Color color(ColorScheme scheme) => switch (this) {
        peerConnected => Colors.green,
        peerDisconnected => Colors.orange,
        attestationReceived => scheme.primary,
        replicationComplete => Colors.green,
        repairTriggered => Colors.orange,
        bootstrapComplete => scheme.primary,
      };
}

class NetworkEvent {
  final String timestamp;
  final NetworkEventType type;
  final String details;

  const NetworkEvent({
    required this.timestamp,
    required this.type,
    required this.details,
  });
}

/// Connected peer for the peers table.
class ConnectedPeer {
  final String nodeId;
  final String address;
  final String lastSeen;

  const ConnectedPeer({
    required this.nodeId,
    required this.address,
    required this.lastSeen,
  });
}

import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../l10n/app_localizations.dart';

/// Display model for network events.
enum NetworkEventType {
  peerConnected(Icons.person_add),
  peerDisconnected(Icons.person_remove),
  attestationReceived(Icons.verified),
  replicationComplete(Icons.check_circle),
  repairTriggered(Icons.build),
  bootstrapComplete(Icons.rocket_launch);

  final IconData icon;
  const NetworkEventType(this.icon);

  String label(AppLocalizations l) => switch (this) {
        peerConnected => l.eventPeerConnected,
        peerDisconnected => l.eventPeerDisconnected,
        attestationReceived => l.eventAttestationReceived,
        replicationComplete => l.eventReplicationComplete,
        repairTriggered => l.eventRepairTriggered,
        bootstrapComplete => l.eventBootstrapComplete,
      };

  Color color(ColorScheme scheme) => switch (this) {
        peerConnected => const Color(0xFF4CAF50),
        peerDisconnected => const Color(0xFFFF9800),
        attestationReceived => scheme.primary,
        replicationComplete => const Color(0xFF4CAF50),
        repairTriggered => const Color(0xFFFF9800),
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

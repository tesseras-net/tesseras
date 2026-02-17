import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../models/network_event.dart';

class NetworkEventTile extends StatelessWidget {
  final NetworkEvent event;

  const NetworkEventTile({super.key, required this.event});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          SizedBox(
            width: 50,
            child: Text(event.timestamp).small.mono.muted,
          ),
          const SizedBox(width: 12),
          Icon(event.type.icon,
              size: 16, color: event.type.color(theme.colorScheme)),
          const SizedBox(width: 8),
          Expanded(child: Text(event.type.label(l))),
          Text(event.details).small.mono.muted,
        ],
      ),
    );
  }
}

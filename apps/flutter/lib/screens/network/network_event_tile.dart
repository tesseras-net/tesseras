import 'package:flutter/material.dart';
import '../../l10n/app_localizations.dart';
import '../../models/network_event.dart';

class NetworkEventTile extends StatelessWidget {
  final NetworkEvent event;

  const NetworkEventTile({super.key, required this.event});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final l = AppLocalizations.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          SizedBox(
            width: 50,
            child: Text(
              event.timestamp,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                    color: colorScheme.onSurfaceVariant,
                  ),
            ),
          ),
          const SizedBox(width: 12),
          Icon(event.type.icon, size: 16, color: event.type.color(colorScheme)),
          const SizedBox(width: 8),
          Expanded(
            child: Text(event.type.label(l),
                style: Theme.of(context).textTheme.bodyMedium),
          ),
          Text(
            event.details,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                  color: colorScheme.onSurfaceVariant,
                ),
          ),
        ],
      ),
    );
  }
}

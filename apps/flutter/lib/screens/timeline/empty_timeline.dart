import 'package:flutter/material.dart';
import '../../l10n/app_localizations.dart';

class EmptyTimeline extends StatelessWidget {
  const EmptyTimeline({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.photo_library_outlined,
              size: 80, color: Theme.of(context).colorScheme.onSurfaceVariant),
          const SizedBox(height: 16),
          Text(l.emptyTimelineHeading,
              style: Theme.of(context).textTheme.headlineSmall),
          const SizedBox(height: 8),
          Text(l.emptyTimelineSubtitle,
              style: Theme.of(context).textTheme.bodyLarge),
          const SizedBox(height: 4),
          Text(l.emptyTimelineHint,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  )),
        ],
      ),
    );
  }
}

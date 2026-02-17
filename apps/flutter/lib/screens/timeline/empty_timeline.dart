import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';

class EmptyTimeline extends StatelessWidget {
  const EmptyTimeline({super.key});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Icon(Icons.photo_library_outlined,
              size: 80, color: theme.colorScheme.mutedForeground),
          const SizedBox(height: 16),
          Text(l.emptyTimelineHeading).h3.semiBold,
          const SizedBox(height: 8),
          Text(l.emptyTimelineSubtitle).large,
          const SizedBox(height: 4),
          Text(l.emptyTimelineHint).small.muted,
        ],
      ),
    );
  }
}

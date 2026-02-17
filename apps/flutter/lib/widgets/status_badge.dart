import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../l10n/app_localizations.dart';
import '../models/visibility.dart' as v;

/// Small colored badge for visibility or memory type.
class StatusBadge extends StatelessWidget {
  final String label;
  final Color? color;
  final IconData? icon;

  const StatusBadge({super.key, required this.label, this.color, this.icon});

  /// Badge for Visibility values with appropriate icons.
  factory StatusBadge.visibility(v.Visibility visibility, AppLocalizations l) {
    final (icon, label) = switch (visibility) {
      v.Visibility.private => (Icons.lock, l.badgePrivate),
      v.Visibility.circle => (Icons.group, l.badgeCircle),
      v.Visibility.public => (Icons.public, l.badgePublic),
      v.Visibility.publicAfterDeath => (Icons.schedule, l.badgeAfterDeath),
      v.Visibility.sealed_ => (Icons.lock_clock, l.badgeSealed),
    };
    return StatusBadge(label: label, icon: icon);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color ?? theme.colorScheme.muted,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 14, color: theme.colorScheme.mutedForeground),
            const SizedBox(width: 4),
          ],
          Text(label).small,
        ],
      ),
    );
  }
}

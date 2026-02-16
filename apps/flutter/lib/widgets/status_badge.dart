import 'package:flutter/material.dart';
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
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: color ?? colorScheme.primaryContainer.withValues(alpha: 0.8),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 14, color: colorScheme.onPrimaryContainer),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: colorScheme.onPrimaryContainer,
                ),
          ),
        ],
      ),
    );
  }
}

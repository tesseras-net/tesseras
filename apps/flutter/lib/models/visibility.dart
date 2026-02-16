import '../l10n/app_localizations.dart';

/// Mirrors tesseras-core Visibility enum.
enum Visibility {
  private,
  circle,
  public,
  publicAfterDeath,
  sealed_;

  String label(AppLocalizations l) => switch (this) {
        private => l.visibilityPrivate,
        circle => l.visibilityCircle,
        public => l.visibilityPublic,
        publicAfterDeath => l.visibilityPublicAfterDeath,
        sealed_ => l.visibilitySealed,
      };
}

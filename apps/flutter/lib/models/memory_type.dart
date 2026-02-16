import '../l10n/app_localizations.dart';

/// Mirrors tesseras-core MemoryType enum.
enum MemoryType {
  moment,
  reflection,
  daily,
  relation,
  object;

  String label(AppLocalizations l) => switch (this) {
        moment => l.memoryTypeMoment,
        reflection => l.memoryTypeReflection,
        daily => l.memoryTypeDaily,
        relation => l.memoryTypeRelation,
        object => l.memoryTypeObject,
      };
}

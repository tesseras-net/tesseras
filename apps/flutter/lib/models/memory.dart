import '../src/rust/types.dart';
import 'memory_type.dart';
import 'visibility.dart';

/// Display model for a memory in the UI.
/// Richer than the FFI MemoryInfo — includes all metadata fields.
class Memory {
  final String hash;
  final String tesseraHash;
  final MemoryType type;
  final Visibility visibility;
  final String? context;
  final String createdAt;
  final List<String> tags;
  final String? location;
  final List<String> people;
  final String language;
  final String mediaType; // 'jpeg', 'png', 'wav', 'webm', 'txt'
  final String mediaPath; // original file path from import

  // Visibility-specific fields
  final DateTime? sealedOpenAfter;
  final int? publicAfterDeathYears;

  const Memory({
    required this.hash,
    required this.tesseraHash,
    required this.type,
    required this.visibility,
    this.context,
    required this.createdAt,
    this.tags = const [],
    this.location,
    this.people = const [],
    this.language = 'en',
    this.mediaType = 'jpeg',
    this.mediaPath = '',
    this.sealedOpenAfter,
    this.publicAfterDeathYears,
  });

  /// Convert from FFI MemoryInfo to display Memory.
  factory Memory.fromMemoryInfo(MemoryInfo info) {
    return Memory(
      hash: info.hash,
      tesseraHash: info.tesseraHash,
      type: _parseMemoryType(info.memoryType),
      visibility: _parseVisibility(info.visibility),
      context: info.context,
      createdAt: info.createdAt,
      tags: info.tags,
      location: info.location,
      people: info.people,
      language: info.language,
      mediaType: info.mediaType.isNotEmpty ? info.mediaType : 'jpeg',
      mediaPath: info.mediaPath,
      sealedOpenAfter: info.sealedOpenAfter != null
          ? DateTime.tryParse(info.sealedOpenAfter!)
          : null,
      publicAfterDeathYears: info.publicAfterDeathYears,
    );
  }

  static MemoryType _parseMemoryType(String s) => switch (s.toLowerCase()) {
        'moment' => MemoryType.moment,
        'reflection' => MemoryType.reflection,
        'daily' => MemoryType.daily,
        'relation' => MemoryType.relation,
        'object' => MemoryType.object,
        _ => MemoryType.moment,
      };

  static Visibility _parseVisibility(String s) => switch (s.toLowerCase()) {
        'private' => Visibility.private,
        'circle' => Visibility.circle,
        'public' => Visibility.public,
        'publicafterdeath' => Visibility.publicAfterDeath,
        'sealed' => Visibility.sealed_,
        _ => Visibility.private,
      };
}

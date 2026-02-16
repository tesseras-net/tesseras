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
    this.sealedOpenAfter,
    this.publicAfterDeathYears,
  });
}

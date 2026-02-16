import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../models/memory.dart';
import '../../models/visibility.dart' as v;
import '../../widgets/placeholder_image.dart';
import '../../widgets/status_badge.dart';

/// Memory detail dialog with full metadata, left/right navigation.
class MemoryDetailDialog extends StatefulWidget {
  final List<Memory> memories;
  final int initialIndex;

  const MemoryDetailDialog({
    super.key,
    required this.memories,
    required this.initialIndex,
  });

  @override
  State<MemoryDetailDialog> createState() => _MemoryDetailDialogState();
}

class _MemoryDetailDialogState extends State<MemoryDetailDialog> {
  late int _currentIndex;

  @override
  void initState() {
    super.initState();
    _currentIndex = widget.initialIndex;
  }

  Memory get _memory => widget.memories[_currentIndex];

  void _previous() {
    if (_currentIndex > 0) {
      setState(() => _currentIndex--);
    }
  }

  void _next() {
    if (_currentIndex < widget.memories.length - 1) {
      setState(() => _currentIndex++);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final memory = _memory;
    final isText = memory.mediaType == 'txt';
    final isAudio = memory.mediaType == 'wav' || memory.mediaType == 'webm';

    return Shortcuts(
      shortcuts: {
        LogicalKeySet(LogicalKeyboardKey.arrowLeft): const _PreviousIntent(),
        LogicalKeySet(LogicalKeyboardKey.arrowRight): const _NextIntent(),
      },
      child: Actions(
        actions: {
          _PreviousIntent:
              CallbackAction<_PreviousIntent>(onInvoke: (_) {
            _previous();
            return null;
          }),
          _NextIntent: CallbackAction<_NextIntent>(onInvoke: (_) {
            _next();
            return null;
          }),
        },
        child: Focus(
          autofocus: true,
          child: Dialog(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 800, maxHeight: 700),
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // Header
                    Row(
                      children: [
                        Text('Memory Detail',
                            style: Theme.of(context).textTheme.titleLarge),
                        const Spacer(),
                        Text(
                          '${_currentIndex + 1} / ${widget.memories.length}',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                        const SizedBox(width: 8),
                        IconButton(
                          icon: const Icon(Icons.close),
                          onPressed: () => Navigator.of(context).pop(),
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                    // Scrollable content
                    Flexible(
                      child: SingleChildScrollView(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            // Media preview
                            if (isText)
                              Container(
                                padding: const EdgeInsets.all(20),
                                decoration: BoxDecoration(
                                  color: colorScheme.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Text(
                                  memory.context ?? '',
                                  style: Theme.of(context)
                                      .textTheme
                                      .bodyLarge
                                      ?.copyWith(
                                        fontStyle: FontStyle.italic,
                                        height: 1.6,
                                      ),
                                ),
                              )
                            else if (isAudio)
                              Container(
                                height: 80,
                                decoration: BoxDecoration(
                                  color: colorScheme.surfaceContainerHighest,
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Row(
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    IconButton(
                                      icon: const Icon(Icons.play_arrow,
                                          size: 36),
                                      onPressed: () {
                                        ScaffoldMessenger.of(context)
                                            .showSnackBar(
                                          const SnackBar(
                                              content: Text(
                                                  'Audio playback not available in mockup')),
                                        );
                                      },
                                    ),
                                    const SizedBox(width: 12),
                                    Expanded(
                                      child: Container(
                                        height: 32,
                                        margin:
                                            const EdgeInsets.only(right: 20),
                                        decoration: BoxDecoration(
                                          color: colorScheme.primary
                                              .withValues(alpha: 0.2),
                                          borderRadius:
                                              BorderRadius.circular(4),
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              )
                            else
                              ClipRRect(
                                borderRadius: BorderRadius.circular(8),
                                child: PlaceholderImage(
                                  hash: memory.hash,
                                  mediaType: memory.mediaType,
                                  height: 300,
                                ),
                              ),
                            const SizedBox(height: 16),
                            // Context (for non-text media)
                            if (!isText && memory.context != null) ...[
                              Text('Context',
                                  style:
                                      Theme.of(context).textTheme.titleSmall),
                              const SizedBox(height: 4),
                              Text(memory.context!,
                                  style:
                                      Theme.of(context).textTheme.bodyMedium),
                              const SizedBox(height: 16),
                            ],
                            // Metadata grid
                            Wrap(
                              spacing: 12,
                              runSpacing: 8,
                              children: [
                                _MetadataChip(
                                    label: 'Type', value: memory.type.label),
                                StatusBadge.visibility(memory.visibility),
                                _MetadataChip(
                                    label: 'Created',
                                    value: _formatDate(memory.createdAt)),
                                _MetadataChip(
                                    label: 'Language',
                                    value: memory.language.toUpperCase()),
                                _MetadataChip(
                                    label: 'Media',
                                    value: memory.mediaType.toUpperCase()),
                              ],
                            ),
                            // Sealed/PublicAfterDeath details
                            if (memory.visibility == v.Visibility.sealed_ &&
                                memory.sealedOpenAfter != null) ...[
                              const SizedBox(height: 8),
                              Text(
                                'Opens after: ${_formatDateTime(memory.sealedOpenAfter!)}',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: Colors.orange),
                              ),
                            ],
                            if (memory.visibility ==
                                    v.Visibility.publicAfterDeath &&
                                memory.publicAfterDeathYears != null) ...[
                              const SizedBox(height: 8),
                              Text(
                                'Public after ${memory.publicAfterDeathYears} years of inactivity',
                                style: Theme.of(context)
                                    .textTheme
                                    .bodySmall
                                    ?.copyWith(color: Colors.orange),
                              ),
                            ],
                            // Tags
                            if (memory.tags.isNotEmpty) ...[
                              const SizedBox(height: 12),
                              Wrap(
                                spacing: 6,
                                runSpacing: 4,
                                children: memory.tags
                                    .map((tag) => Chip(
                                          label: Text(tag),
                                          visualDensity: VisualDensity.compact,
                                        ))
                                    .toList(),
                              ),
                            ],
                            // Location
                            if (memory.location != null) ...[
                              const SizedBox(height: 12),
                              Row(
                                children: [
                                  Icon(Icons.location_on,
                                      size: 16,
                                      color: colorScheme.onSurfaceVariant),
                                  const SizedBox(width: 4),
                                  Text(memory.location!),
                                ],
                              ),
                            ],
                            // People
                            if (memory.people.isNotEmpty) ...[
                              const SizedBox(height: 8),
                              Row(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Icon(Icons.people,
                                      size: 16,
                                      color: colorScheme.onSurfaceVariant),
                                  const SizedBox(width: 4),
                                  Expanded(
                                      child:
                                          Text(memory.people.join(', '))),
                                ],
                              ),
                            ],
                            // Tessera hash
                            const SizedBox(height: 12),
                            Row(
                              children: [
                                Text('Tessera: ',
                                    style:
                                        Theme.of(context).textTheme.bodySmall),
                                Expanded(
                                  child: Text(
                                    memory.tesseraHash,
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodySmall
                                        ?.copyWith(fontFamily: 'monospace'),
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                    const SizedBox(height: 16),
                    // Actions
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        OutlinedButton.icon(
                          onPressed: () {
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(
                                  content:
                                      Text('Exported to ~/Downloads')),
                            );
                          },
                          icon: const Icon(Icons.download, size: 18),
                          label: const Text('Export'),
                        ),
                        const SizedBox(width: 8),
                        OutlinedButton.icon(
                          onPressed: () {
                            ScaffoldMessenger.of(context).showSnackBar(
                              const SnackBar(
                                  content: Text(
                                      'Tessera verified successfully')),
                            );
                          },
                          icon: const Icon(Icons.verified, size: 18),
                          label: const Text('Verify'),
                        ),
                        const SizedBox(width: 8),
                        FilledButton(
                          onPressed: () => Navigator.of(context).pop(),
                          child: const Text('Close'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  String _formatDate(String isoDate) {
    final dt = DateTime.tryParse(isoDate);
    if (dt == null) return isoDate;
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }

  String _formatDateTime(DateTime dt) {
    return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
  }
}

class _MetadataChip extends StatelessWidget {
  final String label;
  final String value;

  const _MetadataChip({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text('$label: $value',
          style: Theme.of(context).textTheme.bodySmall),
    );
  }
}

class _PreviousIntent extends Intent {
  const _PreviousIntent();
}

class _NextIntent extends Intent {
  const _NextIntent();
}

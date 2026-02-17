import 'dart:async';

import 'package:flutter/services.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
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
  bool _verified = false;
  Timer? _verifyTimer;

  @override
  void initState() {
    super.initState();
    _currentIndex = widget.initialIndex;
  }

  @override
  void dispose() {
    _verifyTimer?.cancel();
    super.dispose();
  }

  Memory get _memory => widget.memories[_currentIndex];

  void _previous() {
    if (_currentIndex > 0) {
      setState(() {
        _currentIndex--;
        _verified = false;
      });
    }
  }

  void _next() {
    if (_currentIndex < widget.memories.length - 1) {
      setState(() {
        _currentIndex++;
        _verified = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final memory = _memory;
    final isText = memory.mediaType == 'txt';
    final isAudio = memory.mediaType == 'wav' || memory.mediaType == 'webm';
    final l = AppLocalizations.of(context);

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
          child: AlertDialog(
            title: Text(memory.context != null && memory.context!.isNotEmpty
                ? memory.context!.length > 40
                    ? '${memory.context!.substring(0, 40)}...'
                    : memory.context!
                : '${memory.type.label(l)} — ${_formatDate(memory.createdAt)}'),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text('${_currentIndex + 1} / ${widget.memories.length}')
                    .small
                    .muted,
                const SizedBox(width: 8),
                Button(
                  style:
                      const ButtonStyle.ghost(density: ButtonDensity.icon),
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 16),
                ),
              ],
            ),
            content: ConstrainedBox(
              constraints:
                  const BoxConstraints(maxWidth: 800, maxHeight: 550),
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    // Media preview
                    if (isText)
                      Container(
                        padding: const EdgeInsets.all(20),
                        decoration: BoxDecoration(
                          color: theme.colorScheme.muted,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Text(
                          memory.context ?? '',
                          style: const TextStyle(
                            fontStyle: FontStyle.italic,
                            height: 1.6,
                          ),
                        ),
                      )
                    else if (isAudio)
                      Container(
                        height: 80,
                        decoration: BoxDecoration(
                          color: theme.colorScheme.muted,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Button(
                              style: const ButtonStyle.ghost(
                                  density: ButtonDensity.icon),
                              onPressed: () {
                                showToast(
                                  context: context,
                                  builder: (context, overlay) => SurfaceCard(
                                    child: Basic(
                                      title: Text(
                                          l.memoryDetailAudioUnavailable),
                                      trailing: Button(
                                        style: const ButtonStyle.ghost(
                                            density: ButtonDensity.icon),
                                        onPressed: overlay.close,
                                        child: const Icon(Icons.close,
                                            size: 16),
                                      ),
                                    ),
                                  ),
                                  showDuration: const Duration(seconds: 2),
                                );
                              },
                              child:
                                  const Icon(Icons.play_arrow, size: 36),
                            ),
                            const SizedBox(width: 12),
                            Expanded(
                              child: Container(
                                height: 32,
                                margin: const EdgeInsets.only(right: 20),
                                decoration: BoxDecoration(
                                  color: theme.colorScheme.primary
                                      .withValues(alpha: 0.2),
                                  borderRadius: BorderRadius.circular(4),
                                ),
                              ),
                            ),
                          ],
                        ),
                      )
                    else
                      PlaceholderImage(
                        hash: memory.hash,
                        mediaType: memory.mediaType,
                        mediaPath: memory.mediaPath,
                        tesseraHash: memory.tesseraHash,
                        height: 300,
                      ),
                    const SizedBox(height: 16),
                    // Context (for non-text media)
                    if (!isText && memory.context != null) ...[
                      Text(l.memoryDetailContextLabel).small.semiBold,
                      const SizedBox(height: 4),
                      Text(memory.context!),
                      const SizedBox(height: 16),
                    ],
                    // Metadata grid
                    Wrap(
                      spacing: 12,
                      runSpacing: 8,
                      children: [
                        _MetadataChip(
                            label: l.memoryDetailTypeLabel,
                            value: memory.type.label(l)),
                        StatusBadge.visibility(memory.visibility, l),
                        _MetadataChip(
                            label: l.memoryDetailCreatedLabel,
                            value: _formatDate(memory.createdAt)),
                        _MetadataChip(
                            label: l.memoryDetailLanguageLabel,
                            value: memory.language.toUpperCase()),
                        _MetadataChip(
                            label: l.memoryDetailMediaLabel,
                            value: memory.mediaType.toUpperCase()),
                        _StatusChip(label: 'Status', value: 'Published'),
                      ],
                    ),
                    // Sealed/PublicAfterDeath details
                    if (memory.visibility == v.Visibility.sealed_ &&
                        memory.sealedOpenAfter != null) ...[
                      const SizedBox(height: 8),
                      Text(l.memoryDetailOpensAfter(
                              _formatDateTime(memory.sealedOpenAfter!)))
                          .small,
                    ],
                    if (memory.visibility ==
                            v.Visibility.publicAfterDeath &&
                        memory.publicAfterDeathYears != null) ...[
                      const SizedBox(height: 8),
                      Text(l.memoryDetailPublicAfterDeath(
                              memory.publicAfterDeathYears!))
                          .small,
                    ],
                    // Tags
                    if (memory.tags.isNotEmpty) ...[
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 6,
                        runSpacing: 4,
                        children: memory.tags
                            .map((tag) => Chip(child: Text(tag)))
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
                              color: theme.colorScheme.mutedForeground),
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
                              color: theme.colorScheme.mutedForeground),
                          const SizedBox(width: 4),
                          Expanded(
                              child: Text(memory.people.join(', '))),
                        ],
                      ),
                    ],
                    // Tessera hash
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        Text(l.memoryDetailTesseraLabel).small,
                        Expanded(
                          child: Text(memory.tesseraHash).small.mono.muted,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            actions: [
              Button.outline(
                onPressed: () {
                  showToast(
                    context: context,
                    builder: (context, overlay) => SurfaceCard(
                      child: Basic(
                        title: Text(l.memoryDetailExported),
                        trailing: Button(
                          style: const ButtonStyle.ghost(
                              density: ButtonDensity.icon),
                          onPressed: overlay.close,
                          child: const Icon(Icons.close, size: 16),
                        ),
                      ),
                    ),
                    showDuration: const Duration(seconds: 2),
                  );
                },
                leading: const Icon(Icons.download, size: 18),
                child: Text(l.memoryDetailExport),
              ),
              const SizedBox(width: 8),
              _verified
                  ? Button.outline(
                      onPressed: null,
                      leading: const Icon(Icons.check_circle,
                          size: 18, color: Color(0xFF4CAF50)),
                      child: Text(l.memoryDetailVerified),
                    )
                  : Button.outline(
                      onPressed: () {
                        setState(() => _verified = true);
                        _verifyTimer?.cancel();
                        _verifyTimer = Timer(
                          const Duration(seconds: 3),
                          () {
                            if (mounted) setState(() => _verified = false);
                          },
                        );
                      },
                      leading: const Icon(Icons.verified, size: 18),
                      child: Text(l.memoryDetailVerify),
                    ),
              const SizedBox(width: 8),
              Button.primary(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l.memoryDetailClose),
              ),
            ],
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
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.muted,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text('$label: $value').small,
    );
  }
}

class _StatusChip extends StatelessWidget {
  final String label;
  final String value;

  const _StatusChip({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
      decoration: BoxDecoration(
        color: const Color(0xFF4CAF50).withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(
            color: const Color(0xFF4CAF50).withValues(alpha: 0.4)),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 8,
            height: 8,
            decoration: const BoxDecoration(
              color: Color(0xFF4CAF50),
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 6),
          Text('$label: $value').small,
        ],
      ),
    );
  }
}

class _PreviousIntent extends Intent {
  const _PreviousIntent();
}

class _NextIntent extends Intent {
  const _NextIntent();
}

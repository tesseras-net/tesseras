import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../models/memory.dart';
import '../../models/visibility.dart' as v;
import '../../widgets/placeholder_image.dart';

/// Single grid tile for a memory in the timeline.
class MemoryTile extends StatefulWidget {
  final Memory memory;
  final VoidCallback onTap;

  const MemoryTile({super.key, required this.memory, required this.onTap});

  @override
  State<MemoryTile> createState() => _MemoryTileState();
}

class _MemoryTileState extends State<MemoryTile> {
  bool _hovering = false;

  IconData _visibilityIcon(v.Visibility visibility) => switch (visibility) {
        v.Visibility.private => Icons.lock,
        v.Visibility.circle => Icons.group,
        v.Visibility.public => Icons.public,
        v.Visibility.publicAfterDeath => Icons.schedule,
        v.Visibility.sealed_ => Icons.lock_clock,
      };

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final memory = widget.memory;
    final l = AppLocalizations.of(context);

    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: theme.colorScheme.border),
            color: theme.colorScheme.card,
            boxShadow: _hovering
                ? [
                    BoxShadow(
                      color: theme.colorScheme.border.withValues(alpha: 0.5),
                      blurRadius: 8,
                      offset: const Offset(0, 2),
                    ),
                  ]
                : null,
          ),
          clipBehavior: Clip.antiAlias,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Thumbnail with badges
              Expanded(
                flex: 3,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    PlaceholderImage(
                      hash: memory.hash,
                      mediaType: memory.mediaType,
                      mediaPath: memory.mediaPath,
                      tesseraHash: memory.tesseraHash,
                    ),
                    // Type badge (top-left)
                    Positioned(
                      top: 8,
                      left: 8,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 6, vertical: 3),
                        decoration: BoxDecoration(
                          color: const Color(0x88000000),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Text(
                          memory.type.label(l),
                          style: const TextStyle(
                              color: Color(0xFFFFFFFF), fontSize: 11),
                        ),
                      ),
                    ),
                    // Status dot (bottom-right) — green = stored locally
                    Positioned(
                      bottom: 8,
                      right: 8,
                      child: Container(
                        width: 10,
                        height: 10,
                        decoration: BoxDecoration(
                          color: const Color(0xFF4CAF50),
                          shape: BoxShape.circle,
                          border: Border.all(
                              color: const Color(0xFFFFFFFF), width: 1.5),
                        ),
                      ),
                    ),
                    // Visibility icon (top-right)
                    Positioned(
                      top: 8,
                      right: 8,
                      child: Container(
                        padding: const EdgeInsets.all(4),
                        decoration: BoxDecoration(
                          color: const Color(0x88000000),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Icon(
                          _visibilityIcon(memory.visibility),
                          size: 16,
                          color: const Color(0xFFFFFFFF),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              // Content area
              Expanded(
                flex: 2,
                child: Padding(
                  padding: const EdgeInsets.all(10),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      if (memory.context != null)
                        Expanded(
                          child: Text(
                            memory.context!,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ).small,
                        ),
                      const SizedBox(height: 4),
                      Text(_formatDate(memory.createdAt)).small.muted,
                    ],
                  ),
                ),
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
}

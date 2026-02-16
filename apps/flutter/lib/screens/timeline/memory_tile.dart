import 'package:flutter/material.dart';
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
    final colorScheme = Theme.of(context).colorScheme;
    final memory = widget.memory;

    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      child: GestureDetector(
        onTap: widget.onTap,
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 150),
          child: Card(
            elevation: _hovering ? 4 : 1,
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
                      ),
                      // Type badge (top-left)
                      Positioned(
                        top: 8,
                        left: 8,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 6, vertical: 3),
                          decoration: BoxDecoration(
                            color: Colors.black54,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Text(
                            memory.type.label,
                            style: const TextStyle(
                                color: Colors.white, fontSize: 11),
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
                            color: Colors.black54,
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: Icon(
                            _visibilityIcon(memory.visibility),
                            size: 16,
                            color: Colors.white,
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
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                          ),
                        const SizedBox(height: 4),
                        Text(
                          _formatDate(memory.createdAt),
                          style:
                              Theme.of(context).textTheme.labelSmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                  ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
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
}

import 'dart:async';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../models/memory.dart';
import '../../providers/timeline_provider.dart';
import 'empty_timeline.dart';
import 'memory_tile.dart';
import 'memory_detail_dialog.dart';

enum _SortMode { newestFirst, oldestFirst, byType }

class TimelineScreen extends ConsumerStatefulWidget {
  final FocusNode? searchFocusNode;

  const TimelineScreen({super.key, this.searchFocusNode});

  @override
  ConsumerState<TimelineScreen> createState() => _TimelineScreenState();
}

class _TimelineScreenState extends ConsumerState<TimelineScreen> {
  final _searchController = TextEditingController();
  _SortMode _sortMode = _SortMode.newestFirst;
  String _searchQuery = '';
  Timer? _debounce;

  @override
  void dispose() {
    _searchController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  void _onSearchChanged(String value) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 300), () {
      setState(() => _searchQuery = value.toLowerCase());
    });
  }

  List<Memory> _filterAndSort(List<Memory> memories) {
    var filtered = memories;
    if (_searchQuery.isNotEmpty) {
      filtered = memories.where((m) {
        final context = m.context?.toLowerCase() ?? '';
        final tags = m.tags.join(' ').toLowerCase();
        return context.contains(_searchQuery) || tags.contains(_searchQuery);
      }).toList();
    }

    switch (_sortMode) {
      case _SortMode.newestFirst:
        filtered.sort((a, b) => b.createdAt.compareTo(a.createdAt));
      case _SortMode.oldestFirst:
        filtered.sort((a, b) => a.createdAt.compareTo(b.createdAt));
      case _SortMode.byType:
        filtered.sort((a, b) => a.type.index.compareTo(b.type.index));
    }
    return filtered;
  }

  void _showMemoryDetail(List<Memory> memories, int index) {
    showDialog(
      context: context,
      builder: (context) => MemoryDetailDialog(
        memories: memories,
        initialIndex: index,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final timelineAsync = ref.watch(timelineProvider);
    final allMemories = timelineAsync.value ?? [];
    final memories = _filterAndSort(List.of(allMemories));
    final l = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          // Toolbar
          Row(
            children: [
              Text(l.timelineTitle).h3.semiBold,
              const Spacer(),
              SizedBox(
                width: 240,
                child: TextField(
                  controller: _searchController,
                  focusNode: widget.searchFocusNode,
                  onChanged: _onSearchChanged,
                  placeholder: Text(l.timelineSearchHint),
                  features: [
                    if (_searchQuery.isNotEmpty)
                      const InputClearFeature(),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Select<_SortMode>(
                value: _sortMode,
                onChanged: (mode) {
                  if (mode != null) setState(() => _sortMode = mode);
                },
                itemBuilder: (context, value) => Icon(Icons.sort, size: 18),
                popup: (context) => SelectPopup(
                  items: SelectItemList(
                    children: [
                      SelectItemButton(
                        value: _SortMode.newestFirst,
                        child: Text(l.timelineSortNewest),
                      ),
                      SelectItemButton(
                        value: _SortMode.oldestFirst,
                        child: Text(l.timelineSortOldest),
                      ),
                      SelectItemButton(
                        value: _SortMode.byType,
                        child: Text(l.timelineSortByType),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          // Grid or empty state
          Expanded(
            child: memories.isEmpty
                ? const EmptyTimeline()
                : GridView.builder(
                    gridDelegate:
                        const SliverGridDelegateWithMaxCrossAxisExtent(
                      maxCrossAxisExtent: 280,
                      childAspectRatio: 0.75,
                      crossAxisSpacing: 12,
                      mainAxisSpacing: 12,
                    ),
                    itemCount: memories.length,
                    itemBuilder: (context, index) => MemoryTile(
                      memory: memories[index],
                      onTap: () => _showMemoryDetail(memories, index),
                    ),
                  ),
          ),
        ],
      ),
    );
  }
}

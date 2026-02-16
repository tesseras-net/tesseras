import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
import '../../models/memory.dart';
import '../../providers/mock_timeline_provider.dart';
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
    final allMemories = ref.watch(mockTimelineProvider);
    final memories = _filterAndSort(List.of(allMemories));
    final l = AppLocalizations.of(context);

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        children: [
          // Toolbar
          Row(
            children: [
              Text(l.timelineTitle,
                  style: Theme.of(context).textTheme.headlineSmall),
              const Spacer(),
              SizedBox(
                width: 240,
                child: TextField(
                  controller: _searchController,
                  focusNode: widget.searchFocusNode,
                  onChanged: _onSearchChanged,
                  decoration: InputDecoration(
                    hintText: l.timelineSearchHint,
                    prefixIcon: const Icon(Icons.search, size: 20),
                    suffixIcon: _searchQuery.isNotEmpty
                        ? IconButton(
                            icon: const Icon(Icons.clear, size: 18),
                            onPressed: () {
                              _searchController.clear();
                              setState(() => _searchQuery = '');
                            },
                          )
                        : null,
                    isDense: true,
                    contentPadding: const EdgeInsets.symmetric(
                        horizontal: 12, vertical: 10),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(20),
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              PopupMenuButton<_SortMode>(
                icon: const Icon(Icons.sort),
                tooltip: l.timelineSortTooltip,
                onSelected: (mode) => setState(() => _sortMode = mode),
                itemBuilder: (_) => [
                  CheckedPopupMenuItem(
                    value: _SortMode.newestFirst,
                    checked: _sortMode == _SortMode.newestFirst,
                    child: Text(l.timelineSortNewest),
                  ),
                  CheckedPopupMenuItem(
                    value: _SortMode.oldestFirst,
                    checked: _sortMode == _SortMode.oldestFirst,
                    child: Text(l.timelineSortOldest),
                  ),
                  CheckedPopupMenuItem(
                    value: _SortMode.byType,
                    checked: _sortMode == _SortMode.byType,
                    child: Text(l.timelineSortByType),
                  ),
                ],
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

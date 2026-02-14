import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/timeline_provider.dart';
import '../../src/rust/types.dart';
import '../create_memory/create_memory_screen.dart';

class TimelineScreen extends ConsumerWidget {
  const TimelineScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final timeline = ref.watch(timelineProvider);

    return Scaffold(
      appBar: AppBar(title: const Text('Timeline')),
      floatingActionButton: FloatingActionButton(
        onPressed: () async {
          await Navigator.of(context).push(
            MaterialPageRoute(builder: (_) => const CreateMemoryScreen()),
          );
          ref.read(timelineProvider.notifier).refresh();
        },
        child: const Icon(Icons.add_a_photo),
      ),
      body: timeline.when(
        loading: () => const Center(child: CircularProgressIndicator()),
        error: (e, _) => Center(child: Text('Error: $e')),
        data: (memories) {
          if (memories.isEmpty) {
            return const Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.photo_library_outlined,
                      size: 64, color: Colors.grey),
                  SizedBox(height: 16),
                  Text('No memories yet'),
                  SizedBox(height: 8),
                  Text('Tap + to create your first memory'),
                ],
              ),
            );
          }
          return RefreshIndicator(
            onRefresh: () => ref.read(timelineProvider.notifier).refresh(),
            child: ListView.builder(
              itemCount: memories.length,
              itemBuilder: (context, index) =>
                  _MemoryCard(memory: memories[index]),
            ),
          );
        },
      ),
    );
  }
}

class _MemoryCard extends StatelessWidget {
  final MemoryInfo memory;

  const _MemoryCard({required this.memory});

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (File(memory.mediaPath).existsSync())
            Image.file(
              File(memory.mediaPath),
              height: 200,
              width: double.infinity,
              fit: BoxFit.cover,
              errorBuilder: (context, error, stackTrace) => const SizedBox(
                height: 200,
                child: Center(child: Icon(Icons.broken_image, size: 48)),
              ),
            ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (memory.context != null)
                  Text(
                    memory.context!,
                    style: Theme.of(context).textTheme.bodyLarge,
                  ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Chip(label: Text(memory.memoryType)),
                    const SizedBox(width: 8),
                    Chip(label: Text(memory.visibility)),
                    const Spacer(),
                    Text(
                      _formatDate(memory.createdAt),
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  ],
                ),
                if (memory.tags.isNotEmpty)
                  Wrap(
                    spacing: 4,
                    children: memory.tags
                        .map((t) => Chip(
                              label: Text(t),
                              visualDensity: VisualDensity.compact,
                            ))
                        .toList(),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  String _formatDate(String isoDate) {
    try {
      final dt = DateTime.parse(isoDate);
      return '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')}';
    } catch (_) {
      return isoDate;
    }
  }
}

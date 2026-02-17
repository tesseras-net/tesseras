import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/memory.dart';
import '../src/rust/api/simple.dart' as rust;
import '../src/rust/types.dart';
import 'node_provider.dart';

final timelineProvider =
    AsyncNotifierProvider<TimelineNotifier, List<Memory>>(() {
  return TimelineNotifier();
});

class TimelineNotifier extends AsyncNotifier<List<Memory>> {
  @override
  Future<List<Memory>> build() async {
    final nodeRunning = ref.watch(nodeProvider);
    if (nodeRunning is! AsyncData || nodeRunning.value != true) {
      return [];
    }
    final infos = rust.getTimeline(offset: 0, limit: 50);
    return infos.map(Memory.fromMemoryInfo).toList();
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    final infos = rust.getTimeline(offset: 0, limit: 50);
    state = AsyncData(infos.map(Memory.fromMemoryInfo).toList());
  }
}

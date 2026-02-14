import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../src/rust/api/simple.dart' as rust;
import '../src/rust/types.dart';
import 'node_provider.dart';

final timelineProvider =
    AsyncNotifierProvider<TimelineNotifier, List<MemoryInfo>>(() {
  return TimelineNotifier();
});

class TimelineNotifier extends AsyncNotifier<List<MemoryInfo>> {
  @override
  Future<List<MemoryInfo>> build() async {
    final nodeRunning = ref.watch(nodeProvider);
    if (nodeRunning is! AsyncData || nodeRunning.value != true) {
      return [];
    }
    return rust.getTimeline(offset: 0, limit: 50);
  }

  Future<void> refresh() async {
    state = const AsyncLoading();
    state = AsyncData(rust.getTimeline(offset: 0, limit: 50));
  }
}

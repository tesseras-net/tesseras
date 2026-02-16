import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/memory.dart';
import 'mock_data.dart';

final mockTimelineProvider =
    StateNotifierProvider<MockTimelineNotifier, List<Memory>>((ref) {
  return MockTimelineNotifier();
});

class MockTimelineNotifier extends StateNotifier<List<Memory>> {
  MockTimelineNotifier() : super(mockMemories);

  void addMemory(Memory memory) {
    state = [memory, ...state];
  }
}

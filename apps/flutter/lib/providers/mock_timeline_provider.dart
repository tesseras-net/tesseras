import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/memory.dart';
import 'mock_data.dart';

final mockTimelineProvider =
    NotifierProvider<MockTimelineNotifier, List<Memory>>(
        MockTimelineNotifier.new);

class MockTimelineNotifier extends Notifier<List<Memory>> {
  @override
  List<Memory> build() => mockMemories;

  void addMemory(Memory memory) {
    state = [memory, ...state];
  }
}

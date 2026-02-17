import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../src/rust/api/simple.dart' as rust;
import '../src/rust/types.dart';
import 'node_provider.dart';

final storageProvider =
    AsyncNotifierProvider<StorageNotifier, StorageStats>(() {
  return StorageNotifier();
});

class StorageNotifier extends AsyncNotifier<StorageStats> {
  @override
  Future<StorageStats> build() async {
    final nodeRunning = ref.watch(nodeProvider);
    if (nodeRunning is! AsyncData || nodeRunning.value != true) {
      return StorageStats(
        totalBytes: BigInt.zero,
        tesseraCount: 0,
        fragmentCount: 0,
      );
    }
    return rust.getStorageStats();
  }

  Future<void> refresh() async {
    state = AsyncData(rust.getStorageStats());
  }
}

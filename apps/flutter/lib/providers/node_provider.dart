import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import '../src/rust/api/simple.dart' as rust;

/// Tracks whether the embedded Rust node is running.
/// All other providers depend on this being true before calling Rust APIs.
final nodeProvider = AsyncNotifierProvider<NodeNotifier, bool>(() {
  return NodeNotifier();
});

class NodeNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    final appDir = await getApplicationDocumentsDirectory();
    final dataDir = '${appDir.path}/tesseras';

    rust.nodeStart(dataDir: dataDir);

    ref.onDispose(() {
      if (rust.nodeIsRunning()) {
        rust.nodeStop();
      }
    });

    return true;
  }
}

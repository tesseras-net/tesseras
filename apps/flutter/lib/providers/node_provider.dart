import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../src/rust/api/simple.dart' as rust;

/// Resolve the tesseras data directory, matching the CLI's default:
/// `$XDG_DATA_HOME/tesseras` or `~/.local/share/tesseras`.
/// This ensures Flutter and CLI share the same identity and storage.
String _resolveDataDir() {
  final xdgDataHome = Platform.environment['XDG_DATA_HOME'];
  if (xdgDataHome != null && xdgDataHome.isNotEmpty) {
    return '$xdgDataHome/tesseras';
  }
  final home = Platform.environment['HOME'] ?? '/tmp';
  return '$home/.local/share/tesseras';
}

/// Tracks whether the embedded Rust node is running.
/// All other providers depend on this being true before calling Rust APIs.
final nodeProvider = AsyncNotifierProvider<NodeNotifier, bool>(() {
  return NodeNotifier();
});

class NodeNotifier extends AsyncNotifier<bool> {
  @override
  Future<bool> build() async {
    final dataDir = _resolveDataDir();
    stderr.writeln('[tesseras] data_dir=$dataDir');

    rust.nodeStart(dataDir: dataDir);

    ref.onDispose(() {
      if (rust.nodeIsRunning()) {
        rust.nodeStop();
      }
    });

    return true;
  }
}

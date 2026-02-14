import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../src/rust/api/simple.dart' as rust;
import '../src/rust/types.dart';
import 'node_provider.dart';

final identityProvider =
    AsyncNotifierProvider<IdentityNotifier, IdentityInfo?>(() {
  return IdentityNotifier();
});

class IdentityNotifier extends AsyncNotifier<IdentityInfo?> {
  @override
  Future<IdentityInfo?> build() async {
    // Wait for node to be running before querying identity.
    final nodeRunning = ref.watch(nodeProvider);
    if (nodeRunning is! AsyncData || nodeRunning.value != true) {
      return null;
    }
    return rust.getIdentity();
  }

  Future<void> createIdentity(String name, {String? avatarPath}) async {
    final identity =
        rust.createIdentity(name: name, avatarPath: avatarPath);
    state = AsyncData(identity);
  }
}

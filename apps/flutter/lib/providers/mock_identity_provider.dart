import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'mock_data.dart';

/// Mock identity provider — returns static fake identity.
/// Replaces the FFI-dependent identityProvider for mockup mode.
/// After onboarding, set to mockIdentity.
/// Screens check this to determine onboarding vs home.
final mockIdentityProvider =
    NotifierProvider<MockIdentityNotifier, MockIdentity?>(
        MockIdentityNotifier.new);

class MockIdentityNotifier extends Notifier<MockIdentity?> {
  @override
  MockIdentity? build() => null;

  void set(MockIdentity identity) => state = identity;
}

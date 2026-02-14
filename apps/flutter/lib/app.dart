import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'providers/node_provider.dart';
import 'screens/onboarding/welcome_screen.dart';
import 'screens/home_screen.dart';
import 'providers/identity_provider.dart';

class TesserasApp extends ConsumerWidget {
  const TesserasApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return MaterialApp(
      title: 'Tesseras',
      theme: ThemeData(
        colorSchemeSeed: const Color(0xFF2D5016),
        useMaterial3: true,
      ),
      home: ref.watch(nodeProvider).when(
            loading: () => const Scaffold(
              body: Center(child: CircularProgressIndicator()),
            ),
            error: (e, _) => Scaffold(
              body: Center(child: Text('Failed to start node: $e')),
            ),
            data: (_) => const _HomeOrOnboarding(),
          ),
    );
  }
}

class _HomeOrOnboarding extends ConsumerWidget {
  const _HomeOrOnboarding();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return ref.watch(identityProvider).when(
          loading: () => const Scaffold(
            body: Center(child: CircularProgressIndicator()),
          ),
          error: (e, _) => Scaffold(
            body: Center(child: Text('Error: $e')),
          ),
          data: (identity) {
            if (identity == null) {
              return const WelcomeScreen();
            }
            return const HomeScreen();
          },
        );
  }
}

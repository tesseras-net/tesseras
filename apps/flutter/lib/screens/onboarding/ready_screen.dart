import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/identity_provider.dart';
import '../home_screen.dart';

class ReadyScreen extends ConsumerWidget {
  const ReadyScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final identity = ref.watch(identityProvider);

    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.check_circle_outline, size: 96, color: Colors.green),
              const SizedBox(height: 24),
              Text(
                'Welcome, ${identity.value?.name ?? ""}!',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 16),
              const Text(
                'Your cryptographic identity has been created.\n'
                'You can now start preserving memories.',
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 48),
              FilledButton.icon(
                onPressed: () {
                  Navigator.of(context).pushAndRemoveUntil(
                    MaterialPageRoute(builder: (_) => const HomeScreen()),
                    (route) => false,
                  );
                },
                icon: const Icon(Icons.arrow_forward),
                label: const Text('Start Preserving'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

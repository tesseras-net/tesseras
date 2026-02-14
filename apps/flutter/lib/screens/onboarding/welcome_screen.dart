import 'package:flutter/material.dart';
import 'identity_screen.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              const Icon(Icons.photo_library_outlined, size: 96),
              const SizedBox(height: 24),
              Text(
                'Tesseras',
                style: Theme.of(context).textTheme.headlineLarge,
              ),
              const SizedBox(height: 16),
              Text(
                'Preserve your memories across millennia.\n'
                'No cloud. No company. Just you and the network.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              const SizedBox(height: 48),
              FilledButton.icon(
                onPressed: () {
                  Navigator.of(context).push(
                    MaterialPageRoute(
                      builder: (_) => const IdentityScreen(),
                    ),
                  );
                },
                icon: const Icon(Icons.arrow_forward),
                label: const Text('Get Started'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

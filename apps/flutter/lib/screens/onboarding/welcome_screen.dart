import 'package:flutter/material.dart';

class WelcomeScreen extends StatelessWidget {
  final VoidCallback onNext;

  const WelcomeScreen({super.key, required this.onNext});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // Gradient circle with "T" letter (placeholder logo)
              Container(
                width: 96,
                height: 96,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  gradient: LinearGradient(
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                    colors: [
                      colorScheme.primary,
                      colorScheme.tertiary,
                    ],
                  ),
                ),
                child: const Center(
                  child: Text(
                    'T',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 48,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              Text(
                'Tesseras',
                style: Theme.of(context).textTheme.displaySmall?.copyWith(
                      fontWeight: FontWeight.bold,
                      color: colorScheme.primary,
                    ),
              ),
              const SizedBox(height: 12),
              Text(
                'Preserve your memories across millennia.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 16),
              Text(
                'A peer-to-peer network where your photos, audio, and stories '
                'are preserved forever \u2014 no company, no cloud, no single '
                'point of failure.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
              const SizedBox(height: 48),
              FilledButton.icon(
                onPressed: onNext,
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

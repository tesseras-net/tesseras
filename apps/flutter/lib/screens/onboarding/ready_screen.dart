import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/mock_identity_provider.dart';
import '../../widgets/copy_button.dart';
import '../desktop_shell.dart';

class ReadyScreen extends ConsumerStatefulWidget {
  const ReadyScreen({super.key});

  @override
  ConsumerState<ReadyScreen> createState() => _ReadyScreenState();
}

class _ReadyScreenState extends ConsumerState<ReadyScreen>
    with SingleTickerProviderStateMixin {
  late final AnimationController _scaleController;
  late final Animation<double> _scaleAnimation;

  @override
  void initState() {
    super.initState();
    _scaleController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 400),
    );
    _scaleAnimation = CurvedAnimation(
      parent: _scaleController,
      curve: Curves.elasticOut,
    );
    _scaleController.forward();
  }

  @override
  void dispose() {
    _scaleController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final identity = ref.watch(mockIdentityProvider);
    final name = identity?.name ?? 'User';
    final nodeId = identity?.nodeIdHex ?? '';

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // Animated checkmark
              ScaleTransition(
                scale: _scaleAnimation,
                child: const Icon(
                  Icons.check_circle,
                  size: 96,
                  color: Colors.green,
                ),
              ),
              const SizedBox(height: 24),
              Text(
                'Welcome, $name!',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 16),
              // Node ID
              if (nodeId.isNotEmpty)
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(
                      'Node: ${nodeId.substring(0, 16)}...',
                      style: Theme.of(context)
                          .textTheme
                          .bodySmall
                          ?.copyWith(fontFamily: 'monospace'),
                    ),
                    CopyButton(text: nodeId, tooltip: 'Copy Node ID'),
                  ],
                ),
              const SizedBox(height: 16),
              Text(
                'Your keys have been generated. Keep your device safe '
                '\u2014 it holds your identity.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
              ),
              const SizedBox(height: 48),
              FilledButton.icon(
                onPressed: () {
                  Navigator.of(context).pushAndRemoveUntil(
                    MaterialPageRoute(
                        builder: (_) => const DesktopShell()),
                    (route) => false,
                  );
                },
                icon: const Icon(Icons.arrow_forward),
                label: const Text('Open Tesseras'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

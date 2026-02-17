import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../providers/identity_provider.dart';
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
    final identityAsync = ref.watch(identityProvider);
    final identity = identityAsync.value;
    final name = identity?.name ?? 'User';
    final nodeId = identity?.nodeIdHex ?? '';
    final l = AppLocalizations.of(context);

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
                  color: Color(0xFF4CAF50),
                ),
              ),
              const SizedBox(height: 24),
              Text(l.readyWelcome(name)).h3.semiBold,
              const SizedBox(height: 16),
              // Node ID
              if (nodeId.isNotEmpty)
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    Text(l.readyNodePrefix(nodeId.substring(0, 16)))
                        .small.mono.muted,
                    CopyButton(text: nodeId, tooltip: l.readyCopyNodeId),
                  ],
                ),
              const SizedBox(height: 16),
              Text(l.readyKeysNote, textAlign: TextAlign.center).base.muted,
              const SizedBox(height: 48),
              PrimaryButton(
                onPressed: () {
                  Navigator.of(context).pushAndRemoveUntil(
                    MaterialPageRoute(
                        builder: (_) => const DesktopShell()),
                    (route) => false,
                  );
                },
                trailing: const Icon(Icons.arrow_forward, size: 16),
                child: Text(l.readyOpenButton),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

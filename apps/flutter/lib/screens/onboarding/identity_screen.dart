import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/mock_data.dart';
import '../../providers/mock_identity_provider.dart';

class IdentityScreen extends ConsumerStatefulWidget {
  final VoidCallback onNext;
  final VoidCallback onBack;

  const IdentityScreen({
    super.key,
    required this.onNext,
    required this.onBack,
  });

  @override
  ConsumerState<IdentityScreen> createState() => _IdentityScreenState();
}

class _IdentityScreenState extends ConsumerState<IdentityScreen> {
  final _nameController = TextEditingController();
  int _colorIndex = 0;

  static const _avatarColors = [
    Colors.indigo,
    Colors.teal,
    Colors.orange,
    Colors.pink,
  ];

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  void _continue() {
    final name = _nameController.text.trim();
    if (name.isEmpty) return;

    // Set mock identity with entered name
    ref.read(mockIdentityProvider.notifier).state = MockIdentity(
      name: name,
      nodeIdHex: mockIdentity.nodeIdHex,
      ed25519PublicKeyHex: mockIdentity.ed25519PublicKeyHex,
      mldsaPublicKeyHex: mockIdentity.mldsaPublicKeyHex,
      createdAt: mockIdentity.createdAt,
      avatarColor: _avatarColors[_colorIndex],
    );

    widget.onNext();
  }

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
              Text(
                'Create your identity',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 32),
              // Avatar picker
              GestureDetector(
                onTap: () {
                  setState(() {
                    _colorIndex = (_colorIndex + 1) % _avatarColors.length;
                  });
                },
                child: CircleAvatar(
                  radius: 48,
                  backgroundColor: _avatarColors[_colorIndex],
                  child: const Icon(Icons.add_a_photo,
                      size: 32, color: Colors.white),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                'Tap to change color',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
              const SizedBox(height: 24),
              // Name field
              TextField(
                controller: _nameController,
                autofocus: true,
                decoration: const InputDecoration(
                  labelText: 'Your name',
                  border: OutlineInputBorder(),
                ),
                textCapitalization: TextCapitalization.words,
                onSubmitted: (_) => _continue(),
              ),
              const SizedBox(height: 16),
              Text(
                'Your identity is secured with cryptographic keys '
                'generated automatically.',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
              const SizedBox(height: 32),
              // Back / Continue
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  OutlinedButton(
                    onPressed: widget.onBack,
                    child: const Text('Back'),
                  ),
                  const SizedBox(width: 12),
                  FilledButton(
                    onPressed: _continue,
                    child: const Text('Continue'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

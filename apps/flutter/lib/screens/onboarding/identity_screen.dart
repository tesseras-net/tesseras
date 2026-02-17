import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../providers/identity_provider.dart';
import '../../widgets/camera_capture_dialog.dart';

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
  String? _avatarImagePath;

  static const _avatarColors = [
    Color(0xFF5C6BC0), // indigo
    Color(0xFF009688), // teal
    Color(0xFFFF9800), // orange
    Color(0xFFE91E63), // pink
  ];

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  Future<void> _continue() async {
    final name = _nameController.text.trim();
    stderr.writeln('[identity] _continue called, name="$name"');
    if (name.isEmpty) return;

    try {
      stderr.writeln('[identity] calling createIdentity...');
      await ref.read(identityProvider.notifier).createIdentity(
            name,
            avatarPath: _avatarImagePath,
          );
      stderr.writeln('[identity] createIdentity succeeded, calling onNext');
      if (mounted) {
        widget.onNext();
      }
    } catch (e, st) {
      stderr.writeln('[identity] ERROR: $e');
      stderr.writeln(st.toString());
      if (mounted) {
        showToast(
          context: context,
          builder: (context, overlay) => SurfaceCard(
            child: Basic(
              title: Text('Error creating identity: $e'),
              trailing: GhostButton(
                density: ButtonDensity.icon,
                onPressed: overlay.close,
                child: const Icon(Icons.close, size: 16),
              ),
            ),
          ),
          showDuration: const Duration(seconds: 5),
        );
      }
    }
  }

  void _showAvatarPicker() {
    final l = AppLocalizations.of(context);

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(l.identityAvatarChoose),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            GhostButton(
              onPressed: () {
                Navigator.pop(ctx);
                _captureFromCamera();
              },
              leading: const Icon(Icons.camera_alt, size: 16),
              alignment: Alignment.centerLeft,
              child: Text(l.identityAvatarCamera),
            ),
            GhostButton(
              onPressed: () {
                Navigator.pop(ctx);
                _pickImage(ImageSource.gallery);
              },
              leading: const Icon(Icons.photo_library, size: 16),
              alignment: Alignment.centerLeft,
              child: Text(l.identityAvatarGallery),
            ),
            GhostButton(
              onPressed: () {
                Navigator.pop(ctx);
                setState(() {
                  _avatarImagePath = null;
                  _colorIndex = (_colorIndex + 1) % _avatarColors.length;
                });
              },
              leading: const Icon(Icons.palette, size: 16),
              alignment: Alignment.centerLeft,
              child: Text(l.identityAvatarColor),
            ),
            if (_avatarImagePath != null)
              GhostButton(
                onPressed: () {
                  Navigator.pop(ctx);
                  setState(() {
                    _avatarImagePath = null;
                  });
                },
                leading: const Icon(Icons.delete_outline, size: 16),
                alignment: Alignment.centerLeft,
                child: Text(l.identityAvatarRemove),
              ),
          ],
        ),
      ),
    );
  }

  Future<void> _captureFromCamera() async {
    final path = await CameraCaptureDialog.show(context);
    if (path != null && mounted) {
      setState(() {
        _avatarImagePath = path;
      });
    }
  }

  Future<void> _pickImage(ImageSource source) async {
    try {
      final picker = ImagePicker();
      final picked = await picker.pickImage(source: source);
      if (picked != null) {
        setState(() {
          _avatarImagePath = picked.path;
        });
      }
    } catch (_) {}
  }

  Future<void> _importIdentity() async {
    final l = AppLocalizations.of(context);
    try {
      final result = await FilePicker.platform.pickFiles(type: FileType.any);
      if (result == null || result.files.isEmpty) return;
    } catch (_) {
      return;
    }

    // TODO: Implement real identity import from file
    // For now, create a new identity as placeholder
    try {
      await ref.read(identityProvider.notifier).createIdentity('Imported User');
    } catch (_) {
      return;
    }

    if (mounted) {
      showToast(
        context: context,
        builder: (context, overlay) => SurfaceCard(
          child: Basic(
            title: Text(l.identityImported),
            trailing: GhostButton(
              density: ButtonDensity.icon,
              onPressed: overlay.close,
              child: const Icon(Icons.close, size: 16),
            ),
          ),
        ),
        showDuration: const Duration(seconds: 3),
      );
      widget.onNext();
    }
  }

  Widget _buildAvatar() {
    if (_avatarImagePath != null) {
      return ClipOval(
        child: SizedBox(
          width: 96,
          height: 96,
          child: Image.file(File(_avatarImagePath!), fit: BoxFit.cover),
        ),
      );
    }
    return Container(
      width: 96,
      height: 96,
      decoration: BoxDecoration(
        color: _avatarColors[_colorIndex],
        shape: BoxShape.circle,
      ),
      child: const Icon(Icons.add_a_photo, size: 32, color: Colors.white),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(l.identityHeadline).h3.semiBold,
              const SizedBox(height: 32),
              // Avatar picker
              GestureDetector(
                onTap: _showAvatarPicker,
                child: _buildAvatar(),
              ),
              const SizedBox(height: 8),
              Text(l.identityAvatarChoose).muted.small,
              const SizedBox(height: 24),
              // Name field
              TextField(
                controller: _nameController,
                placeholder: Text(l.identityNameLabel),
                onSubmitted: (_) => _continue(),
              ),
              const SizedBox(height: 16),
              Text(l.identityKeysNote, textAlign: TextAlign.center).small.muted,
              const SizedBox(height: 32),
              // Back / Continue
              Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  OutlineButton(
                    onPressed: widget.onBack,
                    child: Text(l.identityBack),
                  ),
                  const SizedBox(width: 12),
                  PrimaryButton(
                    onPressed: _continue,
                    child: Text(l.identityContinue),
                  ),
                ],
              ),
              const SizedBox(height: 24),
              // Divider + Import
              Row(
                children: [
                  const Expanded(child: Divider()),
                  Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 12),
                    child: Text(l.identityOrImport).muted.small,
                  ),
                  const Expanded(child: Divider()),
                ],
              ),
              const SizedBox(height: 16),
              OutlineButton(
                onPressed: _importIdentity,
                leading: const Icon(Icons.file_open, size: 16),
                child: Text(l.identityImportButton),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

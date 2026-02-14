import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import '../../src/rust/api/simple.dart' as rust;

class CreateMemoryScreen extends ConsumerStatefulWidget {
  const CreateMemoryScreen({super.key});

  @override
  ConsumerState<CreateMemoryScreen> createState() =>
      _CreateMemoryScreenState();
}

class _CreateMemoryScreenState extends ConsumerState<CreateMemoryScreen> {
  final _contextController = TextEditingController();
  final _tagsController = TextEditingController();
  String? _mediaPath;
  String _memoryType = 'moment';
  String _visibility = 'private';
  bool _creating = false;

  @override
  void dispose() {
    _contextController.dispose();
    _tagsController.dispose();
    super.dispose();
  }

  Future<void> _pickImage() async {
    final picker = ImagePicker();
    final image = await picker.pickImage(source: ImageSource.gallery);
    if (image != null) {
      setState(() => _mediaPath = image.path);
    }
  }

  Future<void> _takePhoto() async {
    final picker = ImagePicker();
    final image = await picker.pickImage(source: ImageSource.camera);
    if (image != null) {
      setState(() => _mediaPath = image.path);
    }
  }

  Future<void> _createMemory() async {
    if (_mediaPath == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Please select a photo first')),
      );
      return;
    }

    setState(() => _creating = true);
    try {
      final tags = _tagsController.text
          .split(',')
          .map((t) => t.trim())
          .where((t) => t.isNotEmpty)
          .toList();

      rust.createMemory(
        mediaPath: _mediaPath!,
        contextText: _contextController.text.isNotEmpty
            ? _contextController.text
            : null,
        memoryType: _memoryType,
        visibility: _visibility,
        tags: tags,
        people: [],
      );

      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e')),
        );
      }
    } finally {
      if (mounted) setState(() => _creating = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('New Memory')),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Media preview / picker
            GestureDetector(
              onTap: _pickImage,
              child: Container(
                height: 200,
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: _mediaPath != null
                    ? ClipRRect(
                        borderRadius: BorderRadius.circular(12),
                        child: Image.file(
                          File(_mediaPath!),
                          fit: BoxFit.cover,
                          width: double.infinity,
                        ),
                      )
                    : const Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(Icons.add_photo_alternate, size: 48),
                          SizedBox(height: 8),
                          Text('Tap to select a photo'),
                        ],
                      ),
              ),
            ),
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                TextButton.icon(
                  onPressed: _pickImage,
                  icon: const Icon(Icons.photo_library),
                  label: const Text('Gallery'),
                ),
                TextButton.icon(
                  onPressed: _takePhoto,
                  icon: const Icon(Icons.camera_alt),
                  label: const Text('Camera'),
                ),
              ],
            ),
            const SizedBox(height: 16),

            // Context
            TextField(
              controller: _contextController,
              decoration: const InputDecoration(
                labelText: 'Context (optional)',
                hintText: 'What is this memory about?',
                border: OutlineInputBorder(),
              ),
              maxLines: 3,
            ),
            const SizedBox(height: 16),

            // Memory type
            DropdownButtonFormField<String>(
              initialValue: _memoryType,
              decoration: const InputDecoration(
                labelText: 'Memory Type',
                border: OutlineInputBorder(),
              ),
              items: const [
                DropdownMenuItem(value: 'moment', child: Text('Moment')),
                DropdownMenuItem(
                    value: 'reflection', child: Text('Reflection')),
                DropdownMenuItem(value: 'daily', child: Text('Daily')),
                DropdownMenuItem(value: 'relation', child: Text('Relation')),
                DropdownMenuItem(value: 'object', child: Text('Object')),
              ],
              onChanged: (v) => setState(() => _memoryType = v!),
            ),
            const SizedBox(height: 16),

            // Visibility
            DropdownButtonFormField<String>(
              initialValue: _visibility,
              decoration: const InputDecoration(
                labelText: 'Visibility',
                border: OutlineInputBorder(),
              ),
              items: const [
                DropdownMenuItem(value: 'private', child: Text('Private')),
                DropdownMenuItem(value: 'circle', child: Text('Circle')),
                DropdownMenuItem(value: 'public', child: Text('Public')),
              ],
              onChanged: (v) => setState(() => _visibility = v!),
            ),
            const SizedBox(height: 16),

            // Tags
            TextField(
              controller: _tagsController,
              decoration: const InputDecoration(
                labelText: 'Tags (comma separated)',
                hintText: 'family, vacation, 2026',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 24),

            // Submit
            FilledButton(
              onPressed: _creating ? null : _createMemory,
              child: _creating
                  ? const SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Text('Create Memory'),
            ),
          ],
        ),
      ),
    );
  }
}

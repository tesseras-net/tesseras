import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/memory.dart';
import '../../models/memory_type.dart';
import '../../models/visibility.dart' as v;
import '../../providers/mock_timeline_provider.dart';

class CreateMemoryDialog extends ConsumerStatefulWidget {
  const CreateMemoryDialog({super.key});

  @override
  ConsumerState<CreateMemoryDialog> createState() =>
      _CreateMemoryDialogState();
}

class _CreateMemoryDialogState extends ConsumerState<CreateMemoryDialog> {
  bool _fileSelected = false;
  String _selectedFileName = '';
  String _selectedMediaType = 'jpeg';
  bool _creating = false;

  final _contextController = TextEditingController();
  final _tagsController = TextEditingController();
  final _locationController = TextEditingController();
  final _peopleController = TextEditingController();

  MemoryType _memoryType = MemoryType.moment;
  v.Visibility _visibility = v.Visibility.private;
  String _language = 'en';

  // Sealed-specific
  DateTime? _sealedOpenAfter;
  // PublicAfterDeath-specific
  final _inactiveYearsController = TextEditingController(text: '5');

  @override
  void dispose() {
    _contextController.dispose();
    _tagsController.dispose();
    _locationController.dispose();
    _peopleController.dispose();
    _inactiveYearsController.dispose();
    super.dispose();
  }

  void _mockSelectFile() {
    setState(() {
      _fileSelected = true;
      _selectedFileName = 'photo_${DateTime.now().millisecondsSinceEpoch}.jpg';
      _selectedMediaType = 'jpeg';
    });
  }

  void _removeFile() {
    setState(() {
      _fileSelected = false;
      _selectedFileName = '';
    });
  }

  Future<void> _createMemory() async {
    setState(() => _creating = true);

    // Mock delay
    await Future.delayed(const Duration(milliseconds: 500));

    final hash = List.generate(
        64, (_) => Random().nextInt(16).toRadixString(16)).join();

    final tags = _tagsController.text
        .split(',')
        .map((t) => t.trim())
        .where((t) => t.isNotEmpty)
        .toList();

    final people = _peopleController.text
        .split('\n')
        .map((p) => p.trim())
        .where((p) => p.isNotEmpty)
        .toList();

    final memory = Memory(
      hash: hash,
      tesseraHash: 'a3f8b2c1e7d49f6a2b1c3d4e5f6a7b8c9d0e1f23',
      type: _memoryType,
      visibility: _visibility,
      context:
          _contextController.text.isNotEmpty ? _contextController.text : null,
      createdAt: DateTime.now().toIso8601String(),
      tags: tags,
      location: _locationController.text.isNotEmpty
          ? _locationController.text
          : null,
      people: people,
      language: _language,
      mediaType: _selectedMediaType,
      sealedOpenAfter:
          _visibility == v.Visibility.sealed_ ? _sealedOpenAfter : null,
      publicAfterDeathYears: _visibility == v.Visibility.publicAfterDeath
          ? int.tryParse(_inactiveYearsController.text)
          : null,
    );

    ref.read(mockTimelineProvider.notifier).addMemory(memory);

    if (mounted) {
      Navigator.of(context).pop(true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Dialog(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500, maxHeight: 700),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // Header
              Row(
                children: [
                  Text('New Memory',
                      style: Theme.of(context).textTheme.titleLarge),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              // Scrollable content
              Flexible(
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      // File drop zone
                      if (!_fileSelected)
                        InkWell(
                          onTap: _mockSelectFile,
                          borderRadius: BorderRadius.circular(12),
                          child: Container(
                            height: 140,
                            decoration: BoxDecoration(
                              border: Border.all(
                                  color: colorScheme.outline, width: 1.5),
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: Column(
                              mainAxisAlignment: MainAxisAlignment.center,
                              children: [
                                Icon(Icons.cloud_upload_outlined,
                                    size: 36,
                                    color: colorScheme.onSurfaceVariant),
                                const SizedBox(height: 8),
                                const Text('Drop file here or click to browse'),
                                const SizedBox(height: 8),
                                FilledButton.tonal(
                                  onPressed: _mockSelectFile,
                                  child: const Text('Browse files'),
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  'Supported: JPEG, PNG, WAV, WebM, TXT',
                                  style: Theme.of(context)
                                      .textTheme
                                      .bodySmall
                                      ?.copyWith(
                                          color:
                                              colorScheme.onSurfaceVariant),
                                ),
                              ],
                            ),
                          ),
                        )
                      else
                        Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: colorScheme.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Row(
                            children: [
                              Icon(Icons.insert_drive_file,
                                  color: colorScheme.primary),
                              const SizedBox(width: 8),
                              Expanded(child: Text(_selectedFileName)),
                              IconButton(
                                icon: const Icon(Icons.close, size: 18),
                                onPressed: _removeFile,
                              ),
                            ],
                          ),
                        ),
                      const SizedBox(height: 16),
                      // Context
                      TextField(
                        controller: _contextController,
                        maxLines: 3,
                        decoration: const InputDecoration(
                          labelText: 'Context (optional)',
                          hintText: 'What is this memory about?',
                          border: OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // Type + Visibility side by side
                      Row(
                        children: [
                          Expanded(
                            child: DropdownButtonFormField<MemoryType>(
                              initialValue: _memoryType,
                              decoration: const InputDecoration(
                                labelText: 'Type',
                                border: OutlineInputBorder(),
                              ),
                              items: MemoryType.values
                                  .map((t) => DropdownMenuItem(
                                      value: t, child: Text(t.label)))
                                  .toList(),
                              onChanged: (val) {
                                if (val != null) {
                                  setState(() => _memoryType = val);
                                }
                              },
                            ),
                          ),
                          const SizedBox(width: 12),
                          Expanded(
                            child: DropdownButtonFormField<v.Visibility>(
                              initialValue: _visibility,
                              decoration: const InputDecoration(
                                labelText: 'Visibility',
                                border: OutlineInputBorder(),
                              ),
                              items: v.Visibility.values
                                  .map((vis) => DropdownMenuItem(
                                      value: vis, child: Text(vis.label)))
                                  .toList(),
                              onChanged: (val) {
                                if (val != null) {
                                  setState(() => _visibility = val);
                                }
                              },
                            ),
                          ),
                        ],
                      ),
                      // Conditional fields
                      if (_visibility == v.Visibility.sealed_) ...[
                        const SizedBox(height: 12),
                        InputDatePickerFormField(
                          firstDate: DateTime.now(),
                          lastDate: DateTime(2100),
                          initialDate:
                              _sealedOpenAfter ?? DateTime(2030, 1, 1),
                          fieldLabelText: 'Open after date',
                          onDateSaved: (date) => _sealedOpenAfter = date,
                          onDateSubmitted: (date) =>
                              _sealedOpenAfter = date,
                        ),
                      ],
                      if (_visibility == v.Visibility.publicAfterDeath) ...[
                        const SizedBox(height: 12),
                        TextField(
                          controller: _inactiveYearsController,
                          keyboardType: TextInputType.number,
                          decoration: const InputDecoration(
                            labelText: 'Years of inactivity',
                            border: OutlineInputBorder(),
                          ),
                        ),
                      ],
                      if (_visibility == v.Visibility.circle) ...[
                        const SizedBox(height: 12),
                        Text(
                          'Circle members will be configurable in a future update.',
                          style: Theme.of(context)
                              .textTheme
                              .bodySmall
                              ?.copyWith(
                                  color: colorScheme.onSurfaceVariant),
                        ),
                      ],
                      const SizedBox(height: 16),
                      // Language
                      DropdownButtonFormField<String>(
                        initialValue: _language,
                        decoration: const InputDecoration(
                          labelText: 'Language',
                          border: OutlineInputBorder(),
                        ),
                        items: const [
                          DropdownMenuItem(value: 'en', child: Text('English')),
                          DropdownMenuItem(
                              value: 'pt', child: Text('Portuguese')),
                          DropdownMenuItem(value: 'es', child: Text('Spanish')),
                          DropdownMenuItem(value: 'fr', child: Text('French')),
                          DropdownMenuItem(value: 'de', child: Text('German')),
                          DropdownMenuItem(
                              value: 'ja', child: Text('Japanese')),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _language = val);
                        },
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
                      const SizedBox(height: 16),
                      // Location
                      TextField(
                        controller: _locationController,
                        decoration: const InputDecoration(
                          labelText: 'Location (optional)',
                          hintText: 'Paraty, RJ',
                          border: OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // People
                      TextField(
                        controller: _peopleController,
                        maxLines: 3,
                        decoration: const InputDecoration(
                          labelText: 'People (optional, one per line)',
                          hintText: 'Maria - spouse\nLucas - son',
                          border: OutlineInputBorder(),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
              // Actions
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(),
                    child: const Text('Cancel'),
                  ),
                  const SizedBox(width: 8),
                  FilledButton(
                    onPressed: _fileSelected && !_creating
                        ? _createMemory
                        : null,
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
            ],
          ),
        ),
      ),
    );
  }
}

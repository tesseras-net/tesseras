import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../l10n/app_localizations.dart';
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
  int _selectedFileCount = 1;
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
      _selectedFileCount = 1;
    });
  }

  void _mockSelectFolder() {
    setState(() {
      _fileSelected = true;
      _selectedFileName = 'vacation_2026/';
      _selectedMediaType = 'jpeg';
      _selectedFileCount = 7;
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
    final l = AppLocalizations.of(context);

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
                  Text(l.createMemoryTitle,
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
                                Text(l.createMemoryDropZone),
                                const SizedBox(height: 8),
                                Row(
                                  mainAxisAlignment: MainAxisAlignment.center,
                                  children: [
                                    FilledButton.tonal(
                                      onPressed: _mockSelectFile,
                                      child: Text(l.createMemoryBrowseFiles),
                                    ),
                                    const SizedBox(width: 8),
                                    OutlinedButton.icon(
                                      onPressed: _mockSelectFolder,
                                      icon: const Icon(Icons.folder_open,
                                          size: 18),
                                      label: Text(l.createMemoryOpenFolder),
                                    ),
                                  ],
                                ),
                                const SizedBox(height: 4),
                                Text(
                                  l.createMemorySupportedFormats,
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
                              Icon(
                                _selectedFileCount > 1
                                    ? Icons.folder
                                    : Icons.insert_drive_file,
                                color: colorScheme.primary,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Column(
                                  crossAxisAlignment:
                                      CrossAxisAlignment.start,
                                  children: [
                                    Text(_selectedFileName),
                                    if (_selectedFileCount > 1)
                                      Text(
                                        l.createMemoryFilesFound(
                                            _selectedFileCount),
                                        style: Theme.of(context)
                                            .textTheme
                                            .bodySmall
                                            ?.copyWith(
                                                color: colorScheme
                                                    .onSurfaceVariant),
                                      ),
                                  ],
                                ),
                              ),
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
                        decoration: InputDecoration(
                          labelText: l.createMemoryContextLabel,
                          hintText: l.createMemoryContextHint,
                          border: const OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // Type + Visibility side by side
                      Row(
                        children: [
                          Expanded(
                            child: DropdownButtonFormField<MemoryType>(
                              initialValue: _memoryType,
                              decoration: InputDecoration(
                                labelText: l.createMemoryTypeLabel,
                                border: const OutlineInputBorder(),
                              ),
                              items: MemoryType.values
                                  .map((t) => DropdownMenuItem(
                                      value: t, child: Text(t.label(l))))
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
                              decoration: InputDecoration(
                                labelText: l.createMemoryVisibilityLabel,
                                border: const OutlineInputBorder(),
                              ),
                              items: v.Visibility.values
                                  .map((vis) => DropdownMenuItem(
                                      value: vis, child: Text(vis.label(l))))
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
                          fieldLabelText: l.createMemoryOpenAfterDate,
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
                          decoration: InputDecoration(
                            labelText: l.createMemoryInactiveYears,
                            border: const OutlineInputBorder(),
                          ),
                        ),
                      ],
                      if (_visibility == v.Visibility.circle) ...[
                        const SizedBox(height: 12),
                        Text(
                          l.createMemoryCircleNote,
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
                        decoration: InputDecoration(
                          labelText: l.createMemoryLanguageLabel,
                          border: const OutlineInputBorder(),
                        ),
                        items: [
                          DropdownMenuItem(
                              value: 'en',
                              child: Text(l.createMemoryLangEnglish)),
                          DropdownMenuItem(
                              value: 'pt',
                              child: Text(l.createMemoryLangPortuguese)),
                          DropdownMenuItem(
                              value: 'es',
                              child: Text(l.createMemoryLangSpanish)),
                          DropdownMenuItem(
                              value: 'fr',
                              child: Text(l.createMemoryLangFrench)),
                          DropdownMenuItem(
                              value: 'de',
                              child: Text(l.createMemoryLangGerman)),
                          DropdownMenuItem(
                              value: 'ja',
                              child: Text(l.createMemoryLangJapanese)),
                        ],
                        onChanged: (val) {
                          if (val != null) setState(() => _language = val);
                        },
                      ),
                      const SizedBox(height: 16),
                      // Tags
                      TextField(
                        controller: _tagsController,
                        decoration: InputDecoration(
                          labelText: l.createMemoryTagsLabel,
                          hintText: l.createMemoryTagsHint,
                          border: const OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // Location
                      TextField(
                        controller: _locationController,
                        decoration: InputDecoration(
                          labelText: l.createMemoryLocationLabel,
                          hintText: l.createMemoryLocationHint,
                          border: const OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      // People
                      TextField(
                        controller: _peopleController,
                        maxLines: 3,
                        decoration: InputDecoration(
                          labelText: l.createMemoryPeopleLabel,
                          hintText: l.createMemoryPeopleHint,
                          border: const OutlineInputBorder(),
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
                    child: Text(l.createMemoryCancel),
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
                        : Text(l.createMemoryCreate),
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

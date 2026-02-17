import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';
import '../../models/memory_type.dart';
import '../../models/visibility.dart' as v;
import '../../providers/timeline_provider.dart';
import '../../src/rust/api/simple.dart' as rust;

class CreateMemoryDialog extends ConsumerStatefulWidget {
  const CreateMemoryDialog({super.key});

  @override
  ConsumerState<CreateMemoryDialog> createState() =>
      _CreateMemoryDialogState();
}

class _CreateMemoryDialogState extends ConsumerState<CreateMemoryDialog> {
  bool _fileSelected = false;
  String _selectedFileName = '';
  String _selectedFilePath = '';
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

  Future<void> _selectFile() async {
    stderr.writeln('[file_picker] _selectFile called');
    try {
      final result = await FilePicker.platform.pickFiles(
        type: FileType.custom,
        allowedExtensions: ['jpg', 'jpeg', 'png', 'wav', 'webm', 'txt'],
      );
      stderr.writeln('[file_picker] _selectFile result: $result');
      if (result != null && result.files.isNotEmpty) {
        final file = result.files.first;
        setState(() {
          _fileSelected = true;
          _selectedFileName = file.name;
          _selectedFilePath = file.path ?? '';
          _selectedMediaType = _guessMediaType(file.name);
          _selectedFileCount = 1;
        });
      }
    } catch (e, st) {
      stderr.writeln('[file_picker] _selectFile error: $e\n$st');
    }
  }

  Future<void> _selectFolder() async {
    stderr.writeln('[file_picker] _selectFolder called');
    try {
      final dirPath = await FilePicker.platform.getDirectoryPath();
      stderr.writeln('[file_picker] _selectFolder result: $dirPath');
      if (dirPath != null) {
        setState(() {
          _fileSelected = true;
          _selectedFileName = dirPath.split('/').last;
          _selectedFilePath = dirPath;
          _selectedMediaType = 'jpeg';
          _selectedFileCount = 1;
        });
      }
    } catch (e, st) {
      stderr.writeln('[file_picker] _selectFolder error: $e\n$st');
    }
  }

  String _guessMediaType(String name) {
    final lower = name.toLowerCase();
    if (lower.endsWith('.png')) return 'png';
    if (lower.endsWith('.wav')) return 'wav';
    if (lower.endsWith('.webm')) return 'webm';
    if (lower.endsWith('.txt')) return 'txt';
    return 'jpeg';
  }

  void _removeFile() {
    setState(() {
      _fileSelected = false;
      _selectedFileName = '';
      _selectedFilePath = '';
    });
  }

  Future<void> _createMemory() async {
    setState(() => _creating = true);

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

    final visibilityStr = switch (_visibility) {
      v.Visibility.private => 'private',
      v.Visibility.circle => 'circle',
      v.Visibility.public => 'public',
      v.Visibility.publicAfterDeath => 'publicafterdeath',
      v.Visibility.sealed_ => 'sealed',
    };

    try {
      rust.createMemory(
        mediaPath: _selectedFilePath,
        contextText: _contextController.text.isNotEmpty
            ? _contextController.text
            : null,
        memoryType: _memoryType.name,
        visibility: visibilityStr,
        locationDescription: _locationController.text.isNotEmpty
            ? _locationController.text
            : null,
        tags: tags,
        people: people,
        sealedOpenAfter: _visibility == v.Visibility.sealed_ &&
                _sealedOpenAfter != null
            ? _sealedOpenAfter!.toUtc().toIso8601String()
            : null,
        inactiveYears: _visibility == v.Visibility.publicAfterDeath
            ? int.tryParse(_inactiveYearsController.text)
            : null,
      );

      // Refresh the timeline to show the new memory
      ref.read(timelineProvider.notifier).refresh();

      if (mounted) {
        Navigator.of(context).pop(true);
      }
    } catch (e) {
      setState(() => _creating = false);
      if (mounted) {
        showToast(
          context: context,
          builder: (context, overlay) => SurfaceCard(
            child: Basic(
              title: Text('Error: $e'),
              trailing: Button(
                style: const ButtonStyle.ghost(density: ButtonDensity.icon),
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l = AppLocalizations.of(context);

    return AlertDialog(
      title: Text(l.createMemoryTitle),
      trailing: Button(
        style: const ButtonStyle.ghost(density: ButtonDensity.icon),
        onPressed: () => Navigator.of(context).pop(),
        child: const Icon(Icons.close, size: 16),
      ),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500, maxHeight: 550),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              // File drop zone
              if (!_fileSelected)
                GestureDetector(
                  onTap: _selectFile,
                  child: Container(
                    height: 140,
                    decoration: BoxDecoration(
                      border: Border.all(
                        color: theme.colorScheme.border,
                        width: 1.5,
                      ),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.cloud_upload_outlined,
                            size: 36,
                            color: theme.colorScheme.mutedForeground),
                        const SizedBox(height: 8),
                        Text(l.createMemoryDropZone),
                        const SizedBox(height: 8),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Button.primary(
                              onPressed: _selectFile,
                              child: Text(l.createMemoryBrowseFiles),
                            ),
                            const SizedBox(width: 8),
                            Button.outline(
                              onPressed: _selectFolder,
                              leading: const Icon(Icons.folder_open, size: 18),
                              child: Text(l.createMemoryOpenFolder),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Text(l.createMemorySupportedFormats).small.muted,
                      ],
                    ),
                  ),
                )
              else
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.muted,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Row(
                    children: [
                      Icon(
                        _selectedFileCount > 1
                            ? Icons.folder
                            : Icons.insert_drive_file,
                        color: theme.colorScheme.primary,
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(_selectedFileName),
                            if (_selectedFileCount > 1)
                              Text(l.createMemoryFilesFound(
                                      _selectedFileCount))
                                  .small
                                  .muted,
                          ],
                        ),
                      ),
                      Button(
                        style: const ButtonStyle.ghost(
                            density: ButtonDensity.icon),
                        onPressed: _removeFile,
                        child: const Icon(Icons.close, size: 18),
                      ),
                    ],
                  ),
                ),
              const SizedBox(height: 16),
              // Context
              TextField(
                controller: _contextController,
                maxLines: 3,
                placeholder: Text(l.createMemoryContextHint),
              ),
              const SizedBox(height: 8),
              Text(l.createMemoryContextLabel).small.muted,
              const SizedBox(height: 16),
              // Type + Visibility side by side
              Row(
                children: [
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(l.createMemoryTypeLabel).small.muted,
                        const SizedBox(height: 4),
                        Select<MemoryType>(
                          value: _memoryType,
                          onChanged: (val) {
                            if (val != null) setState(() => _memoryType = val);
                          },
                          itemBuilder: (context, value) =>
                              Text(value.label(l)),
                          popup: (context) => SelectPopup(
                            items: SelectItemList(
                              children: MemoryType.values
                                  .map((t) => SelectItemButton(
                                      value: t, child: Text(t.label(l))))
                                  .toList(),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(l.createMemoryVisibilityLabel).small.muted,
                        const SizedBox(height: 4),
                        Select<v.Visibility>(
                          value: _visibility,
                          onChanged: (val) {
                            if (val != null) setState(() => _visibility = val);
                          },
                          itemBuilder: (context, value) =>
                              Text(value.label(l)),
                          popup: (context) => SelectPopup(
                            items: SelectItemList(
                              children: v.Visibility.values
                                  .map((vis) => SelectItemButton(
                                      value: vis, child: Text(vis.label(l))))
                                  .toList(),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              // Conditional fields
              if (_visibility == v.Visibility.sealed_) ...[
                const SizedBox(height: 12),
                Text(l.createMemoryOpenAfterDate).small.muted,
                const SizedBox(height: 4),
                DatePicker(
                  value: _sealedOpenAfter,
                  onChanged: (date) => setState(() => _sealedOpenAfter = date),
                  placeholder: const Text('Select date'),
                ),
              ],
              if (_visibility == v.Visibility.publicAfterDeath) ...[
                const SizedBox(height: 12),
                Text(l.createMemoryInactiveYears).small.muted,
                const SizedBox(height: 4),
                TextField(
                  controller: _inactiveYearsController,
                  keyboardType: TextInputType.number,
                  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                ),
              ],
              if (_visibility == v.Visibility.circle) ...[
                const SizedBox(height: 12),
                Text(l.createMemoryCircleNote).small.muted,
              ],
              const SizedBox(height: 16),
              // Language
              Text(l.createMemoryLanguageLabel).small.muted,
              const SizedBox(height: 4),
              Select<String>(
                value: _language,
                onChanged: (val) {
                  if (val != null) setState(() => _language = val);
                },
                itemBuilder: (context, value) => Text(_langLabel(value, l)),
                popup: (context) => SelectPopup(
                  items: SelectItemList(
                    children: [
                      SelectItemButton(
                          value: 'en',
                          child: Text(l.createMemoryLangEnglish)),
                      SelectItemButton(
                          value: 'pt',
                          child: Text(l.createMemoryLangPortuguese)),
                      SelectItemButton(
                          value: 'es',
                          child: Text(l.createMemoryLangSpanish)),
                      SelectItemButton(
                          value: 'fr',
                          child: Text(l.createMemoryLangFrench)),
                      SelectItemButton(
                          value: 'de',
                          child: Text(l.createMemoryLangGerman)),
                      SelectItemButton(
                          value: 'ja',
                          child: Text(l.createMemoryLangJapanese)),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
              // Tags
              Text(l.createMemoryTagsLabel).small.muted,
              const SizedBox(height: 4),
              TextField(
                controller: _tagsController,
                placeholder: Text(l.createMemoryTagsHint),
              ),
              const SizedBox(height: 16),
              // Location
              Text(l.createMemoryLocationLabel).small.muted,
              const SizedBox(height: 4),
              TextField(
                controller: _locationController,
                placeholder: Text(l.createMemoryLocationHint),
              ),
              const SizedBox(height: 16),
              // People
              Text(l.createMemoryPeopleLabel).small.muted,
              const SizedBox(height: 4),
              TextField(
                controller: _peopleController,
                maxLines: 3,
                placeholder: Text(l.createMemoryPeopleHint),
              ),
            ],
          ),
        ),
      ),
      actions: [
        Button.outline(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l.createMemoryCancel),
        ),
        const SizedBox(width: 8),
        Button.primary(
          onPressed: _fileSelected && !_creating ? _createMemory : null,
          child: _creating
              ? const SizedBox(
                  width: 20,
                  height: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Text(l.createMemoryCreate),
        ),
      ],
    );
  }

  String _langLabel(String code, AppLocalizations l) => switch (code) {
        'en' => l.createMemoryLangEnglish,
        'pt' => l.createMemoryLangPortuguese,
        'es' => l.createMemoryLangSpanish,
        'fr' => l.createMemoryLangFrench,
        'de' => l.createMemoryLangGerman,
        'ja' => l.createMemoryLangJapanese,
        _ => code,
      };
}

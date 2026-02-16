import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

/// Captures a photo from the webcam with live preview.
///
/// On desktop, streams MJPEG frames from ffmpeg and displays them live.
/// User clicks a button to freeze and save the current frame.
class CameraCaptureDialog extends StatefulWidget {
  const CameraCaptureDialog({super.key});

  /// Shows the dialog and returns the captured image path, or null.
  static Future<String?> show(BuildContext context) async {
    if (kIsWeb) return null;
    if (Platform.isLinux || Platform.isMacOS || Platform.isWindows) {
      return _captureDesktop(context);
    }
    return null;
  }

  static Future<String?> _captureDesktop(BuildContext context) async {
    String inputFormat;
    String device;
    if (Platform.isLinux) {
      inputFormat = 'v4l2';
      device = '/dev/video0';
    } else if (Platform.isMacOS) {
      inputFormat = 'avfoundation';
      device = '0';
    } else {
      inputFormat = 'dshow';
      device = 'video=Integrated Camera';
    }

    if (!context.mounted) return null;

    return showDialog<String>(
      context: context,
      barrierDismissible: false,
      builder: (_) => _LiveCaptureDialog(
        inputFormat: inputFormat,
        device: device,
      ),
    );
  }

  @override
  State<CameraCaptureDialog> createState() => _CameraCaptureDialogState();
}

class _CameraCaptureDialogState extends State<CameraCaptureDialog> {
  @override
  Widget build(BuildContext context) => const SizedBox.shrink();
}

/// Finds the ffmpeg binary.
Future<String?> _findFfmpeg() async {
  for (final path in ['/usr/bin/ffmpeg', '/usr/local/bin/ffmpeg']) {
    if (await File(path).exists()) return path;
  }
  try {
    final result = await Process.run('/usr/bin/which', ['ffmpeg']);
    if (result.exitCode == 0) {
      final path = (result.stdout as String).trim();
      if (path.isNotEmpty) return path;
    }
  } catch (_) {}
  return null;
}

/// JPEG start-of-image marker.
const _jpegSOI = [0xFF, 0xD8];

/// JPEG end-of-image marker.
const _jpegEOI = [0xFF, 0xD9];

/// Dialog with live camera preview via ffmpeg MJPEG stream.
class _LiveCaptureDialog extends StatefulWidget {
  final String inputFormat;
  final String device;

  const _LiveCaptureDialog({
    required this.inputFormat,
    required this.device,
  });

  @override
  State<_LiveCaptureDialog> createState() => _LiveCaptureDialogState();
}

class _LiveCaptureDialogState extends State<_LiveCaptureDialog> {
  Process? _process;
  Uint8List? _currentFrame;
  Uint8List? _frozenFrame;
  String? _error;
  bool _frozen = false;

  @override
  void initState() {
    super.initState();
    _startStream();
  }

  Future<void> _startStream() async {
    try {
      final ffmpeg = await _findFfmpeg();
      if (ffmpeg == null) {
        if (mounted) setState(() => _error = 'ffmpeg not found');
        return;
      }

      debugPrint('Starting live preview with $ffmpeg');

      final process = await Process.start(ffmpeg, [
        '-f', widget.inputFormat,
        '-i', widget.device,
        '-f', 'image2pipe',
        '-c:v', 'mjpeg',
        '-r', '10',
        'pipe:1',
      ]);

      _process = process;

      // Parse JPEG frames from stdout
      final buffer = BytesBuilder(copy: false);

      process.stdout.listen(
        (data) {
          if (_frozen) return; // Don't process frames while frozen
          buffer.add(data);
          _extractFrames(buffer);
        },
        onError: (e) {
          debugPrint('ffmpeg stdout error: $e');
          if (mounted && _error == null) {
            setState(() => _error = e.toString());
          }
        },
        onDone: () {
          debugPrint('ffmpeg stream ended');
        },
      );

      // Log stderr for debugging
      final stderrBuffer = StringBuffer();
      process.stderr.listen((data) {
        stderrBuffer.write(String.fromCharCodes(data));
      }, onDone: () {
        final stderr = stderrBuffer.toString().trim();
        if (stderr.isNotEmpty) debugPrint('ffmpeg stderr:\n$stderr');
      });

      process.exitCode.then((code) {
        debugPrint('ffmpeg exited with code $code');
        if (mounted && _currentFrame == null && _error == null) {
          setState(() => _error = 'Camera stream ended (exit $code)');
        }
      });
    } catch (e) {
      debugPrint('Camera stream error: $e');
      if (mounted) setState(() => _error = e.toString());
    }
  }

  /// Extract complete JPEG frames from the buffer.
  void _extractFrames(BytesBuilder buffer) {
    final bytes = buffer.toBytes();
    buffer.clear();

    int searchFrom = 0;
    int lastFrameEnd = 0;

    while (searchFrom < bytes.length - 1) {
      // Find SOI marker
      final soiIndex = _findMarker(bytes, _jpegSOI, searchFrom);
      if (soiIndex == -1) break;

      // Find EOI marker after SOI
      final eoiIndex = _findMarker(bytes, _jpegEOI, soiIndex + 2);
      if (eoiIndex == -1) {
        // Incomplete frame — put remaining bytes back in buffer
        buffer.add(bytes.sublist(soiIndex));
        return;
      }

      // Complete frame found
      final frame = Uint8List.sublistView(bytes, soiIndex, eoiIndex + 2);
      if (mounted && !_frozen) {
        setState(() => _currentFrame = frame);
      }

      lastFrameEnd = eoiIndex + 2;
      searchFrom = lastFrameEnd;
    }

    // Put remaining bytes back
    if (lastFrameEnd < bytes.length) {
      buffer.add(bytes.sublist(lastFrameEnd));
    }
  }

  /// Find a 2-byte marker in bytes starting from offset.
  int _findMarker(Uint8List bytes, List<int> marker, int from) {
    for (var i = from; i < bytes.length - 1; i++) {
      if (bytes[i] == marker[0] && bytes[i + 1] == marker[1]) return i;
    }
    return -1;
  }

  /// Freeze the current frame and save it.
  Future<void> _capture() async {
    if (_currentFrame == null) return;
    setState(() {
      _frozen = true;
      _frozenFrame = _currentFrame;
    });
  }

  /// Unfreeze and resume live preview.
  void _retake() {
    setState(() {
      _frozen = false;
      _frozenFrame = null;
    });
  }

  /// Save the frozen frame to a file and return the path.
  Future<void> _usePhoto() async {
    if (_frozenFrame == null) return;
    try {
      final tempDir = await getTemporaryDirectory();
      final filePath =
          '${tempDir.path}/tesseras_avatar_${DateTime.now().millisecondsSinceEpoch}.jpg';
      await File(filePath).writeAsBytes(_frozenFrame!);
      if (mounted) Navigator.of(context).pop(filePath);
    } catch (e) {
      debugPrint('Save error: $e');
    }
  }

  @override
  void dispose() {
    _process?.kill();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      clipBehavior: Clip.antiAlias,
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480, maxHeight: 520),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Flexible(
              child: AspectRatio(
                aspectRatio: 4 / 3,
                child: _buildPreview(),
              ),
            ),
            Padding(
              padding: const EdgeInsets.all(12),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(null),
                    child: const Text('Cancel'),
                  ),
                  if (!_frozen && _currentFrame != null)
                    FloatingActionButton(
                      onPressed: _capture,
                      child: const Icon(Icons.camera),
                    ),
                  if (_frozen && _frozenFrame != null) ...[
                    OutlinedButton.icon(
                      onPressed: _retake,
                      icon: const Icon(Icons.refresh),
                      label: const Text('Retake'),
                    ),
                    FilledButton.icon(
                      onPressed: _usePhoto,
                      icon: const Icon(Icons.check),
                      label: const Text('Use photo'),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPreview() {
    if (_error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Icons.error_outline, size: 48),
              const SizedBox(height: 8),
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: 8),
              const Text(
                'Make sure ffmpeg is installed and a webcam is connected.',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 12),
              ),
            ],
          ),
        ),
      );
    }

    final frame = _frozen ? _frozenFrame : _currentFrame;
    if (frame != null) {
      return Image.memory(
        frame,
        fit: BoxFit.cover,
        gaplessPlayback: true, // prevents flicker between frames
      );
    }

    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircularProgressIndicator(),
          SizedBox(height: 12),
          Text('Starting camera...'),
        ],
      ),
    );
  }
}

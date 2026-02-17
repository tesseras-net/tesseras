import 'dart:typed_data';

import 'package:shadcn_flutter/shadcn_flutter.dart';

import '../src/rust/api/simple.dart' as rust;

/// Shows the real image from blob storage when available, falling back to a
/// deterministic gradient placeholder with icon overlay.
class PlaceholderImage extends StatefulWidget {
  final String hash;
  final String mediaType;
  final String? mediaPath;
  final String? tesseraHash;
  final double? width;
  final double? height;

  const PlaceholderImage({
    super.key,
    required this.hash,
    this.mediaType = 'jpeg',
    this.mediaPath,
    this.tesseraHash,
    this.width,
    this.height,
  });

  @override
  State<PlaceholderImage> createState() => _PlaceholderImageState();
}

class _PlaceholderImageState extends State<PlaceholderImage> {
  Uint8List? _imageBytes;

  static const _palettes = <(Color, Color)>[
    (Color(0xFF1565C0), Color(0xFF42A5F5)), // blue
    (Color(0xFF2E7D32), Color(0xFF66BB6A)), // green
    (Color(0xFFE65100), Color(0xFFFF9800)), // orange
    (Color(0xFF6A1B9A), Color(0xFFAB47BC)), // purple
    (Color(0xFFC62828), Color(0xFFEF5350)), // red
    (Color(0xFF00695C), Color(0xFF26A69A)), // teal
    (Color(0xFFFF6F00), Color(0xFFFFCA28)), // amber
    (Color(0xFFAD1457), Color(0xFFEC407A)), // pink
  ];

  static const _icons = <IconData>[
    Icons.photo,
    Icons.landscape,
    Icons.camera_alt,
    Icons.image,
    Icons.panorama,
    Icons.filter_hdr,
    Icons.wb_sunny,
    Icons.nature,
  ];

  bool get _isImage {
    final mt = widget.mediaType;
    return mt == 'jpeg' || mt == 'png' || mt == 'jpg';
  }

  @override
  void initState() {
    super.initState();
    _tryLoadBlob();
  }

  @override
  void didUpdateWidget(PlaceholderImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.hash != widget.hash ||
        oldWidget.mediaPath != widget.mediaPath) {
      _imageBytes = null;
      _tryLoadBlob();
    }
  }

  /// Parse mediaPath (e.g. "memories/hash/media.jpg") and load via CAS.
  void _tryLoadBlob() {
    if (!_isImage) return;
    final mediaPath = widget.mediaPath;
    final tesseraHash = widget.tesseraHash ?? widget.hash;
    if (mediaPath == null || mediaPath.isEmpty) return;

    // mediaPath format: "memories/<memory_hash>/<filename>"
    final parts = mediaPath.split('/');
    if (parts.length < 3) return;
    final memoryHash = parts[1];
    final filename = parts[2];

    try {
      final bytes = rust.getMediaBlob(
        tesseraHash: tesseraHash,
        memoryHash: memoryHash,
        name: filename,
      );
      if (mounted) {
        setState(() => _imageBytes = bytes);
      }
    } catch (_) {
      // Blob not available — gradient placeholder will be shown.
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_isImage && _imageBytes != null) {
      return ClipRRect(
        borderRadius: BorderRadius.circular(8),
        child: Image.memory(
          _imageBytes!,
          width: widget.width,
          height: widget.height,
          fit: BoxFit.cover,
          errorBuilder: (_, __, ___) => _gradientPlaceholder(),
        ),
      );
    }
    return _gradientPlaceholder();
  }

  Widget _gradientPlaceholder() {
    final index = widget.hash.codeUnits.fold<int>(0, (a, b) => a + b);
    final paletteIndex = index % _palettes.length;
    final iconIndex = (index ~/ _palettes.length) % _icons.length;
    final (startColor, endColor) = _palettes[paletteIndex];

    final isText = widget.mediaType == 'txt';
    final isAudio =
        widget.mediaType == 'wav' || widget.mediaType == 'webm';

    return Container(
      width: widget.width,
      height: widget.height,
      decoration: BoxDecoration(
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [startColor, endColor],
        ),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Center(
        child: Icon(
          isText
              ? Icons.article
              : isAudio
                  ? Icons.audiotrack
                  : _icons[iconIndex],
          size: 48,
          color: const Color(0xFFFFFFFF).withValues(alpha: 0.6),
        ),
      ),
    );
  }
}

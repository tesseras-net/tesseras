import 'package:flutter/material.dart';

/// Deterministic gradient placeholder from hash string with icon overlay.
/// For mediaType == 'txt', shows a text document icon instead.
class PlaceholderImage extends StatelessWidget {
  final String hash;
  final String mediaType;
  final double? width;
  final double? height;

  const PlaceholderImage({
    super.key,
    required this.hash,
    this.mediaType = 'jpeg',
    this.width,
    this.height,
  });

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

  @override
  Widget build(BuildContext context) {
    final index = hash.codeUnits.fold<int>(0, (a, b) => a + b);
    final paletteIndex = index % _palettes.length;
    final iconIndex = (index ~/ _palettes.length) % _icons.length;
    final (startColor, endColor) = _palettes[paletteIndex];

    final isText = mediaType == 'txt';
    final isAudio = mediaType == 'wav' || mediaType == 'webm';

    return Container(
      width: width,
      height: height,
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
          color: Colors.white.withValues(alpha: 0.6),
        ),
      ),
    );
  }
}

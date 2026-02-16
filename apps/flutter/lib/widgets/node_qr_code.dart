import 'package:flutter/material.dart';

/// Simple deterministic grid pattern generated from a hex string.
/// Not a real QR code, but visually represents the node identity
/// in a scannable-looking format. Replace with qr_flutter when
/// adding real peer connection flow.
class NodeQrCode extends StatelessWidget {
  final String hexData;
  final double size;

  const NodeQrCode({super.key, required this.hexData, this.size = 140});

  @override
  Widget build(BuildContext context) {
    final isDark = Theme.of(context).brightness == Brightness.dark;
    final fg = isDark ? Colors.white : Colors.black;
    final bg = isDark ? Colors.grey.shade900 : Colors.white;

    // Generate a 9x9 grid from hex chars
    const gridSize = 9;
    final cells = <bool>[];
    for (var i = 0; i < gridSize * gridSize; i++) {
      if (i < hexData.length) {
        final charCode = hexData.codeUnitAt(i % hexData.length);
        cells.add(charCode % 2 == 0);
      } else {
        cells.add(i % 3 == 0);
      }
    }

    // Mirror horizontally for QR-like symmetry
    for (var row = 0; row < gridSize; row++) {
      for (var col = 0; col < gridSize ~/ 2; col++) {
        cells[row * gridSize + (gridSize - 1 - col)] =
            cells[row * gridSize + col];
      }
    }

    // Force corner squares (QR finder patterns)
    for (var r = 0; r < 3; r++) {
      for (var c = 0; c < 3; c++) {
        cells[r * gridSize + c] = (r == 1 && c == 1) ? true : true;
        cells[r * gridSize + (gridSize - 1 - c)] = true;
        cells[(gridSize - 1 - r) * gridSize + c] = true;
      }
    }
    // Center of corner patterns
    cells[1 * gridSize + 1] = false;
    cells[1 * gridSize + (gridSize - 2)] = false;
    cells[(gridSize - 2) * gridSize + 1] = false;

    return Container(
      width: size,
      height: size,
      padding: const EdgeInsets.all(8),
      decoration: BoxDecoration(
        color: bg,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Theme.of(context).dividerColor),
      ),
      child: CustomPaint(
        size: Size(size - 16, size - 16),
        painter: _QrPainter(
          cells: cells,
          gridSize: gridSize,
          cellSize: (size - 16) / gridSize,
          color: fg,
        ),
      ),
    );
  }
}

class _QrPainter extends CustomPainter {
  final List<bool> cells;
  final int gridSize;
  final double cellSize;
  final Color color;

  _QrPainter({
    required this.cells,
    required this.gridSize,
    required this.cellSize,
    required this.color,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()..color = color;

    for (var row = 0; row < gridSize; row++) {
      for (var col = 0; col < gridSize; col++) {
        if (cells[row * gridSize + col]) {
          canvas.drawRRect(
            RRect.fromRectAndRadius(
              Rect.fromLTWH(
                col * cellSize,
                row * cellSize,
                cellSize - 1,
                cellSize - 1,
              ),
              const Radius.circular(2),
            ),
            paint,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

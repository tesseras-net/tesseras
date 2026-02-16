import 'package:flutter/material.dart';

/// Progress bar widget for storage usage.
class StorageBar extends StatelessWidget {
  final int usedMB;
  final int totalMB;

  const StorageBar({super.key, required this.usedMB, required this.totalMB});

  @override
  Widget build(BuildContext context) {
    final percentage = totalMB > 0 ? usedMB / totalMB : 0.0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('$usedMB MB'),
            const Spacer(),
            Text('${totalMB ~/ 1024} GB'),
          ],
        ),
        const SizedBox(height: 8),
        LinearProgressIndicator(
          value: percentage,
          minHeight: 8,
          borderRadius: BorderRadius.circular(4),
        ),
      ],
    );
  }
}

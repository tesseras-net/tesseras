import 'package:shadcn_flutter/shadcn_flutter.dart';

/// Progress bar widget for storage usage.
class StorageBar extends StatelessWidget {
  final int usedMB;
  final int totalMB;

  const StorageBar({super.key, required this.usedMB, required this.totalMB});

  @override
  Widget build(BuildContext context) {
    final percentage = totalMB > 0 ? usedMB / totalMB : 0.0;
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text('$usedMB MB').small,
            const Spacer(),
            Text('${totalMB ~/ 1024} GB').small,
          ],
        ),
        const SizedBox(height: 8),
        Container(
          height: 8,
          decoration: BoxDecoration(
            color: theme.colorScheme.muted,
            borderRadius: BorderRadius.circular(4),
          ),
          child: FractionallySizedBox(
            alignment: Alignment.centerLeft,
            widthFactor: percentage.clamp(0.0, 1.0),
            child: Container(
              decoration: BoxDecoration(
                color: theme.colorScheme.primary,
                borderRadius: BorderRadius.circular(4),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

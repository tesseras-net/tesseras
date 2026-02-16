import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Click-to-copy button with tooltip feedback.
class CopyButton extends StatelessWidget {
  final String text;
  final String tooltip;

  const CopyButton({super.key, required this.text, this.tooltip = 'Copy'});

  @override
  Widget build(BuildContext context) {
    return IconButton(
      icon: const Icon(Icons.copy, size: 18),
      tooltip: tooltip,
      onPressed: () {
        Clipboard.setData(ClipboardData(text: text));
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Copied to clipboard'),
            duration: Duration(seconds: 1),
          ),
        );
      },
    );
  }
}

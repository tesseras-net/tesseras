import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../l10n/app_localizations.dart';

/// Click-to-copy button with tooltip feedback.
class CopyButton extends StatelessWidget {
  final String text;
  final String? tooltip;

  const CopyButton({super.key, required this.text, this.tooltip});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return IconButton(
      icon: const Icon(Icons.copy, size: 18),
      tooltip: tooltip ?? l.copyDefault,
      onPressed: () {
        Clipboard.setData(ClipboardData(text: text));
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(l.copiedToClipboard),
            duration: const Duration(seconds: 1),
          ),
        );
      },
    );
  }
}

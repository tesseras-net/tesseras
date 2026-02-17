import 'package:flutter/services.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../l10n/app_localizations.dart';

/// Click-to-copy button with toast feedback.
class CopyButton extends StatelessWidget {
  final String text;
  final String? tooltip;

  const CopyButton({super.key, required this.text, this.tooltip});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);
    return Button(
      style: const ButtonStyle.ghost(density: ButtonDensity.icon),
      onPressed: () {
        Clipboard.setData(ClipboardData(text: text));
        showToast(
          context: context,
          builder: (context, overlay) => SurfaceCard(
            child: Basic(
              title: Text(l.copiedToClipboard),
              trailing: Button(
                style: const ButtonStyle.ghost(density: ButtonDensity.icon),
                onPressed: overlay.close,
                child: const Icon(Icons.close, size: 16),
              ),
            ),
          ),
          showDuration: const Duration(seconds: 1),
        );
      },
      child: const Icon(Icons.copy, size: 18),
    );
  }
}

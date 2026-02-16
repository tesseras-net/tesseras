import 'package:shadcn_flutter/shadcn_flutter.dart';
import '../../l10n/app_localizations.dart';

class WelcomeScreen extends StatelessWidget {
  final VoidCallback onNext;

  const WelcomeScreen({super.key, required this.onNext});

  @override
  Widget build(BuildContext context) {
    final l = AppLocalizations.of(context);

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              // Logo
              ClipRRect(
                borderRadius: BorderRadius.circular(16),
                child: Image.asset('assets/logo.png', width: 120, height: 120),
              ),
              const SizedBox(height: 24),
              Text(l.welcomeTitle).h2.semiBold,
              const SizedBox(height: 12),
              Text(l.welcomeTagline, textAlign: TextAlign.center).large.muted,
              const SizedBox(height: 16),
              Text(l.welcomeBody, textAlign: TextAlign.center).base.muted,
              const SizedBox(height: 48),
              PrimaryButton(
                onPressed: onNext,
                trailing: const Icon(Icons.arrow_forward, size: 16),
                child: Text(l.welcomeGetStarted),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

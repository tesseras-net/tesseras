import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import 'l10n/app_localizations.dart';
import 'providers/identity_provider.dart';
import 'providers/locale_provider.dart';
import 'providers/theme_provider.dart';
import 'screens/onboarding/onboarding_flow.dart';
import 'screens/desktop_shell.dart';

class TesserasApp extends ConsumerWidget {
  const TesserasApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeProvider);
    final identityAsync = ref.watch(identityProvider);
    final localeOverride = ref.watch(localeProvider);

    // While loading, show nothing (node starting + identity fetch)
    final identity = identityAsync.value;

    return ShadcnApp(
      title: 'Tesseras',
      debugShowCheckedModeBanner: false,
      themeMode: themeMode,
      theme: ThemeData(
        colorScheme: ColorSchemes.lightZinc,
        radius: 0.5,
        scaling: 1,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorSchemes.darkZinc,
        radius: 0.5,
        scaling: 1,
      ),
      locale: localeOverride,
      supportedLocales: AppLocalizationsDelegate.supportedLocales,
      localizationsDelegates: const [
        AppLocalizationsDelegate(),
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      home: identity != null ? const DesktopShell() : const OnboardingFlow(),
    );
  }
}

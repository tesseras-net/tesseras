import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'providers/theme_provider.dart';
import 'providers/mock_identity_provider.dart';
import 'theme/light_theme.dart';
import 'theme/dark_theme.dart';
import 'screens/onboarding/onboarding_flow.dart';
import 'screens/desktop_shell.dart';

class TesserasApp extends ConsumerWidget {
  const TesserasApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeProvider);
    final identity = ref.watch(mockIdentityProvider);

    return MaterialApp(
      title: 'Tesseras',
      debugShowCheckedModeBanner: false,
      themeMode: themeMode,
      theme: lightTheme(),
      darkTheme: darkTheme(),
      home: identity != null ? const DesktopShell() : const OnboardingFlow(),
    );
  }
}

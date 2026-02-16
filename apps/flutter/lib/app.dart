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
      builder: (context, child) {
        // Scale up text slightly for better readability on high-DPI desktops.
        // MediaQuery.textScaleFactorOf defaults to 1.0 on Linux even with
        // HiDPI screens, making text feel too small.
        final mq = MediaQuery.of(context);
        final scale = mq.textScaler.scale(1.0);
        final adjustedScaler =
            scale < 1.1 ? TextScaler.linear(1.1) : mq.textScaler;

        return MediaQuery(
          data: mq.copyWith(textScaler: adjustedScaler),
          child: child!,
        );
      },
      home: identity != null ? const DesktopShell() : const OnboardingFlow(),
    );
  }
}

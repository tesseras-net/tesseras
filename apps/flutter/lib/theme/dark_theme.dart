import 'package:flutter/material.dart';

/// Dark theme based on design doc color palette.
/// Seed color: #5C6BC0 (indigo).
ThemeData darkTheme() {
  final colorScheme = ColorScheme.fromSeed(
    seedColor: const Color(0xFF5C6BC0),
    brightness: Brightness.dark,
  );

  return ThemeData(
    useMaterial3: true,
    colorScheme: colorScheme,
    scaffoldBackgroundColor: const Color(0xFF121212),
    cardTheme: const CardThemeData(
      elevation: 1,
      margin: EdgeInsets.zero,
    ),
  );
}

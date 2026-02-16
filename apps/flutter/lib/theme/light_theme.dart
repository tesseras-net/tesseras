import 'package:flutter/material.dart';

/// Light theme based on design doc color palette.
/// Seed color: #5C6BC0 (indigo).
ThemeData lightTheme() {
  final colorScheme = ColorScheme.fromSeed(
    seedColor: const Color(0xFF5C6BC0),
    brightness: Brightness.light,
  );

  return ThemeData(
    useMaterial3: true,
    colorScheme: colorScheme,
    scaffoldBackgroundColor: const Color(0xFFFAFAFA),
    cardTheme: const CardThemeData(
      elevation: 1,
      margin: EdgeInsets.zero,
    ),
  );
}

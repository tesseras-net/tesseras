import 'dart:ui';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// User locale override. `null` means follow the system locale.
final localeProvider =
    NotifierProvider<LocaleNotifier, Locale?>(LocaleNotifier.new);

class LocaleNotifier extends Notifier<Locale?> {
  @override
  Locale? build() => null;

  void setLocale(Locale? locale) => state = locale;
}

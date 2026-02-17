import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:shadcn_flutter/shadcn_flutter.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'app.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  // Log all Flutter errors to stderr
  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    stderr.writeln('[FlutterError] ${details.exceptionAsString()}');
    stderr.writeln(details.stack.toString());
  };

  // Log uncaught async errors
  PlatformDispatcher.instance.onError = (error, stack) {
    stderr.writeln('[UncaughtError] $error');
    stderr.writeln(stack.toString());
    return true;
  };

  stderr.writeln('[tesseras] app starting...');

  runApp(
    ProviderScope(
      observers: [_LogObserver()],
      child: const TesserasApp(),
    ),
  );
}

base class _LogObserver extends ProviderObserver {
  @override
  void didAddProvider(ProviderObserverContext context, Object? value) {
    stderr.writeln('[provider] added: ${context.provider.name ?? context.provider.runtimeType}');
  }

  @override
  void didUpdateProvider(
    ProviderObserverContext context,
    Object? previousValue,
    Object? newValue,
  ) {
    stderr.writeln(
        '[provider] updated: ${context.provider.name ?? context.provider.runtimeType} -> $newValue');
  }

  @override
  void providerDidFail(
    ProviderObserverContext context,
    Object error,
    StackTrace stackTrace,
  ) {
    stderr.writeln(
        '[provider] FAILED: ${context.provider.name ?? context.provider.runtimeType} error=$error');
    stderr.writeln(stackTrace.toString());
  }
}

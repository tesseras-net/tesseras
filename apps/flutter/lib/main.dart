import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  runApp(const ProviderScope(child: TesserasApp()));
}

class TesserasApp extends StatelessWidget {
  const TesserasApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Tesseras',
      theme: ThemeData(
        colorSchemeSeed: Colors.teal,
        useMaterial3: true,
      ),
      home: const Scaffold(
        body: Center(child: Text('Tesseras - Loading...')),
      ),
    );
  }
}

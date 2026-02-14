import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tesseras_app/app.dart';
import 'package:tesseras_app/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => await RustLib.init());
  testWidgets('App starts and shows loading', (WidgetTester tester) async {
    await tester.pumpWidget(const ProviderScope(child: TesserasApp()));
    // Node initialization shows a loading indicator initially.
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
  });
}

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:tesseras_app/app.dart';

void main() {
  testWidgets('App renders without crashing', (WidgetTester tester) async {
    await tester.pumpWidget(
      const ProviderScope(child: TesserasApp()),
    );
    expect(find.text('Tesseras'), findsAny);
  });
}

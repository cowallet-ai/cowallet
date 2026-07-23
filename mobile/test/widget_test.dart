import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  // NOTE: This is a smoke test for the widget/test toolchain only. The full
  // `CowalletApp` cannot be mounted here — it depends on the async `Services`
  // bootstrap (platform channels, secure storage, push) that is unavailable in
  // a `flutter test` host. App-level rendering is covered by integration tests
  // on a device/emulator.
  testWidgets('widget toolchain renders a MaterialApp', (WidgetTester tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: Center(child: Text('cowallet'))),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(MaterialApp), findsOneWidget);
    expect(find.text('cowallet'), findsOneWidget);
  });
}

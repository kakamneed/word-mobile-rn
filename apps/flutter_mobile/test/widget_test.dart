import 'package:flutter_test/flutter_test.dart';

import 'package:flutter_mobile/main.dart';

void main() {
  testWidgets('renders bootstrap shell instead of demo counter', (WidgetTester tester) async {
    await tester.pumpWidget(const MyApp());

    expect(find.text('Initializing Rust runtime...'), findsOneWidget);
    expect(find.text('Flutter Demo Home Page'), findsNothing);
  });
}

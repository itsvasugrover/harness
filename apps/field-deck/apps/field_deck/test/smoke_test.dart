// Field Deck smoke test: kit resolves, contract gate holds, kit widgets pump.
// Run with `flutter test` (headless on Linux, no device needed).
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:harness_ui/harness_ui.dart';

void main() {
  test('identity parses and newer contracts are detected', () {
    final id = HxIdentity.fromJson(
      const {'host_id': 'abc', 'contract': hxContract},
    );
    expect(id.hostId, 'abc');
    expect(id.contract, hxContract);
    final newer = HxIdentity.fromJson(
      const {'host_id': 'abc', 'contract': 999},
    );
    expect(newer.contract > hxContract, isTrue);
  });

  testWidgets('empty state pumps with title and hint', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: HxEmpty(
            icon: Icons.inbox,
            title: 'Nothing here',
            hint: 'Pair the daemon first',
          ),
        ),
      ),
    );
    expect(find.text('Nothing here'), findsOneWidget);
    expect(find.text('Pair the daemon first'), findsOneWidget);
  });
}

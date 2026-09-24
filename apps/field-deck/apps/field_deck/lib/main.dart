// Field Deck entry placeholder. Five tabs: board/worker/issues/audit/settings.
// Supervisor-only: no execution (see docs/field-deck-mobile.md).
import 'package:flutter/material.dart';

void main() => runApp(const FieldDeck());

class FieldDeck extends StatelessWidget {
  const FieldDeck({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      home: Scaffold(body: Center(child: Text('Field Deck skeleton'))),
    );
  }
}

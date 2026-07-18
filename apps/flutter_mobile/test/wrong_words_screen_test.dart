import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/wrong_words_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('practice wrong words show red/yellow counts in mark order', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: WrongWordsScreen(
          sdk: WordSdk.bridgeForTesting(bridge: _WrongWordsBridge()),
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    await tester.tap(find.text('模拟练习'));
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.text('标红 3 次 · 标黄 2 次'), findsOneWidget);
    expect(find.text('标红 1 次 · 标黄 4 次'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('red-heavy')).dy,
      lessThan(tester.getTopLeft(find.text('yellow-heavy')).dy),
    );
  });
}

class _WrongWordsBridge extends RustBridge {
  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'getWrongWords') return '[]';
    if (method == 'getExamVocabularyPriority') {
      return jsonEncode({
        'items': [
          {
            'word': 'yellow-heavy',
            'priorityScore': 100,
            'paperCount': 9,
            'articleCount': 9,
            'occurrenceCount': 9,
            'unknownMarkCount': 4,
            'wrongAssociationCount': 1,
            'masteredMarkCount': 0,
            'factors': [],
          },
          {
            'word': 'red-heavy',
            'priorityScore': 1,
            'paperCount': 1,
            'articleCount': 1,
            'occurrenceCount': 1,
            'unknownMarkCount': 2,
            'wrongAssociationCount': 3,
            'masteredMarkCount': 0,
            'factors': [],
          },
        ],
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

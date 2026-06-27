import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/wrong_word_graph_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('wrong word graph starts with an empty space and compact rail', (
    tester,
  ) async {
    final bridge = _GraphBridge();

    await tester.pumpWidget(
      MaterialApp(
        home: WrongWordGraphScreen(
          sdk: WordSdk.bridgeForTesting(bridge: bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      find.text('\u4ece\u53f3\u4fa7\u62d6\u5165\u5355\u8bcd'),
      findsOneWidget,
    );
    expect(find.text('\u9519\u8bcd'), findsOneWidget);
    expect(find.text('pre'), findsOneWidget);
    expect(find.text('sub'), findsOneWidget);
    expect(find.text('syn'), findsOneWidget);
    expect(find.text('7x'), findsOneWidget);
    expect(find.text('1x'), findsNWidgets(2));
    expect(find.text('1 errors'), findsNothing);
    expect(find.byIcon(Icons.drag_indicator_rounded), findsNothing);
    expect(bridge.calls, contains('getWrongWordGraph'));
  });

  testWidgets('selecting a rail word saves a graph position', (tester) async {
    final bridge = _GraphBridge();

    await tester.pumpWidget(
      MaterialApp(
        home: WrongWordGraphScreen(
          sdk: WordSdk.bridgeForTesting(bridge: bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(
      find.byKey(const ValueKey('wrong-word-graph-rail-word:word:1')),
    );
    await tester.pumpAndSettle();

    expect(bridge.savedEntryIds, [1]);
    expect(find.text('pre'), findsWidgets);
  });
}

class _GraphBridge extends RustBridge {
  final calls = <String>[];
  final savedEntryIds = <int>[];

  @override
  Future<String> call(String method, [String? argument]) async {
    calls.add(method);
    switch (method) {
      case 'getWrongWordGraph':
        return jsonEncode({
          'version': 1,
          'generatedAt': '2026-06-27T00:00:00Z',
          'nodes': [
            _node('word:1', 1, 'pre'),
            _node('word:2', 2, 'sub'),
            _node('word:3', 3, 'syn'),
          ],
          'edges': [],
          'coordinateSemantics': {},
          'relationLegend': [],
          'viewportHint': {},
        });
      case 'saveWrongWordGraphPosition':
        final payload = jsonDecode(argument ?? '{}') as Map<String, dynamic>;
        savedEntryIds.add((payload['entryId'] as num).toInt());
        return jsonEncode({
          'entryId': payload['entryId'],
          'entryKind': payload['entryKind'],
          'position': payload['position'],
          'isUserPlaced': true,
          'positionUpdatedAt': '2026-06-27T00:00:01Z',
        });
      default:
        return '{}';
    }
  }

  Map<String, Object?> _node(String id, int entryId, String word) => {
    'id': id,
    'entryId': entryId,
    'entryKind': 'word',
    'word': word,
    'primaryGloss': 'gloss',
    'meanings': [],
    'wrongCountToday': 0,
    'wrongCountTotal': entryId == 1 ? 7 : 1,
    'lastWrongAt': null,
    'priorityScore': 1,
    'masteryScore': 0,
    'urgencyScore': 0.2,
    'position': {'x': 0, 'y': 0, 'z': 0},
    'isUserPlaced': false,
    'sources': ['wrongWord'],
  };

  @override
  Future<void> callVoid(String method, [String? argument]) async {}

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> isAvailable() async => true;
}

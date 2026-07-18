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
    expect(
      tester.getTopLeft(find.text('pre')).dy,
      lessThan(tester.getTopLeft(find.text('sub')).dy),
    );
    expect(find.text('1 errors'), findsNothing);
    expect(find.byIcon(Icons.drag_indicator_rounded), findsNothing);
    expect(bridge.calls, contains('getWrongWordGraph'));
  });

  testWidgets('tapping a rail word does not place it', (tester) async {
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

    expect(bridge.savedEntryIds, isEmpty);
    expect(find.text('pre'), findsWidgets);
  });
  testWidgets('selected graph node shows readable relationship detail', (
    tester,
  ) async {
    final bridge = _GraphBridge(placed: true, withRelations: true);

    await tester.pumpWidget(
      MaterialApp(
        home: WrongWordGraphScreen(
          sdk: WordSdk.bridgeForTesting(bridge: bridge),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tapAt(const Offset(588, 478));
    await tester.pumpAndSettle();

    expect(find.textContaining('AI短文'), findsOneWidget);
    expect(find.textContaining('2024年6月四级阅读'), findsOneWidget);
    expect(find.textContaining('题目 q18'), findsOneWidget);
    expect(find.textContaining('释义'), findsWidgets);
    expect(find.textContaining('词根'), findsWidgets);
    expect(find.textContaining('形近'), findsWidgets);
    expect(find.textContaining('7x'), findsWidgets);
    expect(find.textContaining('2026-06-26T12:39'), findsNothing);
    expect(find.text('开始复习'), findsNothing);
  });
}

class _GraphBridge extends RustBridge {
  _GraphBridge({this.placed = false, this.withRelations = false});

  final bool placed;
  final bool withRelations;
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
          'edges': withRelations
              ? [
                  {
                    'id': 'synonym:word:1:word:2:shared',
                    'sourceNodeId': 'word:1',
                    'targetNodeId': 'word:2',
                    'relationType': 'synonym',
                    'weight': 0.6,
                    'evidence': [
                      {'type': 'sharedPrimaryGloss', 'meaning': '向前'},
                    ],
                  },
                  {
                    'id': 'coOccurrence:word:1:word:3:aiPassage:p1',
                    'sourceNodeId': 'word:1',
                    'targetNodeId': 'word:3',
                    'relationType': 'coOccurrence',
                    'weight': 0.7,
                    'evidence': [
                      {'type': 'aiPassage', 'passageTitle': '晨读短文'},
                    ],
                  },
                  {
                    'id': 'coOccurrence:word:1:word:2:sameArticle:paper-1',
                    'sourceNodeId': 'word:1',
                    'targetNodeId': 'word:2',
                    'relationType': 'coOccurrence',
                    'weight': 0.8,
                    'evidence': [
                      {
                        'type': 'sameArticle',
                        'articleId': 'cet4-2024-06:reading',
                        'articleTitle': '2024年6月四级阅读',
                        'questionId': 'q18',
                      },
                    ],
                  },
                  {
                    'id': 'rootFamily:word:1:word:2:prefix:pre',
                    'sourceNodeId': 'word:1',
                    'targetNodeId': 'word:2',
                    'relationType': 'rootFamily',
                    'weight': 0.6,
                    'evidence': [
                      {'type': 'wordFamilyHeuristic', 'family': 'prefix:pre'},
                    ],
                  },
                ]
              : [],
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
    'lastWrongAt': placed && entryId == 1
        ? '2026-06-26T12:39:48.609927810+00:00'
        : null,
    'priorityScore': 1,
    'masteryScore': 0,
    'urgencyScore': 0.2,
    'position': {
      'x': entryId == 1
          ? -1.2
          : entryId == 2
          ? -0.55
          : -0.1,
      'y': entryId == 1
          ? -0.2
          : entryId == 2
          ? 0.1
          : 0.3,
      'z': 0,
    },
    'isUserPlaced': placed,
    'sources': ['wrongWord'],
  };

  @override
  Future<void> callVoid(String method, [String? argument]) async {}

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> isAvailable() async => true;
}

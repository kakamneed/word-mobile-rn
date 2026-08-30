import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/learning_content_mode_selector.dart';
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
          content: LearningContentMode.examPractice,
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(SegmentedButton<LearningContentMode>), findsNothing);
    expect(find.text('模糊 0 次 · 眼熟 0 次 · 不会 2 次 · 致错 3 次'), findsOneWidget);
    expect(find.text('模糊 0 次 · 眼熟 0 次 · 不会 4 次 · 致错 1 次'), findsOneWidget);
    expect(
      tester.getTopLeft(find.text('red-heavy')).dy,
      lessThan(tester.getTopLeft(find.text('yellow-heavy')).dy),
    );

    await tester.tap(find.text('red-heavy'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 400));

    expect(find.text('Kaoyan English I 2019'), findsOneWidget);
    expect(find.textContaining('阅读理解 Text 1'), findsOneWidget);
    expect(find.text('模糊 0 次 · 眼熟 0 次 · 不会 2 次 · 致错 3 次'), findsWidgets);
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
            'sources': [
              {
                'paperId': 'kaoyan-english-1-2019',
                'paperTitle': 'Kaoyan English I 2019',
                'sectionId': 'reading-text-1',
                'sectionTitle': '阅读理解 Text 1',
                'wrongAssociationCount': 1,
                'unknownMarkCount': 0,
              },
            ],
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
            'sources': [
              {
                'paperId': 'kaoyan-english-1-2019',
                'paperTitle': 'Kaoyan English I 2019',
                'sectionId': 'reading-text-1',
                'sectionTitle': '阅读理解 Text 1',
                'wrongAssociationCount': 3,
                'unknownMarkCount': 2,
              },
            ],
          },
        ],
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

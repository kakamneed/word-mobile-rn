import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/learning_content_mode_selector.dart';
import 'package:flutter_mobile/features/reports_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('practice reports select an exam and show paper/type trends', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: ReportsScreen(
          sdk: WordSdk.bridgeForTesting(bridge: _ReportsBridge()),
          content: LearningContentMode.examPractice,
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 100));

    expect(find.byType(SegmentedButton<LearningContentMode>), findsNothing);
    expect(find.text('考试类型'), findsOneWidget);
    expect(find.text('整卷正确率'), findsOneWidget);
    await tester.scrollUntilVisible(
      find.text('完形填空'),
      400,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text('完形填空'), findsOneWidget);
    expect(find.text('2019'), findsWidgets);
    expect(find.text('2020'), findsWidgets);
  });
}

class _ReportsBridge extends RustBridge {
  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'getReportsOverview') {
      return jsonEncode({
        'totalStudyDays': 0,
        'totalWordsLearned': 0,
        'totalQuestionsAnswered': 0,
        'overallAccuracy': 0,
        'streakInfo': {},
        'modeBreakdown': [],
        'last7Days': [],
        'dailySeries': [],
        'modeSeries': {},
      });
    }
    if (method == 'getExamPracticeReport') {
      return jsonEncode({
        'exams': [
          {
            'exam': 'kaoyan-english-1',
            'papers': [
              {
                'paperId': 'p2019',
                'title': 'Kaoyan English I 2019',
                'year': 2019,
                'correctCount': 6,
                'totalQuestions': 10,
                'accuracyPercent': 60,
                'questionTypes': {
                  'cloze': {
                    'correctCount': 2,
                    'totalQuestions': 5,
                    'accuracyPercent': 40,
                  },
                  'reading': {
                    'correctCount': 4,
                    'totalQuestions': 5,
                    'accuracyPercent': 80,
                  },
                },
              },
              {
                'paperId': 'p2020',
                'title': 'Kaoyan English I 2020',
                'year': 2020,
                'correctCount': 8,
                'totalQuestions': 10,
                'accuracyPercent': 80,
                'questionTypes': {},
              },
            ],
          },
        ],
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

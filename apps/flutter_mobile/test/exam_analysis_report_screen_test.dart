import 'package:flutter/material.dart';
import 'package:flutter_mobile/features/ai_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

Iterable<TextSpan> textSpans(InlineSpan span) sync* {
  if (span is! TextSpan) return;
  yield span;
  for (final child in span.children ?? const <InlineSpan>[]) {
    yield* textSpans(child);
  }
}

void main() {
  const task = ExamAnalysisTask(
    taskId: 'task-1',
    exam: 'kaoyan-english-1',
    paperId: 'paper-2008',
    paperTitle: 'Kaoyan English I 2008',
    sectionId: 'reading-2',
    sectionTitle: '阅读 Text2',
    status: 'completed',
    unread: false,
    createdAt: '2026-08-18T12:00:00Z',
    completedAt: '2026-08-18T12:01:00Z',
    findings: [],
    review: {
      'reviewFormat': 'reading-review-v1',
      'questions': [
        {
          'questionId': 'q2',
          'questionNumber': 2,
          'stem': 'Which statement is true?',
          'selectedAnswer': 'A',
          'correctAnswer': 'C',
          'evidenceLocation': '第 2 段',
          'annotatedContext': 'The report makes heavy reading for publishers.',
          'optionAnalysis': [
            {'label': 'A', 'meaning': '批评政府资助的研究', 'analysis': '偷换了被批评的对象。'},
            {
              'label': 'C',
              'meaning': '令营利期刊出版商不安',
              'analysis': '对应 makes heavy reading。',
            },
          ],
          'analysis': '报告冲击的是限制访问并获利的出版商。',
          'knowledgeGap': '未识别 criticize 的宾语偷换。',
        },
      ],
      'correctMarkedQuestions': [
        {
          'questionId': 'q4',
          'questionNumber': 4,
          'distinction': 'cover 表示承担费用。',
        },
      ],
      'vocabularyPriority': [
        {
          'priority': 1,
          'word': 'report',
          'meaning': '报告',
          'mark': 'unknown',
          'markScope': 'current',
          'examFrequency': 28,
          'displayExamFrequency': 28,
          'examRank': 56,
          'priorityReason': '本篇标记。',
        },
        {
          'priority': 2,
          'word': 'publisher',
          'meaning': '出版商',
          'mark': 'familiar',
          'markScope': 'prior',
          'examFrequency': 6,
          'displayExamFrequency': 6,
          'examRank': 893,
          'priorityReason': '已有紫色标记。',
        },
        {
          'priority': 3,
          'word': 'heavy reading',
          'meaning': '令人难以接受的消息',
          'mark': 'familiar',
          'markScope': 'current',
          'examFrequency': 2,
          'displayExamFrequency': 2,
          'examRank': 2289,
          'priorityReason': '本篇标记短语。',
        },
        {
          'priority': 4,
          'word': 'criticize',
          'meaning': '批评',
          'examFrequency': 9,
          'displayExamFrequency': 12,
          'examFamilyRoot': 'critic',
          'examRank': 320,
          'markScope': 'current',
          'priorityReason': '直接造成第 2 题宾语关系误判。',
        },
      ],
    },
    error: '',
  );

  testWidgets('completed analysis opens a dedicated structured report page', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(360, 640));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(body: ExamAnalysisTaskList(tasks: [task])),
      ),
    );

    expect(find.byType(ExpansionTile), findsNothing);
    expect(find.textContaining('已完成'), findsOneWidget);
    expect(find.text('真正缺口'), findsNothing);

    await tester.tap(find.byKey(const ValueKey('exam-analysis-task-task-1')));
    await tester.pumpAndSettle();

    expect(find.text('阅读 Text2 · AI 分析'), findsOneWidget);
    expect(find.text('错题逐题复盘'), findsOneWidget);
    expect(find.text('第 2 题'), findsOneWidget);
    expect(find.text('选项辨析'), findsOneWidget);
    final markedContext = find.byWidgetPredicate(
      (widget) =>
          widget is RichText &&
          widget.text.toPlainText().contains('report（报告）') &&
          widget.text.toPlainText().contains('publishers（出版商）'),
    );
    expect(markedContext, findsOneWidget);
    final contextSpan = tester.widget<RichText>(markedContext).text as TextSpan;
    expect(
      contextSpan.toPlainText(),
      contains('heavy reading（令人难以接受的消息）'),
    );
    final spans = textSpans(contextSpan).toList(growable: false);
    expect(
      spans.any(
        (span) =>
            span.text == 'report' &&
            span.style?.backgroundColor == const Color(0xFFFFB84D),
      ),
      isTrue,
    );
    expect(
      spans.any(
        (span) =>
            span.text == 'heavy reading' &&
            span.style?.backgroundColor == const Color(0xFFFFDF70),
      ),
      isTrue,
    );
    expect(
      spans.any(
        (span) =>
            span.text == 'publishers' &&
            span.style?.backgroundColor == const Color(0xFFD9BDEA),
      ),
      isTrue,
    );

    await tester.scrollUntilVisible(
      find.text('答对题中的标记辨析'),
      240,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.text('答对题中的标记辨析'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.text('标记词重要程度'),
      240,
      scrollable: find.byType(Scrollable).last,
    );
    expect(find.text('标记词重要程度'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

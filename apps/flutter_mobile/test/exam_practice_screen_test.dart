import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/exam_practice_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_mobile/widgets/crocodile_frame_animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('exam passage removes sentence counters and indents paragraphs', () {
    const source =
        '1 Habits are a funny thing. 2 We reach for them mindlessly.\n'
        '1 So it seems paradoxical. 2 But researchers disagree.';

    final formatted = formatExamPassage(source);

    expect(formatted, startsWith('\u2003\u2003Habits are a funny thing.'));
    expect(formatted, contains('\n\u2003\u2003So it seems paradoxical.'));
    expect(formatted, isNot(contains(' 2 We reach')));
    expect(formatted, isNot(contains('\n1 So')));
  });

  test('cloze passage formats blanks as parenthesized numbers', () {
    const source =
        '1 It hoped lighting 1 workers productivity. 2 Instead, it ended 2 '
        'giving its name to the effect.';

    final formatted = formatClozePassage(source);

    expect(formatted, contains('(1) workers productivity'));
    expect(formatted, contains('ended (2) giving'));
    expect(formatted, isNot(contains('lighting 1 workers')));
  });

  test('exam explanation sanitizer removes markup and broken encoding', () {
    const source = '<p><strong>【答案解析】</strong></p><p>end&nbsp;up 表示最终成为��。</p>';

    expect(cleanExamExplanation(source), '【答案解析】\nend up 表示最终成为。');
  });

  testWidgets('learning mode selector switches to practice', (tester) async {
    TodayLearningContent? selected;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TodayLearningModeSelector(
            selected: TodayLearningContent.words,
            onChanged: (value) => selected = value,
          ),
        ),
      ),
    );

    await tester.tap(find.text('\u6a21\u62df\u7ec3\u4e60'));

    expect(selected, TodayLearningContent.practice);
  });

  testWidgets('practice catalog loading uses the crocodile loader', (
    tester,
  ) async {
    final bridge = _PendingExamCatalogBridge();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamPracticeHome(sdk: WordSdk.bridgeForTesting(bridge: bridge)),
        ),
      ),
    );

    expect(find.byType(CrocodileLoadingAnimation), findsOneWidget);
    expect(find.text('\u52a0\u8f7d\u4e2d...'), findsOneWidget);
  });

  testWidgets('question card keeps grading details in the section report', (
    tester,
  ) async {
    final question = ExamQuestion(
      id: 'q1',
      number: 1,
      kind: 'objective',
      stem: 'Choose one.',
      choices: const [
        ExamChoice(label: 'A', text: 'First'),
        ExamChoice(label: 'B', text: 'Second'),
      ],
      answer: 'A',
      explanation: 'Because A is correct.',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );
    String? selected;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamQuestionCard(
            question: question,
            selectedAnswer: null,
            submitted: false,
            onSelect: (value) => selected = value,
          ),
        ),
      ),
    );

    await tester.tap(find.text('First'));
    expect(selected, 'A');

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamQuestionCard(
            question: question,
            selectedAnswer: 'A',
            submitted: false,
            onSelect: (_) {},
          ),
        ),
      ),
    );
    expect(find.text('\u56de\u7b54\u6b63\u786e'), findsNothing);
    expect(find.text('Because A is correct.'), findsNothing);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamQuestionCard(
            question: question,
            selectedAnswer: 'A',
            submitted: true,
            onSelect: (_) {},
          ),
        ),
      ),
    );
    expect(find.text('\u56de\u7b54\u6b63\u786e'), findsNothing);
    expect(find.text('Because A is correct.'), findsNothing);
  });

  testWidgets('choice labels lead compact option rows', (tester) async {
    await tester.binding.setSurfaceSize(const Size(360, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final question = ExamQuestion(
      id: 'q1',
      number: 1,
      kind: 'objective',
      stem: 'Question 1',
      choices: const [
        ExamChoice(label: 'A', text: 'First option'),
        ExamChoice(label: 'B', text: 'Second option'),
        ExamChoice(label: 'C', text: 'Third option'),
        ExamChoice(label: 'D', text: 'Fourth option'),
      ],
      answer: 'A',
      explanation: '',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SingleChildScrollView(
            child: ExamQuestionCard(
              question: question,
              selectedAnswer: null,
              onSelect: (_) {},
              compact: true,
            ),
          ),
        ),
      ),
    );

    expect(
      tester.getTopLeft(find.text('A')).dx,
      lessThan(tester.getTopLeft(find.text('First option')).dx),
    );
    expect(tester.getSize(find.byType(ExamQuestionCard)).height, lessThan(300));
  });

  testWidgets(
    'section report shows overview before one cleaned answer detail',
    (tester) async {
      ExamQuestion question(String id, int number, String answer) =>
          ExamQuestion(
            id: id,
            number: number,
            kind: 'objective',
            stem: 'Question $number',
            choices: const [
              ExamChoice(label: 'A', text: 'First'),
              ExamChoice(label: 'B', text: 'Second'),
            ],
            answer: answer,
            explanation: '<p><strong>【答案解析】</strong></p><p>Clean detail.</p>',
            capabilities: const ExamQuestionCapabilities(
              browsable: true,
              answerable: true,
              autoGradable: true,
              causalAnalyzable: true,
            ),
          );
      final questions = [question('q1', 1, 'A'), question('q2', 2, 'B')];
      final section = ExamSection(
        id: 'reading',
        type: 'section',
        title: 'Reading',
        instructions: '',
        passage: 'Passage',
        questions: questions,
      );

      await tester.pumpWidget(
        MaterialApp(
          home: ExamSectionReportScreen(
            section: section,
            selections: const {'q1': 'A', 'q2': 'A'},
            result: gradeExamSubmission(questions, const {
              'q1': 'A',
              'q2': 'A',
            }),
            onAnalyze: () async => const [],
          ),
        ),
      );

      expect(find.text('1 / 2'), findsOneWidget);
      expect(find.text('AI 分析全部错题'), findsOneWidget);
      expect(find.text('Clean detail.'), findsNothing);
      expect(find.textContaining('<p'), findsNothing);

      await tester.tap(find.byKey(const ValueKey('report-question-q2')));
      await tester.pumpAndSettle();

      expect(find.text('正确答案：B'), findsOneWidget);
      expect(find.textContaining('Clean detail.'), findsOneWidget);
      expect(find.textContaining('<p'), findsNothing);
    },
  );

  test('article grading evaluates supported questions only after submit', () {
    final questions = [
      ExamQuestion(
        id: 'q1',
        number: 1,
        kind: 'objective',
        stem: 'One',
        choices: const [
          ExamChoice(label: 'A', text: 'A'),
          ExamChoice(label: 'B', text: 'B'),
        ],
        answer: 'A',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      ),
      ExamQuestion(
        id: 'q2',
        number: 2,
        kind: 'writing',
        stem: 'Write',
        choices: const [],
        answer: null,
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: false,
          causalAnalyzable: false,
        ),
      ),
    ];

    final result = gradeExamSubmission(questions, {'q1': 'B'});

    expect(result.gradableCount, 1);
    expect(result.correctCount, 0);
    expect(result.incorrectQuestionIds, ['q1']);
    expect(result.unsupportedCount, 1);
  });

  test('current article mark overrides prior mark and controls meaning', () {
    expect(
      resolveExamWordPresentation(
        normalized: 'resilient',
        mode: ExamPracticeMode.doing,
        currentArticleMarks: const {'resilient'},
        priorArticleMarks: const {'resilient'},
      ),
      const ExamWordPresentation(
        highlight: ExamWordHighlight.currentArticle,
        showMeaning: false,
      ),
    );
    expect(
      resolveExamWordPresentation(
        normalized: 'resilient',
        mode: ExamPracticeMode.analysis,
        currentArticleMarks: const {'resilient'},
        priorArticleMarks: const {'resilient'},
      ).showMeaning,
      isTrue,
    );
    expect(
      resolveExamWordPresentation(
        normalized: 'legacy',
        mode: ExamPracticeMode.analysis,
        currentArticleMarks: const {},
        priorArticleMarks: const {'legacy'},
      ),
      const ExamWordPresentation(
        highlight: ExamWordHighlight.priorArticle,
        showMeaning: false,
      ),
    );
  });

  test('word toggle uses article-level mark state across occurrences', () {
    expect(nextExamWordMark('habit', const {'habit'}), 'none');
    expect(nextExamWordMark('habit', const {}), 'unknown');
  });

  test('analysis uses one compact contextual meaning', () {
    expect(pickExamContextMeaning(['只要；只要是；在……期间', '长久地']), '只要');
    expect(pickExamContextMeaning(const []), '');
  });

  test('single reader remembers passage and question bookmarks', () {
    final memory = ExamReadingPositionMemory(
      active: ExamReadingPane.passage,
      passageOffset: 120,
      questionOffset: 860,
    );

    expect(memory.switchFrom(245), 860);
    expect(memory.active, ExamReadingPane.questions);
    expect(memory.passageOffset, 245);
    expect(memory.switchFrom(910), 245);
    expect(memory.questionOffset, 910);
  });

  testWidgets('edge bubble switches the bookmark for one reader', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(360, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    var active = ExamReadingPane.passage;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) => ExamReadingPositionBubble(
            active: active,
            bubbleAlignment: 0,
            onSwitch: () => setState(() {
              active = active == ExamReadingPane.passage
                  ? ExamReadingPane.questions
                  : ExamReadingPane.passage;
            }),
            onBubbleAlignmentChanged: (_) {},
            child: const Text(
              'continuous-reader',
              key: ValueKey('continuous-reader'),
            ),
          ),
        ),
      ),
    );

    expect(
      find.byKey(const ValueKey('exam-pane-focus-bubble')),
      findsOneWidget,
    );
    expect(find.text('\u2193\u9898'), findsOneWidget);
    final initialPosition = tester.widget<Positioned>(
      find.ancestor(
        of: find.byKey(const ValueKey('exam-pane-focus-bubble')),
        matching: find.byType(Positioned),
      ),
    );
    expect(initialPosition.right, lessThan(0));
    expect(
      tester
          .widget<Opacity>(
            find.descendant(
              of: find.byKey(const ValueKey('exam-pane-focus-bubble')),
              matching: find.byType(Opacity),
            ),
          )
          .opacity,
      lessThan(1),
    );

    await tester.tap(find.byKey(const ValueKey('exam-pane-focus-bubble')));
    await tester.pumpAndSettle();

    expect(active, ExamReadingPane.questions);
    expect(find.byKey(const ValueKey('continuous-reader')), findsOneWidget);
    expect(
      find.byKey(const ValueKey('exam-pane-focus-bubble')),
      findsOneWidget,
    );
    expect(find.text('\u2191\u6587'), findsOneWidget);
  });

  testWidgets('continuous reader builds the initial question anchor', (
    tester,
  ) async {
    final controller = ScrollController();
    final questionAnchor = GlobalKey();
    addTearDown(controller.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamContinuousReader(
            controller: controller,
            padding: EdgeInsets.zero,
            children: [
              const SizedBox(height: 5000),
              SizedBox(key: questionAnchor, height: 20),
            ],
          ),
        ),
      ),
    );

    expect(questionAnchor.currentContext, isNotNull);
  });

  testWidgets('doing mode tap marks without revealing a meaning sheet', (
    tester,
  ) async {
    final bridge = _ExamWordBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    var meanings = <String, String>{};
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => ExamInteractiveText(
              client: client,
              text: 'Habit',
              articleId: 'paper:section',
              title: 'Reading',
              metadata: const {'scope': 'passage'},
              currentMeanings: meanings,
              onInspectionChanged: (inspection) => setState(() {
                meanings = inspection.isUnknown
                    ? {inspection.normalized: inspection.meanings.join('; ')}
                    : {};
              }),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byType(SelectableText));
    await tester.pumpAndSettle();
    expect(meanings, {'habit': 'habit meaning'});
    expect(find.text('habit meaning'), findsNothing);
    expect(find.byType(BottomSheet), findsNothing);

    final selectable = tester.widget<SelectableText>(
      find.byType(SelectableText).first,
    );
    final tokenSpan = selectable.textSpan!.children!.first as TextSpan;
    expect(tokenSpan.style?.decoration, isNull);
    expect(tokenSpan.style?.backgroundColor, const Color(0xFFFFE082));
  });

  testWidgets(
    'analysis reveals persisted meanings and bundled translations without AI',
    (tester) async {
      final bridge = _OfflineAnalysisBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());
      final paper = ExamPaper(
        id: 'paper-1',
        exam: 'kaoyan-english-1',
        title: 'Offline paper',
        year: 2010,
        month: null,
        setNumber: null,
        sections: const [
          ExamSection(
            id: 'reading',
            type: 'reading',
            title: 'Reading',
            instructions: '',
            passage: 'Habit matters.',
            paragraphTranslations: ['习惯很重要。'],
            questions: [],
          ),
        ],
      );

      await tester.pumpWidget(
        MaterialApp(
          home: ExamPracticeScreen(
            client: client,
            paper: paper,
            initialSectionId: 'reading',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.textContaining('（习惯）', findRichText: true), findsNothing);
      await tester.tap(find.text('分析').first);
      await tester.pumpAndSettle();
      expect(find.textContaining('（习惯）', findRichText: true), findsOneWidget);

      await tester.tap(find.text('显示段落译文'));
      await tester.pumpAndSettle();
      expect(find.text('习惯很重要。'), findsOneWidget);
      expect(bridge.methods, isNot(contains('analyzeExamReadingContext')));
    },
  );

  testWidgets('subjective reference answer stays hidden until requested', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: ExamSubjectiveReferencePanel(
            kind: 'translation',
            answers: ['第一句参考译文。', '第二句参考译文。'],
          ),
        ),
      ),
    );

    expect(find.text('第一句参考译文。'), findsNothing);
    expect(find.textContaining('主观题暂不评分'), findsOneWidget);
    await tester.tap(find.text('显示第 1 题答案'));
    await tester.pumpAndSettle();
    expect(find.text('第一句参考译文。'), findsOneWidget);
    expect(find.text('隐藏第 1 题答案'), findsOneWidget);
    expect(find.text('第二句参考译文。'), findsNothing);
  });
}

class _ExamWordBridge extends RustBridge {
  bool marked = false;

  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'tokenizeExamText') {
      return jsonEncode({
        'tokens': [
          {
            'text': 'Habit',
            'normalized': 'habit',
            'startOffset': 0,
            'endOffset': 5,
          },
        ],
      });
    }
    if (method == 'inspectExamWord') {
      final request = jsonDecode(argument!) as Map<String, dynamic>;
      if (request['userMark'] case final String mark) {
        marked = mark == 'unknown';
      }
      return jsonEncode({
        'occurrenceId': 1,
        'entryId': 2,
        'word': 'Habit',
        'normalized': 'habit',
        'meanings': ['habit meaning'],
        'userMark': marked ? 'unknown' : 'none',
        'isUnknown': marked,
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _PendingExamCatalogBridge extends RustBridge {
  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'getExamCatalog') {
      return Future<String>.delayed(const Duration(minutes: 1));
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _OfflineAnalysisBridge extends RustBridge {
  final List<String> methods = [];

  @override
  Future<String> call(String method, [String? argument]) async {
    methods.add(method);
    switch (method) {
      case 'getExamAttempt':
        return jsonEncode({'attempt': null});
      case 'tokenizeExamText':
        return jsonEncode({
          'tokens': [
            {
              'text': 'Habit',
              'normalized': 'habit',
              'startOffset': 2,
              'endOffset': 7,
            },
          ],
        });
      case 'getExamAnnotationState':
        return jsonEncode({
          'currentMarks': [
            {'normalized': 'habit', 'meaning': '习惯'},
          ],
          'priorMarks': [],
          'annotations': [],
        });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

import 'dart:convert';
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/exam_practice_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_mobile/widgets/crocodile_frame_animation.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'section AI analysis allows the verified Pro Alpha completion window',
    () {
      expect(examSectionAnalysisTimeout, const Duration(seconds: 360));
    },
  );

  test('interaction tokenizer keeps accented and ligature words whole', () {
    final tokens = tokenizeExamTextForInteraction(
      "A café ﬁnancial co-op isn’t plain.",
    );

    expect(tokens.map((token) => token.text), [
      'A',
      'café',
      'ﬁnancial',
      'co-op',
      'isn’t',
      'plain',
    ]);
    expect(tokens.map((token) => token.normalized), [
      'a',
      'café',
      'ﬁnancial',
      'co-op',
      "isn't",
      'plain',
    ]);
  });

  test('wrong-answer detail is available without AI findings', () {
    final question = ExamQuestion(
      id: 'q1',
      number: 1,
      kind: 'objective',
      stem: 'Why second?',
      choices: const [
        ExamChoice(label: 'A', text: 'First'),
        ExamChoice(label: 'B', text: 'Second'),
      ],
      answer: 'B',
      explanation: 'Reference explanation.',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );
    final detail = buildExamWrongAnswerReportDetail(
      section: ExamSection(
        id: 's',
        type: 'reading',
        title: 'Reading',
        instructions: '',
        passage:
            'First topic.\n\nThe second paragraph explains the second choice.',
        questions: [question],
      ),
      question: question,
      selections: const {'q1': 'A'},
    );
    expect(detail, isNotNull);
    expect(detail!.selectedOption, 'A · First');
    expect(detail.correctOption, 'B · Second');
    expect(detail.passageLocation, '第 2 段');
  });

  test(
    'section AI analysis stops waiting when a provider never returns',
    () async {
      final question = ExamQuestion(
        id: 'q-timeout',
        number: 1,
        kind: 'objective',
        stem: 'Choose one.',
        choices: const [],
        answer: 'A',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      );

      await expectLater(
        analyzeExamWrongQuestionsConcurrently(
          [question],
          timeout: const Duration(milliseconds: 20),
          analyze: (_) => Completer<Map<String, dynamic>>().future,
        ),
        throwsA(
          isA<StateError>().having(
            (error) => error.message,
            'message',
            contains('TimeoutException'),
          ),
        ),
      );
    },
  );

  test(
    'causal analysis surfaces provider failure instead of empty findings',
    () {
      expect(
        () => parseExamCausalFindings(const {
          'eligible': true,
          'providerStatus': 'provider_failure',
          'candidates': [],
          'limitations': ['AI provider unavailable: timeout'],
        }, questionNumber: 1),
        throwsA(
          isA<StateError>().having(
            (error) => error.message,
            'message',
            contains('timeout'),
          ),
        ),
      );
    },
  );

  test(
    'section AI analysis reports an unavailable provider as retryable',
    () async {
      ExamQuestion question(String id, int number) => ExamQuestion(
        id: id,
        number: number,
        kind: 'objective',
        stem: 'Question $number',
        choices: const [],
        answer: 'A',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      );

      await expectLater(
        analyzeExamWrongQuestionsConcurrently(
          [question('q1', 1), question('q2', 2)],
          analyze: (_) async => const {
            'eligible': true,
            'providerStatus': 'provider_failure',
            'candidates': [],
            'limitations': ['AI provider unavailable: HTTP 503'],
          },
        ),
        throwsA(
          isA<ExamCausalAnalysisException>()
              .having(
                (error) => error.kind,
                'kind',
                ExamCausalAnalysisFailureKind.providerUnavailable,
              )
              .having(
                (error) => error.message,
                'message',
                contains('AI 服务暂时不可用'),
              )
              .having(
                (error) => error.message,
                'message',
                isNot(contains('未覆盖全部错题')),
              ),
        ),
      );
    },
  );

  test('section AI analysis rejects partial wrong-question coverage', () async {
    ExamQuestion question(String id, int number) => ExamQuestion(
      id: id,
      number: number,
      kind: 'objective',
      stem: 'Question $number',
      choices: const [],
      answer: 'A',
      explanation: '',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );

    await expectLater(
      analyzeExamWrongQuestionsConcurrently(
        [question('q1', 1), question('q2', 2)],
        analyze: (question) async {
          if (question.id == 'q1') {
            throw StateError('insufficient evidence');
          }
          return const {
            'eligible': true,
            'providerStatus': 'completed',
            'candidates': [
              {
                'word': 'consequential',
                'confidence': 0.9,
                'reasoning': 'reason',
              },
            ],
          };
        },
      ),
      throwsA(
        isA<StateError>().having(
          (error) => error.message,
          'message',
          contains('insufficient evidence'),
        ),
      ),
    );
  });

  test('cloze AI review parser keeps the report sections in review order', () {
    final questions = [
      ExamQuestion(
        id: 'q1',
        number: 1,
        kind: 'objective',
        stem: '',
        choices: const [],
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
        kind: 'objective',
        stem: '',
        choices: const [],
        answer: 'B',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      ),
    ];
    final review = parseExamSectionAiAnalysis(const {
      'eligible': true,
      'providerStatus': 'completed',
      'reviewFormat': 'cloze-review-v1',
      'questions': [
        {
          'questionId': 'q1',
          'contextSentence': 'A rare bird appeared.',
          'annotatedContext': 'A rare（罕见的） bird appeared.',
          'optionAnalysis': [
            {'label': 'A', 'meaning': '罕见的', 'analysis': '符合语境'},
          ],
          'analysis': '上下文强调少见。',
          'knowledgeGap': '未掌握 rare。',
          'candidates': [],
        },
      ],
      'correctMarkedQuestions': [
        {'questionId': 'q2', 'distinction': 'observe 是观察，不是赞同。'},
      ],
      'vocabularyPriority': [
        {
          'priority': 1,
          'word': 'observe',
          'meaning': '观察',
          'mark': 'familiar',
          'examFrequency': 1,
          'displayExamFrequency': 12,
          'examFamilyFrequency': 12,
          'examFamilyRoot': 'observe',
          'examRank': 8,
          'priorityReason': '高频且多义。',
        },
      ],
    }, questions: questions);

    expect(review.clozeQuestions.single.annotatedContext, contains('罕见的'));
    expect(review.correctMarkedQuestions.single.questionNumber, 2);
    expect(review.vocabularyPriority.single.examFrequency, 12);
    expect(review.vocabularyPriority.single.strictExamFrequency, 1);
    expect(review.vocabularyPriority.single.examFamilyRoot, 'observe');
  });

  test('cloze AI review rejects a structurally incomplete wrong question', () {
    final question = ExamQuestion(
      id: 'q1',
      number: 1,
      kind: 'objective',
      stem: '',
      choices: const [ExamChoice(label: 'A', text: 'rare')],
      answer: 'A',
      explanation: '',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );

    expect(
      () => parseExamSectionAiAnalysis(
        const {
          'eligible': true,
          'providerStatus': 'completed',
          'reviewFormat': 'cloze-review-v1',
          'questions': [
            {'questionId': 'q1', 'candidates': []},
          ],
          'vocabularyPriority': [],
        },
        questions: [question],
        requiredWrongQuestionIds: const {'q1'},
      ),
      throwsA(
        isA<ExamCausalAnalysisException>().having(
          (error) => error.kind,
          'kind',
          ExamCausalAnalysisFailureKind.incomplete,
        ),
      ),
    );
  });

  test(
    'reading AI review parser keeps question answers and evidence location',
    () {
      final question = ExamQuestion(
        id: 'q1',
        number: 1,
        kind: 'objective',
        stem: 'What changed?',
        choices: const [
          ExamChoice(label: 'A', text: 'The old process'),
          ExamChoice(label: 'B', text: 'The publication model'),
        ],
        answer: 'B',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      );

      final review = parseExamSectionAiAnalysis(
        const {
          'eligible': true,
          'providerStatus': 'completed',
          'reviewFormat': 'reading-review-v1',
          'questions': [
            {
              'questionId': 'q1',
              'stem': 'What changed?',
              'selectedAnswer': 'A',
              'correctAnswer': 'B',
              'evidenceLocation': '第2段第1至2句',
              'contextSentence':
                  'The old process changed. A new model emerged.',
              'annotatedContext':
                  'The old process changed. A new model emerged（出现）.',
              'optionAnalysis': [
                {'label': 'A', 'meaning': '旧流程', 'analysis': '范围过窄'},
                {'label': 'B', 'meaning': '出版模式', 'analysis': '概括全文'},
              ],
              'analysis': '转折后反复说明新出版模式。',
              'knowledgeGap': '忽略了全文主线。',
              'candidates': [],
            },
          ],
          'correctMarkedQuestions': [],
          'vocabularyPriority': [],
        },
        questions: [question],
        requiredWrongQuestionIds: const {'q1'},
      );

      expect(review.reviewFormat, 'reading-review-v1');
      expect(review.clozeQuestions.single.stem, 'What changed?');
      expect(review.clozeQuestions.single.selectedAnswer, 'A');
      expect(review.clozeQuestions.single.correctAnswer, 'B');
      expect(review.clozeQuestions.single.evidenceLocation, '第2段第1至2句');
    },
  );

  test('wrong-question causal requests start concurrently', () async {
    ExamQuestion question(String id, int number) => ExamQuestion(
      id: id,
      number: number,
      kind: 'objective',
      stem: 'Question $number',
      choices: const [ExamChoice(label: 'A', text: 'First')],
      answer: 'A',
      explanation: '',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );
    final questions = [question('q1', 1), question('q2', 2)];
    final pending = {
      for (final question in questions)
        question.id: Completer<Map<String, dynamic>>(),
    };
    final started = <String>[];

    final result = analyzeExamWrongQuestionsConcurrently(
      questions,
      analyze: (question) {
        started.add(question.id);
        return pending[question.id]!.future;
      },
    );
    await Future<void>.delayed(Duration.zero);
    expect(started, ['q1', 'q2']);

    for (final completer in pending.values) {
      completer.complete(const {
        'eligible': true,
        'providerStatus': 'completed',
        'candidates': [],
      });
    }
    expect(await result, isEmpty);
  });

  test(
    'section AI analysis does not overfill the two native workers',
    () async {
      ExamQuestion question(int number) => ExamQuestion(
        id: 'q$number',
        number: number,
        kind: 'objective',
        stem: 'Question $number',
        choices: const [],
        answer: 'A',
        explanation: '',
        capabilities: const ExamQuestionCapabilities(
          browsable: true,
          answerable: true,
          autoGradable: true,
          causalAnalyzable: true,
        ),
      );
      final pending = <int, Completer<Map<String, dynamic>>>{};
      final started = <int>[];
      final result = analyzeExamWrongQuestionsConcurrently(
        [question(1), question(2), question(3)],
        analyze: (question) {
          started.add(question.number);
          return (pending[question.number] = Completer<Map<String, dynamic>>())
              .future;
        },
      );

      await Future<void>.delayed(Duration.zero);
      expect(started, [1, 2]);
      pending[1]!.complete(const {
        'eligible': true,
        'providerStatus': 'completed',
        'candidates': [],
      });
      await Future<void>.delayed(Duration.zero);
      expect(started, [1, 2, 3]);
      for (final number in [2, 3]) {
        pending[number]!.complete(const {
          'eligible': true,
          'providerStatus': 'completed',
          'candidates': [],
        });
      }
      expect(await result, isEmpty);
    },
  );

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

    final formatted = formatClozePassage(source, blankNumbers: {1, 2});

    expect(formatted, contains('(1) workers productivity'));
    expect(formatted, contains('ended (2) giving'));
    expect(formatted, isNot(contains('lighting 1 workers')));
  });

  test('cloze passage preserves ordinary numbers that resemble blank ids', () {
    const source =
        '1 The method was authorized 10 years ago before behavior 10 itself '
        'changed. 2 A 20th century pattern eventually reached 20 a plateau.';

    final formatted = formatClozePassage(source, blankNumbers: {10, 20});

    expect(formatted, contains('10 years ago'));
    expect(formatted, contains('behavior (10) itself'));
    expect(formatted, contains('20th century'));
    expect(formatted, contains('reached (20) a plateau'));
  });

  test('submitted cloze answers replace only their matching blank markers', () {
    const source =
        '1 The method was authorized 10 years ago before behavior 10 itself.';

    final formatted = formatClozePassage(
      source,
      blankNumbers: {10},
      submittedAnswers: const {10: 'by'},
    );

    expect(formatted, contains('10 years ago'));
    expect(formatted, contains('behavior (by) itself'));
    expect(formatted, isNot(contains('(10) years')));
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
        passage:
            'The first paragraph discusses an unrelated topic.\n\n'
            'The second paragraph explains why the correct choice is second.',
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
            onAnalyze: () async => const ExamSectionAiAnalysis(findings: []),
          ),
        ),
      );

      expect(find.text('1 / 2'), findsOneWidget);
      expect(find.text('AI 分析全部错题'), findsOneWidget);
      expect(find.text('Clean detail.'), findsNothing);
      expect(find.textContaining('<p'), findsNothing);

      await tester.tap(find.byKey(const ValueKey('report-question-q2')));
      await tester.pumpAndSettle();

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

  test(
    'current and prior familiarity marks preserve both highlight levels',
    () {
      expect(
        resolveExamWordPresentation(
          normalized: 'resilient',
          mode: ExamPracticeMode.doing,
          currentArticleMarks: const {'resilient'},
          priorArticleMarks: const {'resilient'},
          currentMarkKinds: const {'resilient': 'familiar'},
          priorMarkKinds: const {'resilient': 'unknown'},
        ),
        const ExamWordPresentation(
          highlight: ExamWordHighlight.mixed,
          currentMarkLevel: 'familiar',
          priorMarkLevel: 'unknown',
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
          priorMarkLevel: 'unknown',
          showMeaning: false,
        ),
      );
    },
  );

  test('word toggle uses article-level mark state across occurrences', () {
    expect(nextExamWordMark('habit', const {'habit'}), 'none');
    expect(nextExamWordMark('habit', const {}), 'unknown');
  });

  test('three familiarity levels use distinct yellow and purple strengths', () {
    final presentation = resolveExamWordPresentation(
      normalized: 'patents',
      mode: ExamPracticeMode.doing,
      currentArticleMarks: const {'patent'},
      priorArticleMarks: const {},
      currentMarkKinds: const {'patent': 'fuzzy'},
    );
    expect(presentation.highlight, ExamWordHighlight.fuzzy);
    expect(examCurrentHighlightColor('fuzzy'), const Color(0xFFFFF3BF));
    expect(examCurrentHighlightColor('familiar'), const Color(0xFFFFDF70));
    expect(examCurrentHighlightColor('unknown'), const Color(0xFFFFB84D));
    expect(examPriorHighlightColor('fuzzy'), const Color(0xFFEDE2F7));
    expect(examPriorHighlightColor('familiar'), const Color(0xFFD9BDEA));
    expect(examPriorHighlightColor('unknown'), const Color(0xFFBE8ADB));
  });

  test('mark palette maps horizontal drag to fuzzy familiar and unknown', () {
    const rect = Rect.fromLTWH(100, 80, 120, 40);
    expect(pickExamFamiliarityLevel(rect, const Offset(110, 100)), 'fuzzy');
    expect(pickExamFamiliarityLevel(rect, const Offset(160, 100)), 'familiar');
    expect(pickExamFamiliarityLevel(rect, const Offset(210, 100)), 'unknown');
  });

  test('inflected forms share one exam vocabulary family', () {
    expect(normalizeExamWordFamily('patents'), 'patent');
    expect(normalizeExamWordFamily('authorized'), 'authorize');
    expect(normalizeExamWordFamily('studies'), 'study');
    expect(normalizeExamWordFamily('children'), 'child');
    expect(normalizeExamWordFamily('went'), 'go');
    expect(normalizeExamWordFamily('conduct'), 'conduct');
  });

  test('marked inflections resolve meanings stored under either word form', () {
    expect(
      resolveExamMarkedMeaning(const {'arousing': '引起强烈感情的；激起的'}, 'arousing'),
      '引起强烈感情的；激起的',
    );
    expect(
      resolveExamMarkedMeaning(const {'arouse': '唤起；激起'}, 'arousing'),
      '唤起；激起',
    );
  });

  test('context meaning selects the sense used by the sentence', () {
    expect(
      pickExamContextMeaning(
        ['n. 电路, 环(行)道, 巡回；[计] 线路; 电路'],
        normalized: 'circuit',
        context: 'the U.S. Court of Appeals for the Federal Circuit',
        translation: '美国联邦巡回上诉法院将审查这起案件。',
      ),
      '巡回上诉法院',
    );
    expect(
      pickExamContextMeaning(
        ['n. 行为, 举动, 指导；vt. 为人, 指挥, 管理, 实施；vi. 领导, 传导, 指挥'],
        normalized: 'conduct',
        context: 'conduct a broad review of business-method patents',
      ),
      '开展；进行',
    );
    expect(
      pickExamContextMeaning(
        ['n. 刻度，规模，比例；v. 攀登，衡量'],
        normalized: 'scale',
        context: 'the scale of the losses',
        translation: '损失的规模',
      ),
      '规模',
    );
    expect(
      pickExamContextMeaning(
        ['n. 刻度，规模，比例；v. 攀登，衡量'],
        normalized: 'scale',
        context: 'the company plans to scale the service quickly',
      ),
      '攀登，衡量',
    );
  });

  test(
    'phrase selection normalization removes drag punctuation and whitespace',
    () {
      expect(
        normalizeExamPhraseSelection('  “In  case\nthat,”  '),
        'in case that',
      );
    },
  );

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

  testWidgets('long press on a marked word opens the color palette directly', (
    tester,
  ) async {
    final bridge = _ExamWordBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    var meanings = <String, String>{'habit': 'habit meaning'};
    var marks = <String, String>{'habit': 'fuzzy'};
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
              currentMarks: marks,
              onInspectionChanged: (inspection) => setState(() {
                meanings = inspection.isUnknown
                    ? {inspection.normalized: inspection.meanings.join('; ')}
                    : {};
                marks = inspection.isUnknown
                    ? {inspection.normalized: bridge.inspectedUserMark!}
                    : {};
              }),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.longPress(find.byType(SelectableText));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('exam-mark-palette')), findsOneWidget);
    expect(find.byKey(const ValueKey('exam-mark-familiar')), findsOneWidget);
    final editable = tester.state<EditableTextState>(find.byType(EditableText));
    expect(editable.textEditingValue.selection.isCollapsed, isTrue);

    await tester.tap(find.byKey(const ValueKey('exam-mark-unknown')));
    await tester.pumpAndSettle();
    expect(bridge.inspectedUserMark, 'unknown');
    expect(marks, {'habit': 'unknown'});
  });

  testWidgets('single tap marks an unmarked word as familiar', (tester) async {
    final bridge = _ExamWordBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    var marks = <String, String>{};
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
              currentMarks: marks,
              onInspectionChanged: (inspection) => setState(() {
                marks = inspection.isUnknown
                    ? {inspection.normalized: bridge.inspectedUserMark!}
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

    expect(bridge.inspectedUserMark, 'familiar');
    expect(marks, {'habit': 'familiar'});
  });

  testWidgets('words remain tappable when native tokenization fails', (
    tester,
  ) async {
    final bridge = _FailedTokenizationBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ExamInteractiveText(
            client: client,
            text: 'Resilient',
            articleId: 'paper:section',
            title: 'Reading',
            metadata: const {'scope': 'passage'},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(SelectableText), findsOneWidget);
    await tester.tap(find.byType(SelectableText));
    await tester.pumpAndSettle();

    expect(bridge.inspectedWord, 'Resilient');
    expect(bridge.inspectedUserMark, 'familiar');
  });

  testWidgets(
    'short words include their trailing separator in the tap target',
    (tester) async {
      const style = TextStyle(fontSize: 36, height: 1);
      final bridge = _PerWordHitBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ExamInteractiveText(
              client: client,
              text: 'a resilient',
              articleId: 'paper:section',
              title: 'Reading',
              metadata: const {'scope': 'passage'},
              style: style,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final wordPainter = TextPainter(
        text: const TextSpan(text: 'a', style: style),
        textDirection: TextDirection.ltr,
      )..layout();
      final wordAndGapPainter = TextPainter(
        text: const TextSpan(text: 'a ', style: style),
        textDirection: TextDirection.ltr,
      )..layout();
      final textOrigin = tester.getTopLeft(find.byType(SelectableText));
      await tester.tapAt(
        textOrigin +
            Offset(
              (wordPainter.width + wordAndGapPainter.width) / 2,
              wordAndGapPainter.height / 2,
            ),
      );
      await tester.pumpAndSettle();

      expect(bridge.inspectedWord, 'a');
      expect(bridge.inspectedUserMark, 'familiar');
    },
  );

  testWidgets('first and last words remain tappable beside the text edges', (
    tester,
  ) async {
    const style = TextStyle(fontSize: 32, height: 1.2);
    final visibleText = TextPainter(
      text: const TextSpan(text: '\u2003\u2003edge target', style: style),
      textDirection: TextDirection.ltr,
    )..layout();
    final bridge = _PerWordHitBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: visibleText.width + 2,
              child: ExamInteractiveText(
                client: client,
                text: '\u2003\u2003edge target',
                articleId: 'paper:section',
                title: 'Reading',
                metadata: const {'scope': 'choice:A'},
                style: style,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final selectable = find.byType(SelectableText);
    final origin = tester.getTopLeft(selectable);
    final size = tester.getSize(selectable);
    final leadingIndent = TextPainter(
      text: const TextSpan(text: '\u2003\u2003', style: style),
      textDirection: TextDirection.ltr,
    )..layout();
    await tester.tapAt(
      origin + Offset(leadingIndent.width / 2, size.height / 2),
    );
    await tester.pumpAndSettle();
    expect(bridge.inspectedWord, 'edge');

    bridge.inspectedWord = null;
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    const text = '\u2003\u2003edge target';
    final targetStart = text.indexOf('target');
    final targetBox = editable
        .getBoxesForSelection(
          TextSelection(
            baseOffset: targetStart,
            extentOffset: targetStart + 'target'.length,
          ),
        )
        .first
        .toRect();
    await tester.tapAt(
      editable.localToGlobal(Offset(targetBox.right + 2, targetBox.center.dy)),
    );
    await tester.pumpAndSettle();
    expect(bridge.inspectedWord, 'target');
  });

  testWidgets('2011 reading wrapped words use their actual rendered line boxes', (
    tester,
  ) async {
    const passage =
        '\u2003\u2003The decision of the New York Philharmonic to hire Alan Gilbert as its next music director has been the talk of the classical-music world ever since the sudden announcement of his appointment in 2009. For the most part, the response has been favorable, to say the least. "Hooray! At last!" wrote Anthony Tommasini, a sober-sided classical-music critic.';
    const style = TextStyle(fontSize: 16, height: 1.65);
    final bridge = _PerWordHitBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 378,
              child: ExamInteractiveText(
                client: client,
                text: passage,
                articleId: 'kaoyan-english-1:2011:reading-text-1',
                title: 'Kaoyan English I 2011 / Reading Text 1',
                metadata: const {'scope': 'passage'},
                style: style,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;

    Future<void> tapWordEdge(String word) async {
      final start = passage.indexOf(word);
      final boxes = editable.getBoxesForSelection(
        TextSelection(baseOffset: start, extentOffset: start + word.length),
      );
      expect(boxes, isNotEmpty);
      final rect = boxes.first.toRect();
      await tester.tapAt(
        editable.localToGlobal(Offset(rect.right + 2, rect.center.dy)),
      );
      await tester.pumpAndSettle();
    }

    await tapWordEdge('director');
    expect(bridge.inspectedWord, 'director');

    bridge.inspectedWord = null;
    await tapWordEdge('sober-sided');
    expect(bridge.inspectedWord, 'sober-sided');
  });

  testWidgets('marking music highlights music instances but not musicians', (
    tester,
  ) async {
    const passage =
        '\u2003\u2003To be sure, he performs an impressive variety of interesting compositions, but it is not necessary for me to visit Avery Fisher Hall, or anywhere else, to hear interesting orchestral music. All I have to do is to go to my CD shelf, or boot up my computer and download still more recorded music from iTunes.\n\u2003\u2003Devoted concertgoers must compete with the recorded performances of the great classical musicians of the 20th century.';
    final bridge = _PerWordHitBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    var meanings = <String, String>{};
    var marks = <String, String>{};
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 378,
              child: StatefulBuilder(
                builder: (context, setState) => ExamInteractiveText(
                  client: client,
                  text: passage,
                  articleId: 'kaoyan-english-1:2011:reading-text-1',
                  title: 'Kaoyan English I 2011 / Reading Text 1',
                  metadata: const {'scope': 'passage'},
                  style: const TextStyle(fontSize: 16, height: 1.65),
                  currentMeanings: meanings,
                  currentMarks: marks,
                  onInspectionChanged: (inspection) => setState(() {
                    meanings = {inspection.normalized: ''};
                    marks = {inspection.normalized: 'familiar'};
                  }),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    final start = passage.indexOf('music.');
    final box = editable
        .getBoxesForSelection(
          TextSelection(baseOffset: start, extentOffset: start + 5),
        )
        .first
        .toRect();
    await tester.tapAt(
      editable.localToGlobal(Offset(box.right + 2, box.center.dy)),
    );
    await tester.pumpAndSettle();

    expect(bridge.inspectedWord, 'music');
    final selectable = tester.widget<SelectableText>(
      find.byType(SelectableText),
    );
    final wordSpans = selectable.textSpan!.children!.whereType<TextSpan>();
    final musicSpans = wordSpans.where((span) => span.text == 'music').toList();
    final musiciansSpan = wordSpans.singleWhere(
      (span) => span.text == 'musicians',
    );
    expect(musicSpans, hasLength(2));
    expect(
      musicSpans.every(
        (span) => span.style?.backgroundColor == const Color(0xFFFFDF70),
      ),
      isTrue,
    );
    expect(musiciansSpan.style?.backgroundColor, isNull);
  });

  testWidgets('single tap clears an existing yellow word mark', (tester) async {
    final bridge = _ExamWordBridge()
      ..marked = true
      ..inspectedUserMark = 'familiar';
    final client = ExamPracticeClient(bridge, const BridgeCodec());
    var meanings = <String, String>{'habit': 'habit meaning'};
    var marks = <String, String>{'habit': 'familiar'};
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
              currentMarks: marks,
              onInspectionChanged: (inspection) => setState(() {
                meanings = inspection.isUnknown
                    ? {inspection.normalized: inspection.meanings.join('; ')}
                    : {};
                marks = inspection.isUnknown
                    ? {inspection.normalized: bridge.inspectedUserMark!}
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

    expect(bridge.inspectedUserMark, 'none');
    expect(meanings, isEmpty);
    expect(marks, isEmpty);
    final selectable = tester.widget<SelectableText>(
      find.byType(SelectableText).first,
    );
    final tokenSpan = selectable.textSpan!.children!.first as TextSpan;
    expect(tokenSpan.style?.backgroundColor, isNull);
  });

  testWidgets('long press selection resolves a phrase meaning directly', (
    tester,
  ) async {
    final bridge = _PhraseSelectionBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 320,
              child: ExamInteractiveText(
                client: client,
                text: 'in case that happens',
                articleId: 'paper:section',
                title: 'Reading',
                metadata: const {'scope': 'passage'},
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final editable = tester.state<EditableTextState>(find.byType(EditableText));
    editable.userUpdateTextEditingValue(
      editable.textEditingValue.copyWith(
        selection: const TextSelection(baseOffset: 0, extentOffset: 12),
      ),
      SelectionChangedCause.longPress,
    );
    await tester.pump(const Duration(milliseconds: 350));
    await tester.pumpAndSettle();

    expect(find.text('in case that'), findsOneWidget);
    expect(find.text('万一；如果'), findsOneWidget);
    expect(find.text('标记为不会'), findsOneWidget);
    expect(find.text('标记词组'), findsNothing);
    expect(find.text('黄色标记 / 笔记'), findsNothing);
  });

  testWidgets(
    'phrase meaning sheet can persist the selected range as unknown',
    (tester) async {
      final bridge = _PhraseSelectionBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ExamInteractiveText(
              client: client,
              text: 'in case that happens',
              articleId: 'paper:section',
              title: 'Reading',
              metadata: const {'scope': 'passage'},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final editable = tester.state<EditableTextState>(
        find.byType(EditableText),
      );
      editable.userUpdateTextEditingValue(
        editable.textEditingValue.copyWith(
          selection: const TextSelection(baseOffset: 0, extentOffset: 12),
        ),
        SelectionChangedCause.longPress,
      );
      await tester.pump(const Duration(milliseconds: 350));
      await tester.pumpAndSettle();
      expect(find.text('in case that'), findsOneWidget);
      expect(find.text('万一；如果'), findsOneWidget);

      await tester.tap(find.text('标记为不会'));
      await tester.pumpAndSettle();
      expect(bridge.savedAnnotation?['selectedText'], 'in case that');
      expect(bridge.inspectedWord, 'in case that');
      expect(bridge.inspectedUserMark, 'unknown');
    },
  );

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

  testWidgets('saved attempt restoration blocks destructive answer changes', (
    tester,
  ) async {
    final bridge = _DelayedAttemptRestoreBridge();
    final client = ExamPracticeClient(bridge, const BridgeCodec());
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
      explanation: '',
      capabilities: const ExamQuestionCapabilities(
        browsable: true,
        answerable: true,
        autoGradable: true,
        causalAnalyzable: true,
      ),
    );
    final paper = ExamPaper(
      id: 'paper-1',
      exam: 'kaoyan-english-1',
      title: 'Paper',
      year: 2010,
      month: null,
      setNumber: null,
      sections: [
        ExamSection(
          id: 'reading',
          type: 'reading',
          title: 'Reading',
          instructions: '',
          passage: 'Passage.',
          questions: [question],
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
    await tester.pump();

    expect(find.text('正在恢复作答记录…'), findsOneWidget);
    await tester.tap(find.text('Second'), warnIfMissed: false);
    await tester.pump();
    expect(bridge.saveAttemptCalls, 0);

    bridge.completeRestore();
    await tester.pumpAndSettle();
    expect(find.text('查看大题报告'), findsOneWidget);
    expect(bridge.saveAttemptCalls, 0);
  });
}

class _DelayedAttemptRestoreBridge extends RustBridge {
  final Completer<String> _attempt = Completer<String>();
  int saveAttemptCalls = 0;

  void completeRestore() {
    _attempt.complete(
      jsonEncode({
        'attempt': {
          'attemptId': 'exam:paper-1:q1',
          'paperId': 'paper-1',
          'sectionId': 'reading',
          'questionId': 'q1',
          'selectedAnswer': 'B',
          'isCorrect': false,
          'answerHistory': ['B'],
          'status': 'answered',
        },
      }),
    );
  }

  @override
  Future<String> call(String method, [String? argument]) async {
    switch (method) {
      case 'getExamAttempt':
        return _attempt.future;
      case 'saveExamAttempt':
        saveAttemptCalls += 1;
        throw StateError('restore guard failed');
      case 'tokenizeExamText':
        return jsonEncode({'tokens': []});
      case 'getExamAnnotationState':
        return jsonEncode({
          'currentMarks': [],
          'priorMarks': [],
          'annotations': [],
        });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _ExamWordBridge extends RustBridge {
  bool marked = false;
  String? inspectedUserMark;

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
        inspectedUserMark = mark;
        marked = const {'fuzzy', 'familiar', 'unknown'}.contains(mark);
      }
      return jsonEncode({
        'occurrenceId': 1,
        'entryId': 2,
        'word': 'Habit',
        'normalized': 'habit',
        'meanings': ['habit meaning'],
        'userMark': marked ? inspectedUserMark : 'none',
        'isUnknown': marked,
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _FailedTokenizationBridge extends RustBridge {
  String? inspectedWord;
  String? inspectedUserMark;

  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'tokenizeExamText') {
      throw StateError('tokenizer unavailable');
    }
    if (method == 'inspectExamWord') {
      final request = jsonDecode(argument!) as Map<String, dynamic>;
      inspectedWord = request['word'] as String?;
      inspectedUserMark = request['userMark'] as String?;
      return jsonEncode({
        'occurrenceId': 1,
        'entryId': null,
        'word': inspectedWord,
        'normalized': inspectedWord?.toLowerCase(),
        'meanings': const <String>[],
        'userMark': inspectedUserMark,
        'isUnknown': true,
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _PerWordHitBridge extends RustBridge {
  String? inspectedWord;
  String? inspectedUserMark;

  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'tokenizeExamText') {
      final request = jsonDecode(argument!) as Map<String, dynamic>;
      final tokens = tokenizeExamTextForInteraction(request['text'] as String);
      return jsonEncode({
        'tokens': tokens
            .map(
              (token) => {
                'text': token.text,
                'normalized': token.normalized,
                'startOffset': token.startOffset,
                'endOffset': token.endOffset,
              },
            )
            .toList(),
      });
    }
    if (method == 'inspectExamWord') {
      final request = jsonDecode(argument!) as Map<String, dynamic>;
      inspectedWord = request['word'] as String?;
      inspectedUserMark = request['userMark'] as String?;
      return jsonEncode({
        'occurrenceId': 1,
        'entryId': null,
        'word': inspectedWord,
        'normalized': inspectedWord,
        'meanings': const <String>[],
        'userMark': inspectedUserMark,
        'isUnknown': true,
      });
    }
    throw StateError('Unexpected bridge method: $method');
  }
}

class _PhraseSelectionBridge extends RustBridge {
  Map<String, dynamic>? savedAnnotation;
  String? inspectedWord;
  String? inspectedUserMark;

  @override
  Future<String> call(String method, [String? argument]) async {
    if (method == 'tokenizeExamText') {
      return jsonEncode({
        'tokens': [
          {'text': 'in', 'normalized': 'in', 'startOffset': 0, 'endOffset': 2},
          {
            'text': 'case',
            'normalized': 'case',
            'startOffset': 3,
            'endOffset': 7,
          },
          {
            'text': 'that',
            'normalized': 'that',
            'startOffset': 8,
            'endOffset': 12,
          },
          {
            'text': 'happens',
            'normalized': 'happens',
            'startOffset': 13,
            'endOffset': 20,
          },
        ],
      });
    }
    if (method == 'saveExamAnnotation') {
      savedAnnotation = jsonDecode(argument!) as Map<String, dynamic>;
      return jsonEncode({
        'annotations': [
          {
            'annotationId': savedAnnotation!['annotationId'],
            'questionId': savedAnnotation!['questionId'],
            'scope': savedAnnotation!['scope'],
            'startOffset': savedAnnotation!['startOffset'],
            'endOffset': savedAnnotation!['endOffset'],
            'selectedText': savedAnnotation!['selectedText'],
            'noteText': savedAnnotation!['noteText'],
            'color': 'yellow',
          },
        ],
      });
    }
    if (method == 'inspectExamWord') {
      final request = jsonDecode(argument!) as Map<String, dynamic>;
      inspectedWord = request['word'] as String?;
      inspectedUserMark = request['userMark'] as String?;
      final isUnknown =
          inspectedUserMark == 'unknown' || inspectedUserMark == 'uncertain';
      return jsonEncode({
        'occurrenceId': 1,
        'entryId': 2,
        'word': inspectedWord,
        'normalized': inspectedWord,
        'meanings': ['万一；如果'],
        'userMark': isUnknown ? inspectedUserMark : 'none',
        'isUnknown': isUnknown,
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

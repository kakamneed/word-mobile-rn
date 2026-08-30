import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/study_screen.dart';
import 'package:flutter_mobile/sdk/exam_practice_client.dart';
import 'package:flutter_mobile/sdk/study_client.dart';
import 'package:flutter_mobile/sdk/wrong_words_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('cnToEn hero displays the prompt instead of the English answer', () {
    const question = StudyQuestion(
      questionId: 'q1',
      questionType: 'cnToEnChoice',
      entrySourceId: 'surgeon',
      word: 'surgeon',
      prompt: 'doctor who performs operations',
      partOfSpeech: 'n',
      acceptedMeanings: ['doctor who performs operations'],
      choices: [
        {'label': 'A', 'text': 'wreck'},
        {'label': 'B', 'text': 'rabbit'},
        {'label': 'C', 'text': 'reproach'},
        {'label': 'D', 'text': 'surgeon'},
      ],
      correctChoiceLabel: 'D',
      questionIndex: 10,
      totalQuestions: 20,
    );

    final display = studyHeroDisplayForTest(question);

    expect(display.title, 'doctor who performs operations');
    expect(display.subtitle, isNull);
    expect(display.title, isNot('surgeon'));
  });

  test('non cnToEn hero keeps the studied word and part of speech', () {
    const question = StudyQuestion(
      questionId: 'q2',
      questionType: 'enToCnChoice',
      entrySourceId: 'abrupt',
      word: 'abrupt',
      prompt: 'abrupt',
      partOfSpeech: 'adj',
      acceptedMeanings: ['sudden'],
      questionIndex: 3,
      totalQuestions: 20,
    );

    final display = studyHeroDisplayForTest(question);

    expect(display.title, 'abrupt');
    expect(display.subtitle, 'adj');
  });

  testWidgets('every drawn study word shows its cumulative error count', (
    tester,
  ) async {
    const question = StudyQuestion(
      questionId: 'q-error-count',
      questionType: 'enToCnInput',
      entrySourceId: 'available',
      word: 'available',
      prompt: 'available',
      acceptedMeanings: ['可用的'],
      questionIndex: 0,
      totalQuestions: 1,
      errorCount: 7,
    );

    await tester.pumpWidget(
      MaterialApp(home: Scaffold(body: studyQuestionHeaderForTest(question))),
    );

    expect(find.text('错误 7 次'), findsOneWidget);
  });

  test(
    'English-to-Chinese input does not repeat the studied word as a prompt',
    () {
      expect(shouldShowStudyPromptForTest('enToCnInput'), isFalse);
      expect(shouldShowStudyPromptForTest('enToCnChoice'), isTrue);
    },
  );

  test('only a clicked high-frequency hint suppresses mastery credit', () {
    expect(
      hintUsedForStudySubmitForTest(
        mode: 'highFrequency',
        questionId: 'q-hinted',
        clickedQuestionIds: {'q-hinted'},
      ),
      isTrue,
    );
    expect(
      hintUsedForStudySubmitForTest(
        mode: 'highFrequency',
        questionId: 'q-not-clicked',
        clickedQuestionIds: {'q-hinted'},
      ),
      isFalse,
    );
    expect(
      hintUsedForStudySubmitForTest(
        mode: 'review',
        questionId: 'q-hinted',
        clickedQuestionIds: {'q-hinted'},
      ),
      isFalse,
    );
  });

  testWidgets(
    'high-frequency related-meaning drawers are collapsed until opened',
    (tester) async {
      final question = StudyQuestion.fromJson({
        'questionId': 'q-related-drawers',
        'questionType': 'enToCnInput',
        'entrySourceId': 'assist',
        'word': 'assist',
        'prompt': 'assist',
        'acceptedMeanings': ['帮助'],
        'questionIndex': 0,
        'totalQuestions': 1,
        'derivationalFamily': [
          {'word': 'assistance', 'meaning': '帮助；援助'},
        ],
        'synonymGroups': [
          {
            'meaning': '帮助',
            'words': ['aid'],
          },
        ],
      });

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: studyHighFrequencyMeaningDrawersForTest(question),
          ),
        ),
      );

      expect(find.text('同源词组'), findsOneWidget);
      expect(find.text('近义词'), findsOneWidget);
      expect(find.text('assistance'), findsNothing);
      expect(find.text('aid'), findsNothing);

      await tester.tap(find.text('同源词组'));
      await tester.pumpAndSettle();
      expect(find.text('assistance'), findsOneWidget);

      await tester.tapAt(const Offset(8, 8));
      await tester.pumpAndSettle();

      await tester.tap(find.text('近义词'));
      await tester.pumpAndSettle();
      expect(find.text('aid'), findsOneWidget);
    },
  );

  test('synonym feedback is limited to answered high-frequency questions', () {
    const question = StudyQuestion(
      questionId: 'q-synonym-mode',
      questionType: 'enToCnInput',
      entrySourceId: 'assist',
      word: 'assist',
      prompt: 'assist',
      acceptedMeanings: ['帮助'],
      questionIndex: 0,
      totalQuestions: 1,
      synonymGroups: [
        StudySynonymGroup(meaning: '帮助', words: ['aid']),
      ],
    );

    expect(
      shouldShowStudySynonymsForTest(
        question: question,
        mode: 'highFrequency',
        answered: true,
      ),
      isTrue,
    );
    expect(
      shouldShowStudySynonymsForTest(
        question: question,
        mode: 'highFrequency',
        answered: false,
      ),
      isFalse,
    );
    expect(
      shouldShowStudySynonymsForTest(
        question: question,
        mode: 'newWord',
        answered: true,
      ),
      isFalse,
    );
  });

  test(
    'only high-frequency ordinary questions reveal supplemental exam examples',
    () {
      const question = StudyQuestion(
        questionId: 'segment-q',
        questionType: 'enToCnChoice',
        entrySourceId: 'segment',
        word: 'segment',
        prompt: 'segment',
        acceptedMeanings: ['段；片；部分'],
        exampleSentence: 'Demand from the food service segment grew.',
        exampleTranslation: '真题来源：KAOYAN-ENGLISH-1 / Kaoyan English I 2010',
        questionIndex: 0,
        totalQuestions: 1,
      );

      expect(
        shouldShowSupplementalExamExampleForTest(
          question: question,
          mode: 'highFrequency',
          answered: false,
        ),
        isFalse,
      );
      expect(
        shouldShowSupplementalExamExampleForTest(
          question: question,
          mode: 'highFrequency',
          answered: true,
        ),
        isTrue,
      );
      expect(
        shouldShowSupplementalExamExampleForTest(
          question: question,
          mode: 'newWord',
          answered: true,
        ),
        isFalse,
      );
    },
  );

  test('dedicated example questions keep their sentence as the prompt', () {
    const question = StudyQuestion(
      questionId: 'example-q',
      questionType: 'exampleToCnChoice',
      entrySourceId: 'survive',
      word: 'survive',
      prompt: 'survive',
      acceptedMeanings: ['生存'],
      exampleSentence: 'Only the strongest plants survive.',
      questionIndex: 0,
      totalQuestions: 1,
    );

    expect(
      studyDedicatedExampleSentenceForTest(question),
      'Only the strongest plants survive.',
    );
  });

  test('high-frequency answers expose parsed derivational-family meanings', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'observe-q',
      'questionType': 'enToCnInput',
      'entrySourceId': 'observe',
      'word': 'observe',
      'prompt': 'observe',
      'acceptedMeanings': ['观察；遵守'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'derivationalFamily': [
        {'word': 'observation', 'meaning': '观察；观察结果'},
        {'word': 'observer', 'meaning': '观察者'},
      ],
    });

    expect(question.derivationalFamily, hasLength(2));
    expect(question.derivationalFamily.first.word, 'observation');
    expect(question.derivationalFamily.first.meaning, '观察；观察结果');
    expect(
      shouldShowStudyDerivationalFamilyForTest(
        question: question,
        mode: 'highFrequency',
        answered: false,
      ),
      isFalse,
    );
    expect(
      shouldShowStudyDerivationalFamilyForTest(
        question: question,
        mode: 'highFrequency',
        answered: true,
      ),
      isTrue,
    );
    expect(
      shouldShowStudyDerivationalFamilyForTest(
        question: question,
        mode: 'review',
        answered: true,
      ),
      isFalse,
    );
  });

  testWidgets('derivational family is a bounded internally scrollable card', (
    tester,
  ) async {
    final question = StudyQuestion.fromJson({
      'questionId': 'family-card-q',
      'questionType': 'enToCnInput',
      'entrySourceId': 'accept',
      'word': 'accept',
      'prompt': 'accept',
      'acceptedMeanings': ['接受'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'derivationalFamily': List.generate(
        20,
        (index) => {'word': 'accept$index', 'meaning': '释义$index'},
      ),
    });

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: studyDerivationalFamilyCardForTest(question)),
      ),
    );

    expect(
      find.byKey(const ValueKey('study-derivational-family-card')),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey('study-derivational-family-card')),
        matching: find.byType(Scrollable),
      ),
      findsOneWidget,
    );
    expect(
      tester
          .getSize(find.byKey(const ValueKey('study-derivational-family-card')))
          .height,
      lessThanOrEqualTo(studyDerivationalFamilyCardMaxHeightForTest),
    );
  });

  testWidgets('derivational family merges repeated accepted and real rows', (
    tester,
  ) async {
    final question = StudyQuestion.fromJson({
      'questionId': 'deduped-family-card-q',
      'questionType': 'enToCnInput',
      'entrySourceId': 'accepted',
      'word': 'accepted',
      'prompt': 'accepted',
      'acceptedMeanings': ['公认的', '录取的'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'derivationalFamily': [
        {'word': 'accepted', 'meaning': '公认的'},
        {'word': 'Accepted', 'meaning': '已承兑的'},
        {'word': 'real', 'meaning': '实际的'},
        {'word': 'real', 'meaning': '现实'},
        {'word': 'REAL', 'meaning': '实际的'},
      ],
    });

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(body: studyDerivationalFamilyCardForTest(question)),
      ),
    );

    expect(find.text('accepted'), findsOneWidget);
    expect(find.text('real'), findsOneWidget);
    expect(find.text('公认的；录取的；已承兑的'), findsOneWidget);
    expect(find.text('实际的；现实'), findsOneWidget);
  });

  testWidgets(
    'answered high-frequency example and long family have finite layout',
    (tester) async {
      final bridge = _StudyExampleBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());
      final controller = ScrollController();
      final answerController = TextEditingController();
      final answerFocusNode = FocusNode();
      addTearDown(controller.dispose);
      addTearDown(answerController.dispose);
      addTearDown(answerFocusNode.dispose);
      final question = StudyQuestion.fromJson({
        'questionId': 'available-q',
        'questionType': 'enToCnInput',
        'entrySourceId': 'available',
        'word': 'available',
        'prompt': 'available',
        'acceptedMeanings': ['可用的；可得到的；可以见到的；随时可来的'],
        'exampleSentence':
            'These services are available to every qualified applicant.',
        'exampleTranslation':
            '真题来源：KAOYAN-ENGLISH-1 / Kaoyan English I 2012 / 阅读Text1',
        'questionIndex': 17,
        'totalQuestions': 100,
        'derivationalFamily': List.generate(
          30,
          (index) => {
            'word': 'available$index',
            'meaning': '同源词释义$index；补充释义$index',
          },
        ),
      });
      const result = StudyResult(
        questionId: 'available-q',
        entrySourceId: 'available',
        questionType: 'enToCnInput',
        userResponse: '买得起的',
        correctAnswer: '可用的；可得到的；可以见到的；随时可来的',
        outcome: AnswerOutcome.incorrect,
        responseTimeMs: 100,
        answeredAt: '2026-08-15T00:00:00Z',
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: studyAnsweredHighFrequencyContentForTest(
              question: question,
              result: result,
              examPracticeClient: client,
              scrollController: controller,
              answerController: answerController,
              answerFocusNode: answerFocusNode,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(tester.takeException(), isNull);
      expect(controller.position.maxScrollExtent.isFinite, isTrue);
      final drawer = find.byKey(
        const ValueKey('study-derivational-family-drawer'),
      );
      await tester.ensureVisible(drawer);
      await tester.tap(drawer);
      await tester.pumpAndSettle();
      expect(
        tester
            .getSize(
              find.byKey(const ValueKey('study-derivational-family-card')),
            )
            .height,
        lessThanOrEqualTo(studyDerivationalFamilyCardMaxHeightForTest),
      );
    },
  );

  test('study example marks use a shared three-level blue palette', () {
    expect(studyExampleMarkColorForTest('fuzzy'), const Color(0xFFDCEEFF));
    expect(studyExampleMarkColorForTest('familiar'), const Color(0xFFA9D4FF));
    expect(studyExampleMarkColorForTest('unknown'), const Color(0xFF72B7F2));
    expect(studyExampleMarkColorForTest('wrong'), const Color(0xFF72B7F2));
  });

  testWidgets(
    'study example loads practice marks and persists a selected blue level',
    (tester) async {
      final bridge = _StudyExampleBridge();
      final client = ExamPracticeClient(bridge, const BridgeCodec());
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StudyInteractiveExampleSentence(
              client: client,
              sentence: 'We observe change.',
              word: 'observe',
              entrySourceId: 'observe',
              mode: 'highFrequency',
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is ColoredBox &&
              widget.color == studyExampleMarkColorForTest('unknown'),
        ),
        findsOneWidget,
      );

      await tester.tap(find.text('observe'));
      await tester.pumpAndSettle();
      expect(find.text('观察；遵守'), findsOneWidget);

      await tester.tap(find.text('眼熟'));
      await tester.pumpAndSettle();
      expect(bridge.lastUserMark, 'familiar');
      expect(bridge.lastArticleId, 'study-example:observe');
      expect(
        find.byWidgetPredicate(
          (widget) =>
              widget is ColoredBox &&
              widget.color == studyExampleMarkColorForTest('familiar'),
        ),
        findsOneWidget,
      );
    },
  );

  testWidgets('study example accepts native UTF-8 offsets for Unicode text', (
    tester,
  ) async {
    final client = ExamPracticeClient(
      _Utf8OffsetStudyExampleBridge(),
      const BridgeCodec(),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StudyInteractiveExampleSentence(
            client: client,
            sentence: '汉汉汉汉汉汉 available.',
            word: 'available',
            entrySourceId: 'available',
            mode: 'highFrequency',
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('available'), findsOneWidget);
  });

  test(
    'word skeleton hero displays the masked prompt instead of the full word',
    () {
      const question = StudyQuestion(
        questionId: 'q5',
        questionType: 'wordSkeletonInput',
        entrySourceId: 'architect',
        word: 'architect',
        prompt: 'a___itect',
        partOfSpeech: 'n',
        acceptedMeanings: ['architect'],
        questionIndex: 4,
        totalQuestions: 12,
      );

      final display = studyHeroDisplayForTest(question);

      expect(display.title, 'a_ _ _itect');
      expect(display.title, isNot('architect'));
      expect(display.subtitle, 'n');
    },
  );

  test('word skeleton display fills typed letters into underscores', () {
    expect(wordSkeletonDisplayForTest('f__dge', ''), 'f_ _dge');
    expect(wordSkeletonDisplayForTest('f__dge', 'r'), 'fr _dge');
    expect(wordSkeletonDisplayForTest('f__dge', 'ri'), 'fridge');
    expect(wordSkeletonDisplayForTest('f__dge', 'ride'), 'fridge');
    expect(wordSkeletonDisplayForTest('e___ution', ''), 'e_ _ _ution');
  });

  test('study submit gate rejects duplicate in-flight submissions', () {
    expect(
      shouldAcceptStudySubmitForTest(
        submitting: false,
        activeQuestionId: null,
        questionId: 'q1',
      ),
      isTrue,
    );
    expect(
      shouldAcceptStudySubmitForTest(
        submitting: true,
        activeQuestionId: 'q1',
        questionId: 'q1',
      ),
      isFalse,
    );
    expect(
      shouldAcceptStudySubmitForTest(
        submitting: false,
        activeQuestionId: 'q1',
        questionId: 'q1',
      ),
      isFalse,
    );
  });

  test('input submit hides keyboard independent of focus or bridge state', () {
    expect(
      shouldSettleAnswerInputBeforeSubmitForTest(
        hasFocus: true,
        isChoiceType: false,
      ),
      isTrue,
    );
    expect(
      shouldSettleAnswerInputBeforeSubmitForTest(
        hasFocus: true,
        isChoiceType: true,
      ),
      isFalse,
    );
    expect(
      shouldSettleAnswerInputBeforeSubmitForTest(
        hasFocus: false,
        isChoiceType: false,
      ),
      isTrue,
    );
    expect(inputAnswerFieldReadOnlyWhileSubmittingForTest(), isFalse);
    expect(shouldAutoOpenKeyboardForStudyInputForTest(), isTrue);
    expect(
      keyboardInsetSpacerHeightForTest(baseHeight: 24, bottomInset: 320),
      24,
    );
  });

  test('mastered action remains available after an answer is submitted', () {
    expect(
      canMarkStudyItemMasteredForTest(
        submitting: false,
        isAnswered: true,
        isActiveQuestion: false,
      ),
      isTrue,
    );
    expect(
      canMarkStudyItemMasteredForTest(
        submitting: false,
        isAnswered: false,
        isActiveQuestion: false,
      ),
      isFalse,
    );
    expect(
      canMarkStudyItemMasteredForTest(
        submitting: true,
        isAnswered: true,
        isActiveQuestion: false,
      ),
      isFalse,
    );
  });

  test('mastered action gives visible success feedback after submission', () {
    expect(
      studyMasteredSuccessMessageForTest(wasAnswered: true),
      '\u5df2\u6807\u8bb0\u4e3a\u638c\u63e1\uff0c\u5df2\u4fdd\u7559\u672c\u9898\u7b54\u9898\u8bb0\u5f55',
    );
    expect(
      masteredActionIconForTest(isMastered: true),
      Icons.check_circle_outline,
    );
    expect(
      canMarkStudyItemMasteredForTest(
        submitting: false,
        isAnswered: true,
        isActiveQuestion: false,
        isMastered: true,
      ),
      isFalse,
    );
    expect(
      studyMasteredSuccessMessageForTest(wasAnswered: false),
      '\u5df2\u6807\u8bb0\u4e3a\u638c\u63e1',
    );
  });

  test('dispute failures stay in the current study flow', () {
    expect(
      studyActionFailureMessageForTest('dispute'),
      '\u5f02\u8bae\u63d0\u4ea4\u5931\u8d25\uff0c\u672c\u8f6e\u5b66\u4e60\u5df2\u4fdd\u7559',
    );
    expect(studyActionFailureMessageForTest('dispute'), isNot('Study error'));
  });

  test('dispute success is queued through the native sync path', () {
    expect(
      studyDisputeAcceptedMessageForTest(),
      '\u5df2\u63a5\u53d7\u5f02\u8bae\uff0c\u5df2\u52a0\u5165\u540c\u6b65\u961f\u5217',
    );
  });

  test('pending next question disables automatic page jump', () {
    const pending = StudyQuestion(
      questionId: 'q-pending',
      questionType: 'enToCnChoice',
      entrySourceId: 'debate',
      word: 'debate',
      prompt: 'debate',
      partOfSpeech: 'n',
      acceptedMeanings: ['debate'],
      questionIndex: 2,
      totalQuestions: 20,
    );

    expect(shouldAutoJumpToCurrentQuestionForTest(null), isTrue);
    expect(shouldAutoJumpToCurrentQuestionForTest(pending), isFalse);
    expect(
      shouldAutoJumpToCurrentQuestionWithVisiblePendingForTest(
        pendingNextQuestion: null,
        pendingNextVisible: true,
      ),
      isFalse,
    );
  });

  testWidgets(
    'outer bottom overscroll advances after reaching the finite content end',
    (tester) async {
      var advanceCount = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: studyAdvanceGestureRegionForTest(
              onAdvance: () => advanceCount++,
              child: const SingleChildScrollView(
                key: ValueKey('study-long-content-scroll'),
                child: SizedBox(width: 400, height: 1200),
              ),
            ),
          ),
        ),
      );

      await tester.drag(
        find.byKey(const ValueKey('study-long-content-scroll')),
        const Offset(0, -96),
      );
      await tester.pump();
      expect(advanceCount, 0);

      final scrollable = tester.state<ScrollableState>(
        find.descendant(
          of: find.byKey(const ValueKey('study-long-content-scroll')),
          matching: find.byType(Scrollable),
        ),
      );
      scrollable.position.jumpTo(scrollable.position.maxScrollExtent);
      await tester.pump();
      final drag = await tester.startGesture(
        tester.getCenter(
          find.byKey(const ValueKey('study-long-content-scroll')),
        ),
      );
      await drag.moveBy(
        const Offset(0, -96),
        timeStamp: const Duration(milliseconds: 240),
      );
      await drag.up(timeStamp: const Duration(milliseconds: 280));
      await tester.pump();
      expect(advanceCount, 1);
    },
  );

  testWidgets('short answered page can overscroll into the pending question', (
    tester,
  ) async {
    var advanceCount = 0;
    Widget buildPage({required bool submitted}) => MaterialApp(
      home: Scaffold(
        body: studyAdvanceGestureRegionForTest(
          enabled: submitted,
          onAdvance: () => advanceCount++,
          child: SingleChildScrollView(
            key: const ValueKey('study-short-answered-scroll'),
            physics: studyQuestionScrollPhysicsForTest(
              continuationEnabled: submitted,
            ),
            child: const SizedBox(width: 400, height: 240),
          ),
        ),
      ),
    );

    await tester.pumpWidget(buildPage(submitted: false));
    await tester.drag(
      find.byKey(const ValueKey('study-short-answered-scroll')),
      const Offset(0, -96),
    );
    await tester.pump();
    expect(advanceCount, 0);

    await tester.pumpWidget(buildPage(submitted: true));
    await tester.pump();

    await tester.drag(
      find.byKey(const ValueKey('study-short-answered-scroll')),
      const Offset(0, -96),
    );
    await tester.pump();

    expect(advanceCount, 1);
  });

  testWidgets('nested derivational scrolling never advances the study page', (
    tester,
  ) async {
    var advanceCount = 0;
    final outerController = ScrollController();
    addTearDown(outerController.dispose);
    final question = StudyQuestion.fromJson({
      'questionId': 'nested-family-q',
      'questionType': 'enToCnInput',
      'entrySourceId': 'accept',
      'word': 'accept',
      'prompt': 'accept',
      'acceptedMeanings': ['接受'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'derivationalFamily': List.generate(
        20,
        (index) => {'word': 'accept$index', 'meaning': '释义$index'},
      ),
    });
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: studyAdvanceGestureRegionForTest(
            onAdvance: () => advanceCount++,
            child: SingleChildScrollView(
              key: const ValueKey('study-outer-nested-scroll'),
              controller: outerController,
              child: SizedBox(
                width: 400,
                child: Column(
                  children: [
                    const SizedBox(height: 700),
                    studyDerivationalFamilyCardForTest(question),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );

    outerController.jumpTo(outerController.position.maxScrollExtent);
    await tester.pump();
    await tester.drag(
      find.byKey(const ValueKey('study-derivational-family-card')),
      const Offset(0, -96),
    );
    await tester.pump();

    expect(advanceCount, 0);
  });

  test('pending next question is inserted only during user transition', () {
    const current = StudyQuestion(
      questionId: 'q-current',
      questionType: 'enToCnChoice',
      entrySourceId: 'current',
      word: 'current',
      prompt: 'current',
      acceptedMeanings: ['current'],
      questionIndex: 20,
      totalQuestions: 28,
    );
    const pending = StudyQuestion(
      questionId: 'q-pending',
      questionType: 'enToCnChoice',
      entrySourceId: 'pending',
      word: 'pending',
      prompt: 'pending',
      acceptedMeanings: ['pending'],
      questionIndex: 21,
      totalQuestions: 28,
    );
    final answered = List.generate(
      20,
      (index) => AnsweredStudyQuestion(
        question: StudyQuestion(
          questionId: 'q$index',
          questionType: 'enToCnChoice',
          entrySourceId: 'word$index',
          word: 'word$index',
          prompt: 'word$index',
          acceptedMeanings: const ['word'],
          questionIndex: index,
          totalQuestions: 28,
        ),
        result: StudyResult(
          questionId: 'q$index',
          entrySourceId: 'word$index',
          questionType: 'enToCnChoice',
          userResponse: 'A',
          correctAnswer: 'A',
          outcome: AnswerOutcome.correct,
          responseTimeMs: 100,
          answeredAt: '2026-07-17T00:00:00Z',
        ),
      ),
    );
    final submittedCurrent = AnsweredStudyQuestion(
      question: current,
      result: const StudyResult(
        questionId: 'q-current',
        entrySourceId: 'current',
        questionType: 'enToCnChoice',
        userResponse: 'A',
        correctAnswer: 'A',
        outcome: AnswerOutcome.correct,
        responseTimeMs: 100,
        answeredAt: '2026-07-17T00:00:01Z',
      ),
    );

    final feed = [...answered, submittedCurrent];

    expect(
      shouldInsertPendingNextPageForTest(
        hasPendingNext: true,
        pendingNextVisible: false,
      ),
      isFalse,
    );
    expect(
      shouldInsertPendingNextPageForTest(
        hasPendingNext: true,
        pendingNextVisible: true,
      ),
      isTrue,
    );
    expect(
      questionFeedIndexForTest(
        answeredQuestions: feed,
        currentQuestion: current,
        pendingNextQuestion: pending,
        questionId: 'q-current',
      ),
      20,
    );
    expect(
      questionFeedIndexForTest(
        answeredQuestions: feed,
        currentQuestion: current,
        pendingNextQuestion: pending,
        questionId: 'q-pending',
      ),
      -1,
    );
    expect(
      questionFeedIndexForTest(
        answeredQuestions: feed,
        currentQuestion: current,
        pendingNextQuestion: pending,
        pendingNextVisible: true,
        questionId: 'q-pending',
      ),
      21,
    );
  });

  test('completion page is inserted only after the user asks for it', () {
    expect(
      shouldInsertCompletionPageForTest(
        hasCompletion: false,
        completionVisible: false,
      ),
      isFalse,
    );
    expect(
      shouldInsertCompletionPageForTest(
        hasCompletion: true,
        completionVisible: false,
      ),
      isFalse,
    );
    expect(
      shouldInsertCompletionPageForTest(
        hasCompletion: true,
        completionVisible: true,
      ),
      isTrue,
    );
  });

  test('study start skips plan weights for resume-fixed modes', () {
    expect(modeUsesQuestionTypeWeightsForTest('newWord'), isFalse);
    expect(modeUsesQuestionTypeWeightsForTest('rootAffix'), isFalse);
    expect(modeUsesQuestionTypeWeightsForTest('review'), isTrue);
    expect(modeUsesQuestionTypeWeightsForTest('mixedTest'), isTrue);
    expect(
      modeUsesQuestionTypeWeightsForTest('wrongWordReinforcement'),
      isTrue,
    );
  });

  test(
    'choice display falls back to unique labels when bridge fields are sparse',
    () {
      final first = choiceDisplayForTest({'text': 'versatile'}, 0);
      final second = choiceDisplayForTest({'text': 'commonplace'}, 1);

      expect(first.value, 'A');
      expect(first.label, 'A');
      expect(first.text, 'versatile');
      expect(second.value, 'B');
      expect(second.label, 'B');
      expect(second.text, 'commonplace');
      expect(first.value, isNot(second.value));
    },
  );

  test('hero hint chip does not show for ai suggestions without user hint', () {
    const question = StudyQuestion(
      questionId: 'q3',
      questionType: 'cnToEnChoice',
      entrySourceId: 'commonplace',
      word: 'commonplace',
      prompt: 'ordinary',
      acceptedMeanings: ['ordinary'],
      questionIndex: 1,
      totalQuestions: 3,
      hasHint: false,
      hintSuggestions: [
        WordHintSuggestion(
          id: 'h1',
          word: 'commonplace',
          style: 'mnemonic',
          label: 'link',
          text: 'common + place',
        ),
      ],
    );

    expect(shouldShowHeroHintChipForTest(question), isFalse);
  });

  test('hero hint chip shows when the user has saved a hint', () {
    const question = StudyQuestion(
      questionId: 'q4',
      questionType: 'enToCnChoice',
      entrySourceId: 'commonplace',
      word: 'commonplace',
      prompt: 'commonplace',
      acceptedMeanings: ['ordinary'],
      questionIndex: 1,
      totalQuestions: 3,
      hasHint: true,
      userHint: 'common + place',
    );

    expect(shouldShowHeroHintChipForTest(question), isTrue);
  });

  test(
    'reader-marked study words are highlighted only when the word is visible',
    () {
      final marked = StudyQuestion.fromJson({
        'questionId': 'q-reader-marked',
        'questionType': 'enToCnChoice',
        'entrySourceId': 'consequential',
        'word': 'consequential',
        'prompt': 'consequential',
        'acceptedMeanings': ['important'],
        'choices': [
          {'label': 'A', 'text': 'important'},
          {'label': 'B', 'text': 'ordinary'},
        ],
        'correctChoiceLabel': 'A',
        'questionIndex': 1,
        'totalQuestions': 20,
        'examMarked': true,
        'examMarkLevel': 'unknown',
      });
      final hiddenWord = marked.copyWith();
      final hiddenDisplay = studyHeroDisplayForTest(
        StudyQuestion(
          questionId: marked.questionId,
          questionType: 'cnToEnChoice',
          entrySourceId: marked.entrySourceId,
          word: marked.word,
          prompt: 'important',
          acceptedMeanings: marked.acceptedMeanings,
          choices: marked.choices,
          correctChoiceLabel: marked.correctChoiceLabel,
          questionIndex: marked.questionIndex,
          totalQuestions: marked.totalQuestions,
          examMarked: true,
          examMarkLevel: 'unknown',
        ),
      );

      expect(marked.examMarked, isTrue);
      expect(marked.examMarkLevel, 'unknown');
      expect(shouldHighlightStudyHeroForTest(marked, marked.word), isTrue);
      expect(
        shouldHighlightStudyHeroForTest(hiddenWord, hiddenDisplay.title),
        isFalse,
      );
    },
  );

  test('hint action opens only for saved non-empty hints', () {
    const withoutHint = StudyQuestion(
      questionId: 'q8',
      questionType: 'enToCnChoice',
      entrySourceId: 'ambition',
      word: 'ambition',
      prompt: 'ambition',
      acceptedMeanings: ['雄心'],
      questionIndex: 17,
      totalQuestions: 20,
      hasHint: false,
    );
    const withSuggestion = StudyQuestion(
      questionId: 'q10',
      questionType: 'enToCnChoice',
      entrySourceId: 'ambition',
      word: 'ambition',
      prompt: 'ambition',
      acceptedMeanings: ['雄心'],
      questionIndex: 17,
      totalQuestions: 20,
      hasHint: false,
      hintSuggestions: [
        WordHintSuggestion(
          id: 'h1',
          word: 'ambition',
          style: 'mnemonic',
          label: 'link',
          text: 'aim + ambition',
        ),
      ],
    );
    const withHint = StudyQuestion(
      questionId: 'q9',
      questionType: 'enToCnChoice',
      entrySourceId: 'ambition',
      word: 'ambition',
      prompt: 'ambition',
      acceptedMeanings: ['雄心'],
      questionIndex: 17,
      totalQuestions: 20,
      hasHint: true,
      userHint: 'aim + ambition',
    );

    expect(questionHasHintForAction(withoutHint), isFalse);
    expect(questionHasHintForAction(withSuggestion), isFalse);
    expect(questionHasHintForAction(withHint), isTrue);
  });

  test(
    'round action active surface is disabled when the button is disabled',
    () {
      expect(
        roundActionButtonVisualActiveForTest(active: true, enabled: false),
        isFalse,
      );
      expect(
        roundActionButtonVisualActiveForTest(active: true, enabled: true),
        isTrue,
      );
      expect(
        roundActionButtonVisualActiveForTest(active: false, enabled: true),
        isFalse,
      );
    },
  );

  test('all active special question types have explicit labels', () {
    expect(
      questionLabelForTest('wordSkeletonInput'),
      '\u6839\u636e\u82f1\u6587\u8865\u5168\u7f3a\u5931\u5b57\u6bcd',
    );
    expect(
      questionLabelForTest('exampleToCnChoiceNoTranslation'),
      '\u6839\u636e\u82f1\u6587\u4f8b\u53e5\u9009\u62e9\u4e2d\u6587\u91ca\u4e49',
    );
    expect(questionLabelForTest('wordSkeletonInput'), isNot('回答问题'));
    expect(
      questionLabelForTest('exampleToCnChoiceNoTranslation'),
      isNot('回答问题'),
    );
  });

  test(
    'feed progress keeps the session total instead of visible item count',
    () {
      const question = StudyQuestion(
        questionId: 'q10',
        questionType: 'enToCnChoice',
        entrySourceId: 'ambition',
        word: 'ambition',
        prompt: 'ambition',
        acceptedMeanings: ['雄心'],
        questionIndex: 17,
        totalQuestions: 17,
      );

      final progress = studyFeedProgressForTest(
        visibleQuestion: question,
        visibleIndex: 17,
        itemCount: 17,
        sessionTotal: 20,
      );

      expect(progress.current, 18);
      expect(progress.total, 20);
    },
  );

  test('high-frequency progress uses the active session position', () {
    const question = StudyQuestion(
      questionId: 'q-hf-restart',
      questionType: 'enToCnInput',
      entrySourceId: 'obtain',
      word: 'obtain',
      prompt: 'obtain',
      acceptedMeanings: ['获得'],
      questionIndex: 0,
      totalQuestions: 100,
    );

    final progress = studyFeedProgressForTest(
      visibleQuestion: question,
      visibleIndex: 1,
      itemCount: 1,
      sessionTotal: 100,
    );

    expect(progress.current, 1);
    expect(progress.total, 100);
  });

  test(
    'feed progress falls back to current question when window index is stale',
    () {
      const question = StudyQuestion(
        questionId: 'q29',
        questionType: 'enToCnChoice',
        entrySourceId: 'word29',
        word: 'word29',
        prompt: 'word29',
        acceptedMeanings: ['meaning'],
        questionIndex: 29,
        totalQuestions: 34,
      );

      final progress = studyFeedProgressForTest(
        visibleQuestion: null,
        fallbackQuestion: question,
        visibleIndex: 20,
        itemCount: 20,
        sessionTotal: 34,
      );

      expect(progress.current, 30);
      expect(progress.total, 34);
    },
  );

  test('feed progress never exceeds a pruned session total', () {
    const staleQuestion = StudyQuestion(
      questionId: 'q77',
      questionType: 'enToCnInput',
      entrySourceId: 'security',
      word: 'security',
      prompt: 'security',
      acceptedMeanings: ['security meaning'],
      questionIndex: 76,
      totalQuestions: 80,
    );

    final progress = studyFeedProgressForTest(
      visibleQuestion: staleQuestion,
      fallbackQuestion: null,
      visibleIndex: 49,
      itemCount: 50,
      sessionTotal: 56,
    );

    expect(progress.current, 56);
    expect(progress.total, 56);
  });

  test(
    'unanswered mastered completion opens summary without an answer anchor',
    () {
      expect(
        shouldRevealCompletionAfterMasteredForTest(
          wasAnswered: false,
          isComplete: true,
        ),
        isTrue,
      );
      expect(
        shouldRevealCompletionAfterMasteredForTest(
          wasAnswered: true,
          isComplete: true,
        ),
        isFalse,
      );
    },
  );

  test('input feedback correct answer falls back to accepted meanings', () {
    const question = StudyQuestion(
      questionId: 'q11',
      questionType: 'enToCnInput',
      entrySourceId: 'debate',
      word: 'debate',
      prompt: 'debate',
      acceptedMeanings: ['\u8fa9\u8bba\uff1b\u4e89\u8bba\uff1b\u8ba8\u8bba'],
      questionIndex: 8,
      totalQuestions: 34,
    );
    const result = StudyResult(
      questionId: 'q11',
      entrySourceId: 'debate',
      questionType: 'enToCnInput',
      userResponse: '\u9700\u6c42',
      correctAnswer: '',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-06-25T00:00:00Z',
    );

    expect(
      inputCorrectAnswerTextForTest(result, question),
      '\u8fa9\u8bba\uff1b\u4e89\u8bba\uff1b\u8ba8\u8bba',
    );
  });
  test('wrong choice state marks correct option green and selected option red', () {
    const question = StudyQuestion(
      questionId: 'q6',
      questionType: 'enToCnChoice',
      entrySourceId: 'coalition',
      word: 'coalition',
      prompt: 'coalition',
      acceptedMeanings: [
        '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
      ],
      choices: [
        {
          'label': 'A',
          'text':
              '\u6293\uff0c\u6414\uff1b\u6293\u75d5\uff1b\u8d77\u8dd1\u7ebf',
        },
        {
          'label': 'B',
          'text':
              '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
        },
        {
          'label': 'C',
          'text': '\u87ba\u65cb\uff1b\u87ba\u65cb\u5f0f\u7684\u4e0a\u5347',
        },
        {
          'label': 'D',
          'text':
              '\u5931\u4e8b\u7684\u8239\uff0c\u6b8b\u9ab8\uff1b\u5931\u4e8b',
        },
      ],
      questionIndex: 2,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q6',
      entrySourceId: 'coalition',
      questionType: 'enToCnChoice',
      userResponse: 'A',
      correctAnswer:
          '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-05-12T00:00:00Z',
    );

    final a = choiceStateForTest(question, result, question.choices![0], 0);
    final b = choiceStateForTest(question, result, question.choices![1], 1);

    expect(a.isUserWrong, isTrue);
    expect(a.isCorrect, isFalse);
    expect(b.isCorrect, isTrue);
    expect(b.isUserWrong, isFalse);
  });

  test('wrong choice state also recognizes stored user response text', () {
    const question = StudyQuestion(
      questionId: 'q7',
      questionType: 'enToCnChoice',
      entrySourceId: 'coalition',
      word: 'coalition',
      prompt: 'coalition',
      acceptedMeanings: [
        '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
      ],
      choices: [
        {
          'label': 'A',
          'text':
              '\u6293\uff0c\u6414\uff1b\u6293\u75d5\uff1b\u8d77\u8dd1\u7ebf',
        },
        {
          'label': 'B',
          'text':
              '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
        },
      ],
      questionIndex: 2,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q7',
      entrySourceId: 'coalition',
      questionType: 'enToCnChoice',
      userResponse:
          '\u6293\uff0c\u6414\uff1b\u6293\u75d5\uff1b\u8d77\u8dd1\u7ebf',
      correctAnswer:
          '\u7ed3\u5408\u4f53\uff0c\u540c\u76df\uff1b\u7ed3\u5408\uff0c\u8054\u5408',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-05-12T00:00:00Z',
    );

    final a = choiceStateForTest(question, result, question.choices![0], 0);
    final b = choiceStateForTest(question, result, question.choices![1], 1);

    expect(a.isUserWrong, isTrue);
    expect(b.isCorrect, isTrue);
  });

  test(
    'wrong choice state prefers the locally submitted choice when result is empty',
    () {
      const question = StudyQuestion(
        questionId: 'q8',
        questionType: 'enToCnChoice',
        entrySourceId: 'famine',
        word: 'famine',
        prompt: 'famine',
        acceptedMeanings: ['\u9965\u8352\uff0c\u9965\u9991'],
        choices: [
          {
            'label': 'A',
            'text':
                '\u59ff\u52bf\uff0c\u4f53\u6001\uff1b\u770b\u6cd5\uff0c\u6001\u5ea6',
          },
          {
            'label': 'B',
            'text':
                '\u60f3\u8c61\uff0c\u5e7b\u60f3\uff1b\u5e7b\u60f3\u7684\u4ea7\u7269',
          },
          {'label': 'C', 'text': '\u9965\u8352\uff0c\u9965\u9991'},
          {'label': 'D', 'text': '\u804c\u4e1a\uff0c\u884c\u4e1a'},
        ],
        correctChoiceLabel: 'C',
        questionIndex: 3,
        totalQuestions: 20,
      );
      const result = StudyResult(
        questionId: 'q8',
        entrySourceId: 'famine',
        questionType: 'enToCnChoice',
        userResponse: '',
        correctAnswer: '\u9965\u8352\uff0c\u9965\u9991',
        outcome: AnswerOutcome.incorrect,
        responseTimeMs: 100,
        answeredAt: '2026-05-12T00:00:00Z',
      );

      final a = choiceStateForTest(
        question,
        result,
        question.choices![0],
        0,
        responseOverride: 'A',
      );
      final c = choiceStateForTest(
        question,
        result,
        question.choices![2],
        2,
        responseOverride: 'A',
      );

      expect(a.isUserWrong, isTrue);
      expect(a.isCorrect, isFalse);
      expect(c.isCorrect, isTrue);
      expect(c.isUserWrong, isFalse);
    },
  );

  test('choice state uses the unique correct label instead of broad meanings', () {
    const question = StudyQuestion(
      questionId: 'q9',
      questionType: 'enToCnChoice',
      entrySourceId: 'gesture',
      word: 'gesture',
      prompt: 'gesture',
      acceptedMeanings: [
        '\u53d6\u6d88\uff1b\u5220\u53bb\uff1b\u5212\u6389\uff1b\u628a...\u4f5c\u5e9f',
        '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
      ],
      choices: [
        {
          'label': 'A',
          'text':
              '\u53d6\u6d88\uff1b\u5220\u53bb\uff1b\u5212\u6389\uff1b\u628a...\u4f5c\u5e9f',
        },
        {
          'label': 'B',
          'text': '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
        },
        {
          'label': 'C',
          'text': '\u6982\u8ff0\uff1b\u7b80\u8ff0\uff1b\u52fe\u753b',
        },
      ],
      correctChoiceLabel: 'B',
      questionIndex: 14,
      totalQuestions: 28,
    );
    const result = StudyResult(
      questionId: 'q9',
      entrySourceId: 'gesture',
      questionType: 'enToCnChoice',
      userResponse: 'A',
      correctAnswer: '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-05-12T00:00:00Z',
    );

    final a = choiceStateForTest(question, result, question.choices![0], 0);
    final b = choiceStateForTest(question, result, question.choices![1], 1);

    expect(a.isCorrect, isFalse);
    expect(a.isUserWrong, isTrue);
    expect(b.isCorrect, isTrue);
    expect(b.isUserWrong, isFalse);
  });

  test('choice state uses result text when a stale label points at A', () {
    const question = StudyQuestion(
      questionId: 'q10',
      questionType: 'enToCnChoice',
      entrySourceId: 'gesture',
      word: 'gesture',
      prompt: 'gesture',
      acceptedMeanings: [
        '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
      ],
      choices: [
        {
          'label': 'A',
          'text':
              '\u53d6\u6d88\uff1b\u5220\u53bb\uff1b\u5212\u6389\uff1b\u628a...\u4f5c\u5e9f',
        },
        {
          'label': 'B',
          'text': '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
        },
      ],
      correctChoiceLabel: 'A',
      questionIndex: 14,
      totalQuestions: 28,
    );
    const result = StudyResult(
      questionId: 'q10',
      entrySourceId: 'gesture',
      questionType: 'enToCnChoice',
      userResponse: 'B',
      correctAnswer: '\u505a\u624b\u52bf\uff1b\u7528\u52a8\u4f5c\u793a\u610f',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-05-12T00:00:00Z',
    );

    final a = choiceStateForTest(question, result, question.choices![0], 0);
    final b = choiceStateForTest(question, result, question.choices![1], 1);

    expect(a.isCorrect, isFalse);
    expect(a.isUserWrong, isFalse);
    expect(b.isCorrect, isTrue);
    expect(b.isUserWrong, isFalse);
  });

  test(
    'latest submitted answer is merged when response answered list is stale',
    () {
      const question = StudyQuestion(
        questionId: 'q8',
        questionType: 'enToCnChoice',
        entrySourceId: 'famine',
        word: 'famine',
        prompt: 'famine',
        acceptedMeanings: ['\u9965\u8352\uff0c\u9965\u9991'],
        questionIndex: 3,
        totalQuestions: 20,
      );
      const result = StudyResult(
        questionId: 'q8',
        entrySourceId: 'famine',
        questionType: 'enToCnChoice',
        userResponse: 'A',
        correctAnswer: '\u9965\u8352\uff0c\u9965\u9991',
        outcome: AnswerOutcome.incorrect,
        responseTimeMs: 100,
        answeredAt: '2026-05-12T00:00:00Z',
      );

      final merged = mergeLatestAnsweredQuestionForTest(
        const <AnsweredStudyQuestion>[],
        question,
        result,
      );

      expect(merged, hasLength(1));
      expect(merged.single.question.questionId, 'q8');
      expect(merged.single.result.userResponse, 'A');
      expect(merged.single.result.outcome, AnswerOutcome.incorrect);
    },
  );

  test('submitted answer keeps synonym feedback returned by the bridge', () {
    const fallbackQuestion = StudyQuestion(
      questionId: 'q-synonym',
      questionType: 'enToCnInput',
      entrySourceId: 'assist',
      word: 'assist',
      prompt: 'assist',
      acceptedMeanings: ['帮助'],
      questionIndex: 1,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q-synonym',
      entrySourceId: 'assist',
      questionType: 'enToCnInput',
      userResponse: '帮助',
      correctAnswer: '帮助',
      outcome: AnswerOutcome.correct,
      responseTimeMs: 100,
      answeredAt: '2026-08-29T00:00:00Z',
    );
    const bridgeQuestion = StudyQuestion(
      questionId: 'q-synonym',
      questionType: 'enToCnInput',
      entrySourceId: 'assist',
      word: 'assist',
      prompt: 'assist',
      acceptedMeanings: ['帮助'],
      questionIndex: 1,
      totalQuestions: 20,
      synonymGroups: [
        StudySynonymGroup(meaning: '帮助', words: ['aid', 'help']),
      ],
    );

    final question = submittedQuestionWithFeedbackForTest(
      const [AnsweredStudyQuestion(question: bridgeQuestion, result: result)],
      fallbackQuestion,
      result,
    );

    expect(question.synonymGroups, hasLength(1));
    expect(question.synonymGroups.single.words, ['aid', 'help']);
  });

  test('newly mastered list only records an actual mastery increase', () {
    const question = StudyQuestion(
      questionId: 'q-mastered',
      questionType: 'enToCnInput',
      entrySourceId: 'mastered-word',
      word: 'mastered-word',
      prompt: 'mastered-word',
      acceptedMeanings: ['已掌握'],
      questionIndex: 1,
      totalQuestions: 20,
    );

    expect(
      newlyMasteredQuestionsForTest(
        existing: const [],
        question: question,
        masteredCountBefore: 12,
        masteredCountAfter: 12,
      ),
      isEmpty,
    );
    final recorded = newlyMasteredQuestionsForTest(
      existing: const [],
      question: question,
      masteredCountBefore: 12,
      masteredCountAfter: 13,
    );
    expect(recorded.single.word, 'mastered-word');
    expect(
      newlyMasteredQuestionsForTest(
        existing: recorded,
        question: question,
        masteredCountBefore: 13,
        masteredCountAfter: 14,
      ),
      hasLength(1),
    );
  });

  test(
    'latest submitted final answer is appended after stale answered list',
    () {
      final answered = List<AnsweredStudyQuestion>.generate(19, (index) {
        final questionNumber = index + 1;
        final question = StudyQuestion(
          questionId: 'q$questionNumber',
          questionType: 'enToCnChoice',
          entrySourceId: 'word$questionNumber',
          word: 'word$questionNumber',
          prompt: 'word$questionNumber',
          acceptedMeanings: const ['meaning'],
          questionIndex: questionNumber,
          totalQuestions: 20,
        );
        final result = StudyResult(
          questionId: 'q$questionNumber',
          entrySourceId: 'word$questionNumber',
          questionType: 'enToCnChoice',
          userResponse: 'A',
          correctAnswer: 'meaning',
          outcome: AnswerOutcome.correct,
          responseTimeMs: 100,
          answeredAt: '2026-05-12T00:00:00Z',
        );
        return AnsweredStudyQuestion(question: question, result: result);
      });
      const finalQuestion = StudyQuestion(
        questionId: 'q20',
        questionType: 'enToCnChoice',
        entrySourceId: 'word20',
        word: 'word20',
        prompt: 'word20',
        acceptedMeanings: ['final meaning'],
        questionIndex: 20,
        totalQuestions: 20,
      );
      const finalResult = StudyResult(
        questionId: 'q20',
        entrySourceId: 'word20',
        questionType: 'enToCnChoice',
        userResponse: 'B',
        correctAnswer: 'final meaning',
        outcome: AnswerOutcome.correct,
        responseTimeMs: 100,
        answeredAt: '2026-05-12T00:00:00Z',
      );

      final merged = mergeLatestAnsweredQuestionForTest(
        answered,
        finalQuestion,
        finalResult,
      );

      expect(merged, hasLength(20));
      expect(merged.last.question.questionId, 'q20');
      expect(merged.last.result.userResponse, 'B');
    },
  );

  test('sliding bridge window keeps existing feed identities stable', () {
    AnsweredStudyQuestion answered(int questionNumber, String response) {
      final question = StudyQuestion(
        questionId: 'q$questionNumber',
        questionType: 'enToCnChoice',
        entrySourceId: 'word$questionNumber',
        word: 'word$questionNumber',
        prompt: 'word$questionNumber',
        acceptedMeanings: const ['meaning'],
        questionIndex: questionNumber - 1,
        totalQuestions: 30,
      );
      return AnsweredStudyQuestion(
        question: question,
        result: StudyResult(
          questionId: question.questionId,
          entrySourceId: question.entrySourceId,
          questionType: question.questionType,
          userResponse: response,
          correctAnswer: 'meaning',
          outcome: AnswerOutcome.correct,
          responseTimeMs: 100,
          answeredAt: '2026-08-06T00:00:00Z',
        ),
      );
    }

    final existing = List.generate(
      20,
      (index) => answered(index + 1, 'old-${index + 1}'),
    );
    final incomingWindow = List.generate(
      20,
      (index) => answered(index + 2, 'new-${index + 2}'),
    );

    final merged = mergeStableAnsweredFeedForTest(existing, incomingWindow);

    expect(
      merged.map((item) => item.question.questionId),
      List.generate(21, (index) => 'q${index + 1}'),
    );
    expect(merged.first.result.userResponse, 'old-1');
    expect(merged[1].result.userResponse, 'new-2');
    expect(merged.last.result.userResponse, 'new-21');
  });
}

class _StudyExampleBridge extends RustBridge {
  String? lastUserMark;
  String? lastArticleId;

  @override
  Future<String> call(String method, [String? argument]) async {
    final request = argument == null
        ? <String, dynamic>{}
        : jsonDecode(argument) as Map<String, dynamic>;
    switch (method) {
      case 'tokenizeExamText':
        return jsonEncode({
          'offsetEncoding': 'utf16',
          'tokens': [
            {
              'text': 'We',
              'normalized': 'we',
              'startOffset': 0,
              'endOffset': 2,
            },
            {
              'text': 'observe',
              'normalized': 'observe',
              'startOffset': 3,
              'endOffset': 10,
            },
            {
              'text': 'change',
              'normalized': 'change',
              'startOffset': 11,
              'endOffset': 17,
            },
          ],
        });
      case 'getExamAnnotationState':
        return jsonEncode({
          'currentMarks': <Map<String, dynamic>>[],
          'priorMarks': [
            {'normalized': 'observe', 'meaning': '观察；遵守', 'mark': 'unknown'},
          ],
          'causalWords': <String>[],
          'annotations': <Map<String, dynamic>>[],
        });
      case 'inspectExamWord':
        lastArticleId = request['articleId'] as String?;
        lastUserMark = request['userMark'] as String?;
        return jsonEncode({
          'occurrenceId': 1,
          'entryId': 1,
          'word': request['word'],
          'normalized': 'observe',
          'meanings': ['观察', '遵守'],
          'userMark': lastUserMark ?? 'none',
          'isUnknown': lastUserMark != null && lastUserMark != 'none',
        });
      default:
        return '{}';
    }
  }
}

class _Utf8OffsetStudyExampleBridge extends RustBridge {
  @override
  Future<String> call(String method, [String? argument]) async {
    switch (method) {
      case 'tokenizeExamText':
        return jsonEncode({
          'tokens': [
            {
              'text': 'available',
              'normalized': 'available',
              'startOffset': 19,
              'endOffset': 28,
            },
          ],
        });
      case 'getExamAnnotationState':
        return jsonEncode({
          'currentMarks': <Map<String, dynamic>>[],
          'priorMarks': <Map<String, dynamic>>[],
          'causalWords': <String>[],
          'annotations': <Map<String, dynamic>>[],
        });
      default:
        return '{}';
    }
  }
}

import 'package:flutter_mobile/features/study_screen.dart';
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
  });

  test(
    'pending next question only activates after user-driven scroll settles',
    () {
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

      expect(
        shouldActivateVisiblePendingQuestionForTest(
          answeredQuestions: answered,
          currentQuestion: current,
          pendingNextQuestion: pending,
          visiblePageIndex: 21,
          userDrivenScroll: false,
        ),
        isFalse,
      );
      expect(
        shouldActivateVisiblePendingQuestionForTest(
          answeredQuestions: answered,
          currentQuestion: current,
          pendingNextQuestion: pending,
          visiblePageIndex: 20,
          userDrivenScroll: true,
        ),
        isFalse,
      );
      expect(
        shouldActivateVisiblePendingQuestionForTest(
          answeredQuestions: answered,
          currentQuestion: current,
          pendingNextQuestion: pending,
          visiblePageIndex: 21,
          userDrivenScroll: true,
        ),
        isTrue,
      );
    },
  );

  test('answered current question stays addressable before pending page', () {
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
      21,
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
      hintSuggestions: [],
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

  test('hint action opens for saved hints or AI suggestions', () {
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
    expect(questionHasHintForAction(withSuggestion), isTrue);
    expect(questionHasHintForAction(withHint), isTrue);
  });

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
}

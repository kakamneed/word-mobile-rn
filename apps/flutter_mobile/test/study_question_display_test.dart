import 'package:flutter_mobile/features/study_screen.dart';
import 'package:flutter_mobile/sdk/study_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(
    'cnToEn hero displays the prompt instead of the English answer',
    () {
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
    },
  );

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

  test('word skeleton hero displays the masked prompt instead of the full word', () {
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

    expect(display.title, 'a___itect');
    expect(display.title, isNot('architect'));
    expect(display.subtitle, 'n');
  });

  test('word skeleton display fills typed letters into underscores', () {
    expect(wordSkeletonDisplayForTest('f__dge', ''), 'f__dge');
    expect(wordSkeletonDisplayForTest('f__dge', 'r'), 'fr_dge');
    expect(wordSkeletonDisplayForTest('f__dge', 'ri'), 'fridge');
    expect(wordSkeletonDisplayForTest('f__dge', 'ride'), 'fridge');
  });

  test('choice display falls back to unique labels when bridge fields are sparse', () {
    final first = choiceDisplayForTest({'text': 'versatile'}, 0);
    final second = choiceDisplayForTest({'text': 'commonplace'}, 1);

    expect(first.value, 'A');
    expect(first.label, 'A');
    expect(first.text, 'versatile');
    expect(second.value, 'B');
    expect(second.label, 'B');
    expect(second.text, 'commonplace');
    expect(first.value, isNot(second.value));
  });

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

  test('wrong choice state marks correct option green and selected option red', () {
    const question = StudyQuestion(
      questionId: 'q6',
      questionType: 'enToCnChoice',
      entrySourceId: 'coalition',
      word: 'coalition',
      prompt: 'coalition',
      acceptedMeanings: ['结合体，同盟；结合，联合'],
      choices: [
        {'label': 'A', 'text': '抓，搔；抓痕；起跑线'},
        {'label': 'B', 'text': '结合体，同盟；结合，联合'},
        {'label': 'C', 'text': '螺旋；螺旋式的上升'},
        {'label': 'D', 'text': '失事的船，残骸；失事'},
      ],
      questionIndex: 2,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q6',
      entrySourceId: 'coalition',
      questionType: 'enToCnChoice',
      userResponse: 'A',
      correctAnswer: '结合体，同盟；结合，联合',
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
      acceptedMeanings: ['结合体，同盟；结合，联合'],
      choices: [
        {'label': 'A', 'text': '抓，搔；抓痕；起跑线'},
        {'label': 'B', 'text': '结合体，同盟；结合，联合'},
      ],
      questionIndex: 2,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q7',
      entrySourceId: 'coalition',
      questionType: 'enToCnChoice',
      userResponse: '抓，搔；抓痕；起跑线',
      correctAnswer: '结合体，同盟；结合，联合',
      outcome: AnswerOutcome.incorrect,
      responseTimeMs: 100,
      answeredAt: '2026-05-12T00:00:00Z',
    );

    final a = choiceStateForTest(question, result, question.choices![0], 0);
    final b = choiceStateForTest(question, result, question.choices![1], 1);

    expect(a.isUserWrong, isTrue);
    expect(b.isCorrect, isTrue);
  });

  test('wrong choice state prefers the locally submitted choice when result is empty', () {
    const question = StudyQuestion(
      questionId: 'q8',
      questionType: 'enToCnChoice',
      entrySourceId: 'famine',
      word: 'famine',
      prompt: 'famine',
      acceptedMeanings: ['饥荒，饥馑'],
      choices: [
        {'label': 'A', 'text': '姿势，体态；看法，态度'},
        {'label': 'B', 'text': '想象，幻想；幻想的产物'},
        {'label': 'C', 'text': '饥荒，饥馑'},
        {'label': 'D', 'text': '职业，行业'},
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
      correctAnswer: '饥荒，饥馑',
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
  });

  test('choice state uses the unique correct label instead of broad meanings', () {
    const question = StudyQuestion(
      questionId: 'q9',
      questionType: 'enToCnChoice',
      entrySourceId: 'gesture',
      word: 'gesture',
      prompt: 'gesture',
      acceptedMeanings: ['取消；删去；划掉；把...作废', '做手势；用动作示意'],
      choices: [
        {'label': 'A', 'text': '取消；删去；划掉；把...作废'},
        {'label': 'B', 'text': '做手势；用动作示意'},
        {'label': 'C', 'text': '概述；简述；勾画'},
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
      correctAnswer: '做手势；用动作示意',
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
      acceptedMeanings: ['做手势；用动作示意'],
      choices: [
        {'label': 'A', 'text': '取消；删去；划掉；把...作废'},
        {'label': 'B', 'text': '做手势；用动作示意'},
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
      correctAnswer: '做手势；用动作示意',
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

  test('latest submitted answer is merged when response answered list is stale', () {
    const question = StudyQuestion(
      questionId: 'q8',
      questionType: 'enToCnChoice',
      entrySourceId: 'famine',
      word: 'famine',
      prompt: 'famine',
      acceptedMeanings: ['饥荒，饥馑'],
      questionIndex: 3,
      totalQuestions: 20,
    );
    const result = StudyResult(
      questionId: 'q8',
      entrySourceId: 'famine',
      questionType: 'enToCnChoice',
      userResponse: 'A',
      correctAnswer: '饥荒，饥馑',
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
  });

  test('latest submitted final answer is appended after stale answered list', () {
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
  });
}

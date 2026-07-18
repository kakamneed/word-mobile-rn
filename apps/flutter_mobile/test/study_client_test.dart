import 'package:flutter_mobile/sdk/study_client.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('submit answer response tolerates nullable final result fields', () {
    final response = SubmitAnswerResponse.fromJson({
      'result': {
        'questionId': 'q-final',
        'entrySourceId': null,
        'questionType': null,
        'userResponse': null,
        'normalizedResponse': null,
        'correctAnswer': null,
        'outcome': null,
        'responseTimeMs': null,
        'answeredAt': null,
      },
      'isComplete': true,
      'currentQuestion': null,
      'summary': {
        'sessionId': null,
        'totalQuestions': '1',
        'correctCount': 1.0,
        'fuzzyCorrectCount': null,
        'incorrectCount': null,
        'skippedCount': null,
        'totalWords': null,
        'wrongWordCount': null,
        'accuracyPercent': null,
        'totalTimeMs': null,
        'completedAt': null,
      },
      'nextAction': null,
      'progress': {'current': '1', 'total': null},
      'hintPrompt': {
        'entryId': '42',
        'word': null,
        'errorCount': null,
        'triggerOutcome': null,
        'suggestions': const [],
      },
    });

    expect(response.isComplete, isTrue);
    expect(response.currentQuestion, isNull);
    expect(response.result.entrySourceId, '');
    expect(response.result.questionType, 'unknown');
    expect(response.result.outcome, AnswerOutcome.incorrect);
    expect(response.summary?.sessionId, '');
    expect(response.summary?.totalQuestions, 1);
    expect(response.summary?.correctCount, 1);
    expect(response.summary?.accuracyPercent, 0);
    expect(response.progress.current, 1);
    expect(response.progress.total, 0);
    expect(response.hintPrompt?.entryId, 42);
    expect(response.hintPrompt?.word, '');
  });

  test('submit answer response ignores incomplete currentQuestion objects', () {
    final response = SubmitAnswerResponse.fromJson({
      'result': {
        'questionId': 'q-final',
        'entrySourceId': '1',
        'questionType': 'enToCnChoice',
        'userResponse': 'A',
        'correctAnswer': 'A',
        'outcome': 'correct',
        'responseTimeMs': 10,
        'answeredAt': '2026-05-05T00:00:00Z',
      },
      'isComplete': true,
      'currentQuestion': {
        'userHint': null,
        'hasHint': false,
        'hintSuggestions': const [],
      },
      'summary': {
        'sessionId': 's-final',
        'totalQuestions': 1,
        'correctCount': 1,
        'fuzzyCorrectCount': 0,
        'incorrectCount': 0,
        'skippedCount': 0,
        'totalWords': 1,
        'wrongWordCount': 0,
        'accuracyPercent': 100,
        'totalTimeMs': 10,
        'completedAt': '2026-05-05T00:00:00Z',
      },
      'nextAction': 'Return to today',
      'progress': {'current': 1, 'total': 1},
      'hintPrompt': null,
    });

    expect(response.isComplete, isTrue);
    expect(response.currentQuestion, isNull);
    expect(response.summary?.correctCount, 1);
  });

  test('study question decode preserves non-A correct choice identity', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'q-choice',
      'questionType': 'enToCnChoice',
      'entrySourceId': 'famine',
      'word': 'famine',
      'prompt': 'famine',
      'acceptedMeanings': ['food shortage'],
      'choices': [
        {'label': 'A', 'value': 'A', 'text': 'celebration'},
        {'label': 'B', 'value': 'B', 'text': 'festival'},
        {'label': 'C', 'value': 'C', 'text': 'food shortage'},
        {'label': 'D', 'value': 'D', 'text': 'journey'},
      ],
      'correctChoiceLabel': 'C',
      'questionIndex': 2,
      'totalQuestions': 8,
    });

    expect(question.correctChoiceLabel, 'C');
    expect(question.choices?[2]['label'], 'C');
    expect(question.choices?[2]['value'], 'C');
    expect(question.choices?[2]['text'], 'food shortage');
  });

  test('study question decode filters empty choice text distractors', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'q-choice-empty-distractor',
      'questionType': 'cnToEnChoice',
      'entrySourceId': 'debate',
      'word': 'debate',
      'prompt': 'discussion',
      'acceptedMeanings': ['discussion'],
      'choices': [
        {'label': 'A', 'text': 'debate'},
        {'label': 'B', 'text': ''},
        {'label': 'C', 'text': 'courtesy'},
        {'label': 'D', 'text': '   '},
      ],
      'correctChoiceLabel': 'A',
      'questionIndex': 15,
      'totalQuestions': 28,
    });

    expect(question.choices, hasLength(2));
    expect(question.choices?.map((choice) => choice['label']), ['A', 'C']);
    expect(question.correctChoiceLabel, 'A');
  });

  test(
    'choice question decode fails instead of guessing A when correct label is missing',
    () {
      expect(
        () => StudyQuestion.fromJson({
          'questionId': 'q-choice',
          'questionType': 'enToCnChoice',
          'entrySourceId': 'famine',
          'word': 'famine',
          'prompt': 'famine',
          'acceptedMeanings': ['food shortage'],
          'choices': [
            {'label': 'A', 'text': 'celebration'},
            {'label': 'B', 'text': 'food shortage'},
          ],
          'questionIndex': 2,
          'totalQuestions': 8,
        }),
        throwsA(
          isA<BridgeError>().having(
            (error) => error.kind,
            'kind',
            BridgeErrorKind.protocol,
          ),
        ),
      );
    },
  );

  test(
    'choice question decode fails when correct label does not match choices',
    () {
      expect(
        () => StudyQuestion.fromJson({
          'questionId': 'q-choice',
          'questionType': 'enToCnChoice',
          'entrySourceId': 'famine',
          'word': 'famine',
          'prompt': 'famine',
          'acceptedMeanings': ['food shortage'],
          'choices': [
            {'label': 'A', 'text': 'celebration'},
            {'label': 'B', 'text': 'food shortage'},
          ],
          'correctChoiceLabel': 'C',
          'questionIndex': 2,
          'totalQuestions': 8,
        }),
        throwsA(
          isA<BridgeError>().having(
            (error) => error.code,
            'code',
            'STUDY_QUESTION_CORRECT_CHOICE_INVALID',
          ),
        ),
      );
    },
  );

  test(
    'study question decode falls back when prompt is empty in stale snapshot',
    () {
      final question = StudyQuestion.fromJson({
        'questionId': 'q-empty-prompt',
        'questionType': 'wordSkeletonInput',
        'entrySourceId': 'ruby',
        'word': 'ruby',
        'prompt': '',
        'acceptedMeanings': ['ub'],
        'questionIndex': 5,
        'totalQuestions': 28,
      });

      expect(question.prompt, 'ruby');
      expect(question.word, 'ruby');
      expect(question.acceptedMeanings, ['ub']);
    },
  );
}

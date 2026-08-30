import 'dart:convert';

import 'package:flutter_mobile/sdk/study_client.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('start response preserves its native session progress', () {
    final response = StartSessionResponse.fromJson({
      'session': {
        'sessionId': 'hf-restart',
        'mode': 'highFrequency',
        'totalWords': 100,
        'startedAt': '2026-08-23T00:00:00Z',
      },
      'currentQuestion': {
        'questionId': 'q-hf-restart',
        'questionType': 'enToCnInput',
        'entrySourceId': 'obtain',
        'word': 'obtain',
        'prompt': 'obtain',
        'acceptedMeanings': ['获得'],
        'questionIndex': 0,
        'totalQuestions': 100,
      },
      'progress': {'current': 1, 'total': 100},
    });

    expect(response.progress.current, 1);
    expect(response.progress.total, 100);
  });

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
      'masteredCount': '27',
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
    expect(response.masteredCount, 27);
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

  test('study question decode preserves cumulative error count', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'q-error-count',
      'questionType': 'enToCnInput',
      'entrySourceId': 'available',
      'word': 'available',
      'prompt': 'available',
      'acceptedMeanings': ['可用的'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'errorCount': 7,
    });

    expect(question.errorCount, 7);
  });

  test('study question decodes the native entry ID for saved hint edits', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'q-entry-id',
      'questionType': 'enToCnInput',
      'entrySourceId': 'concept',
      'entryId': 42,
      'word': 'concept',
      'prompt': 'concept',
      'acceptedMeanings': ['概念'],
      'questionIndex': 0,
      'totalQuestions': 1,
    });

    expect(question.entryId, 42);
  });

  test('study question decodes grouped high-frequency synonyms', () {
    final question = StudyQuestion.fromJson({
      'questionId': 'q-synonyms',
      'questionType': 'enToCnInput',
      'entrySourceId': 'assist',
      'word': 'assist',
      'prompt': 'assist',
      'acceptedMeanings': ['帮助'],
      'questionIndex': 0,
      'totalQuestions': 1,
      'synonymGroups': [
        {
          'meaning': '帮助',
          'words': ['aid', 'help'],
        },
        {
          'meaning': '协助',
          'words': ['support'],
        },
      ],
    });

    expect(question.synonymGroups.first.meaning, '帮助');
    expect(question.synonymGroups.first.words, ['aid', 'help']);
    expect(question.synonymGroups.last.words, ['support']);
  });

  test('submit answer sends whether the current hint was used', () async {
    final bridge = _SubmitRecordingBridge();
    final client = StudyClient(bridge, const BridgeCodec());

    await client.submitAnswer(
      questionId: 'q-hinted',
      response: '释义',
      responseTimeMs: 1200,
      hintUsed: true,
      studyMode: 'highFrequency',
    );

    expect(bridge.method, 'submitStudyAnswer');
    final request = jsonDecode(bridge.argument!) as Map<String, dynamic>;
    expect(request['hintUsed'], isTrue);
    expect(request['studyMode'], 'highFrequency');
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

class _SubmitRecordingBridge extends RustBridge {
  String? method;
  String? argument;

  @override
  Future<String> call(String method, [String? argument]) async {
    this.method = method;
    this.argument = argument;
    return jsonEncode({
      'result': {
        'questionId': 'q-hinted',
        'entrySourceId': 'available',
        'questionType': 'enToCnInput',
        'userResponse': '释义',
        'correctAnswer': '释义',
        'outcome': 'correct',
        'responseTimeMs': 1200,
        'answeredAt': '2026-08-20T00:00:00Z',
      },
      'isComplete': false,
      'currentQuestion': null,
      'progress': {'current': 1, 'total': 2},
    });
  }
}

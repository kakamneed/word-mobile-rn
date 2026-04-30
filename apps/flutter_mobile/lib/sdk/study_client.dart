/// Study client - typed SDK for the study session API.
library;

import '../bridge/bridge.dart';

/// Study session info.
class StudySession {
  final String sessionId;
  final String mode;
  final int totalWords;
  final int? wordbookId;
  final String startedAt;

  const StudySession({
    required this.sessionId,
    required this.mode,
    required this.totalWords,
    this.wordbookId,
    required this.startedAt,
  });

  factory StudySession.fromJson(Map<String, dynamic> json) => StudySession(
        sessionId: json['sessionId'] as String,
        mode: json['mode'] as String,
        totalWords: json['totalWords'] as int,
        wordbookId: json['wordbookId'] as int?,
        startedAt: json['startedAt'] as String,
      );
}

/// Entry payload for starting a session without relying on a preloaded local selector.
class StartSessionEntryPayload {
  final String sourceId;
  final String word;
  final String? partOfSpeech;
  final double frequency;
  final String? phoneticUs;
  final String? phoneticUk;
  final List<Map<String, dynamic>> meaningDetails;
  final List<String> meanings;
  final String? exampleSentence;
  final String? exampleTranslation;

  const StartSessionEntryPayload({
    required this.sourceId,
    required this.word,
    this.partOfSpeech,
    this.frequency = 0,
    this.phoneticUs,
    this.phoneticUk,
    this.meaningDetails = const [],
    this.meanings = const [],
    this.exampleSentence,
    this.exampleTranslation,
  });

  Map<String, dynamic> toJson() => {
        'sourceId': sourceId,
        'word': word,
        'partOfSpeech': partOfSpeech,
        'frequency': frequency,
        'phoneticUs': phoneticUs,
        'phoneticUk': phoneticUk,
        'meaningDetails': meaningDetails,
        'meanings': meanings,
        'exampleSentence': exampleSentence,
        'exampleTranslation': exampleTranslation,
      };
}

/// Study question.
class StudyQuestion {
  final String questionId;
  final String questionType;
  final String entrySourceId;
  final String word;
  final String prompt;
  final String? partOfSpeech;
  final String? phoneticUs;
  final String? phoneticUk;
  final String? exampleSentence;
  final String? exampleTranslation;
  final List<String> acceptedMeanings;
  final List<Map<String, dynamic>>? choices;
  final String? correctChoiceLabel;
  final int questionIndex;
  final int totalQuestions;

  const StudyQuestion({
    required this.questionId,
    required this.questionType,
    required this.entrySourceId,
    required this.word,
    required this.prompt,
    this.partOfSpeech,
    this.phoneticUs,
    this.phoneticUk,
    this.exampleSentence,
    this.exampleTranslation,
    required this.acceptedMeanings,
    this.choices,
    this.correctChoiceLabel,
    required this.questionIndex,
    required this.totalQuestions,
  });

  factory StudyQuestion.fromJson(Map<String, dynamic> json) => StudyQuestion(
        questionId: json['questionId'] as String,
        questionType: json['questionType'] as String,
        entrySourceId: json['entrySourceId'] as String,
        word: json['word'] as String,
        prompt: json['prompt'] as String,
        partOfSpeech: json['partOfSpeech'] as String?,
        phoneticUs: json['phoneticUs'] as String?,
        phoneticUk: json['phoneticUk'] as String?,
        exampleSentence: json['exampleSentence'] as String?,
        exampleTranslation: json['exampleTranslation'] as String?,
        acceptedMeanings: (json['acceptedMeanings'] as List<dynamic>).cast<String>(),
        choices: (json['choices'] as List<dynamic>?)?.cast<Map<String, dynamic>>(),
        correctChoiceLabel: json['correctChoiceLabel'] as String?,
        questionIndex: json['questionIndex'] as int,
        totalQuestions: json['totalQuestions'] as int,
      );

  /// Whether this is a choice-type question.
  bool get isChoiceType =>
      questionType == 'enToCnChoice' ||
      questionType == 'exampleToCnChoice' ||
      questionType == 'cnToEnChoice';
}

/// Session progress.
class SessionProgress {
  final int current;
  final int total;

  const SessionProgress({required this.current, required this.total});

  factory SessionProgress.fromJson(Map<String, dynamic> json) => SessionProgress(
        current: json['current'] as int,
        total: json['total'] as int,
      );
}

/// Resume hint for an unfinished study session.
class ResumeSessionHint {
  final bool hasResume;
  final String? mode;
  final int? current;
  final int? total;
  final String? word;

  const ResumeSessionHint({
    required this.hasResume,
    this.mode,
    this.current,
    this.total,
    this.word,
  });

  factory ResumeSessionHint.fromJson(Map<String, dynamic> json) => ResumeSessionHint(
        hasResume: json['hasResume'] as bool? ?? false,
        mode: json['mode'] as String?,
        current: json['current'] as int?,
        total: json['total'] as int?,
        word: json['word'] as String?,
      );
}
/// Answer outcome enum.
enum AnswerOutcome { correct, fuzzyCorrect, incorrect, skipped }

/// Study result for a single answer.
class StudyResult {
  final String questionId;
  final String entrySourceId;
  final String questionType;
  final String userResponse;
  final String? normalizedResponse;
  final String correctAnswer;
  final AnswerOutcome outcome;
  final int responseTimeMs;
  final String answeredAt;

  const StudyResult({
    required this.questionId,
    required this.entrySourceId,
    required this.questionType,
    required this.userResponse,
    this.normalizedResponse,
    required this.correctAnswer,
    required this.outcome,
    required this.responseTimeMs,
    required this.answeredAt,
  });

  factory StudyResult.fromJson(Map<String, dynamic> json) => StudyResult(
        questionId: json['questionId'] as String,
        entrySourceId: json['entrySourceId'] as String,
        questionType: json['questionType'] as String,
        userResponse: json['userResponse'] as String,
        normalizedResponse: json['normalizedResponse'] as String?,
        correctAnswer: json['correctAnswer'] as String,
        outcome: _parseOutcome(json['outcome'] as String),
        responseTimeMs: json['responseTimeMs'] as int,
        answeredAt: json['answeredAt'] as String,
      );
}

AnswerOutcome _parseOutcome(String s) => switch (s) {
      'correct' => AnswerOutcome.correct,
      'fuzzyCorrect' => AnswerOutcome.fuzzyCorrect,
      'incorrect' => AnswerOutcome.incorrect,
      'skipped' => AnswerOutcome.skipped,
      _ => AnswerOutcome.incorrect,
    };

/// Session summary.
class SessionSummary {
  final String sessionId;
  final int totalQuestions;
  final int correctCount;
  final int fuzzyCorrectCount;
  final int incorrectCount;
  final int skippedCount;
  final int totalWords;
  final int wrongWordCount;
  final double accuracyPercent;
  final int totalTimeMs;
  final String completedAt;

  const SessionSummary({
    required this.sessionId,
    required this.totalQuestions,
    required this.correctCount,
    required this.fuzzyCorrectCount,
    required this.incorrectCount,
    required this.skippedCount,
    required this.totalWords,
    required this.wrongWordCount,
    required this.accuracyPercent,
    required this.totalTimeMs,
    required this.completedAt,
  });

  factory SessionSummary.fromJson(Map<String, dynamic> json) => SessionSummary(
        sessionId: json['sessionId'] as String,
        totalQuestions: json['totalQuestions'] as int,
        correctCount: json['correctCount'] as int,
        fuzzyCorrectCount: json['fuzzyCorrectCount'] as int,
        incorrectCount: json['incorrectCount'] as int,
        skippedCount: json['skippedCount'] as int,
        totalWords: json['totalWords'] as int,
        wrongWordCount: json['wrongWordCount'] as int,
        accuracyPercent: (json['accuracyPercent'] as num).toDouble(),
        totalTimeMs: json['totalTimeMs'] as int,
        completedAt: json['completedAt'] as String,
      );
}

/// Response from starting a study session.
class StartSessionResponse {
  final StudySession session;
  final StudyQuestion currentQuestion;
  final SessionProgress progress;

  const StartSessionResponse({
    required this.session,
    required this.currentQuestion,
    required this.progress,
  });

  factory StartSessionResponse.fromJson(Map<String, dynamic> json) =>
      StartSessionResponse(
        session: StudySession.fromJson(json['session'] as Map<String, dynamic>),
        currentQuestion:
            StudyQuestion.fromJson(json['currentQuestion'] as Map<String, dynamic>),
        progress:
            SessionProgress.fromJson(json['progress'] as Map<String, dynamic>),
      );
}

/// Response from submitting a study answer.
class SubmitAnswerResponse {
  final StudyResult result;
  final bool isComplete;
  final StudyQuestion? currentQuestion;
  final SessionSummary? summary;
  final String? nextAction;
  final SessionProgress progress;

  const SubmitAnswerResponse({
    required this.result,
    required this.isComplete,
    this.currentQuestion,
    this.summary,
    this.nextAction,
    required this.progress,
  });

  factory SubmitAnswerResponse.fromJson(Map<String, dynamic> json) =>
      SubmitAnswerResponse(
        result: StudyResult.fromJson(json['result'] as Map<String, dynamic>),
        isComplete: json['isComplete'] as bool,
        currentQuestion: json['currentQuestion'] != null
            ? StudyQuestion.fromJson(json['currentQuestion'] as Map<String, dynamic>)
            : null,
        summary: json['summary'] != null
            ? SessionSummary.fromJson(json['summary'] as Map<String, dynamic>)
            : null,
        nextAction: json['nextAction'] as String?,
        progress:
            SessionProgress.fromJson(json['progress'] as Map<String, dynamic>),
      );
}

/// Response from completing a session.
class CompleteSessionResponse {
  final SessionSummary summary;
  final String nextAction;

  const CompleteSessionResponse({required this.summary, required this.nextAction});

  factory CompleteSessionResponse.fromJson(Map<String, dynamic> json) =>
      CompleteSessionResponse(
        summary: SessionSummary.fromJson(json['summary'] as Map<String, dynamic>),
        nextAction: json['nextAction'] as String,
      );
}

/// Client for study session operations.
class StudyClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const StudyClient(this._bridge, this._codec);

  /// Start a new study session.
  Future<StartSessionResponse> startSession({
    required String mode,
    int? wordbookId,
    required List<String> entrySourceIds,
    List<StartSessionEntryPayload>? entryPayloads,
    List<StartSessionEntryPayload>? distractorPayloads,
  }) async {
    final request = <String, dynamic>{
      'mode': mode,
      'entrySourceIds': entrySourceIds,
    };
    if (wordbookId != null) request['wordbookId'] = wordbookId;
    if (entryPayloads != null) {
      request['entryPayloads'] = entryPayloads.map((payload) => payload.toJson()).toList();
    }
    if (distractorPayloads != null) {
      request['distractorPayloads'] =
          distractorPayloads.map((payload) => payload.toJson()).toList();
    }

    final raw = await _bridge.call('startStudySession', _codec.encodeRequest(request));
    final json = _codec.decodeResponse(raw);
    return StartSessionResponse.fromJson(json);
  }

  Future<ResumeSessionHint> getResumeSessionHint() async {
    final raw = await _bridge.call('getResumeSessionHint');
    final json = _codec.decodeResponse(raw);
    return ResumeSessionHint.fromJson(json);
  }
  /// Submit an answer for the current question.
  Future<SubmitAnswerResponse> submitAnswer({
    required String questionId,
    required String response,
    required int responseTimeMs,
  }) async {
    final request = <String, dynamic>{
      'questionId': questionId,
      'response': response,
      'responseTimeMs': responseTimeMs,
    };

    final raw = await _bridge.call('submitStudyAnswer', _codec.encodeRequest(request));
    final json = _codec.decodeResponse(raw);
    return SubmitAnswerResponse.fromJson(json);
  }

  /// Complete the active study session.
  Future<CompleteSessionResponse> completeSession(String sessionId) async {
    final raw = await _bridge.call('completeStudySession', sessionId);
    final json = _codec.decodeResponse(raw);
    return CompleteSessionResponse.fromJson(json);
  }

  /// Cancel the active study session.
  Future<void> cancelSession(String sessionId) async {
    await _bridge.callVoid('cancelStudySession', sessionId);
  }
}

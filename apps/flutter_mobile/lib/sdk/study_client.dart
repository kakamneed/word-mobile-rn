/// Study client - typed SDK for the study session API.
library;

import '../bridge/bridge.dart';
import 'wrong_words_client.dart';

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
  final String? userHint;
  final bool hasHint;
  final List<WordHintSuggestion> hintSuggestions;

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
    this.userHint,
    this.hasHint = false,
    this.hintSuggestions = const [],
  });

  factory StudyQuestion.fromJson(Map<String, dynamic> json) {
    final questionType = _requiredString(json, 'questionType');
    final choices = _choiceListOrNull(json['choices']);
    final correctChoiceLabel = _jsonNullableString(json['correctChoiceLabel']);
    if (_isChoiceQuestionType(questionType)) {
      if (choices == null || choices.isEmpty) {
        throw BridgeError.protocol(
          'STUDY_QUESTION_CHOICES_MISSING',
          'Choice question is missing choices.',
        );
      }
      final normalizedCorrect = correctChoiceLabel?.trim();
      if (normalizedCorrect == null || normalizedCorrect.isEmpty) {
        throw BridgeError.protocol(
          'STUDY_QUESTION_CORRECT_CHOICE_MISSING',
          'Choice question is missing correctChoiceLabel.',
        );
      }
      final hasCorrectChoice = choices.any((choice) {
        final label = _jsonString(choice['label']).trim();
        final value = _jsonString(choice['value']).trim();
        return label == normalizedCorrect || value == normalizedCorrect;
      });
      if (!hasCorrectChoice) {
        throw BridgeError.protocol(
          'STUDY_QUESTION_CORRECT_CHOICE_INVALID',
          'correctChoiceLabel does not match any choice label or value.',
        );
      }
    }

    return StudyQuestion(
      questionId: _requiredString(json, 'questionId'),
      questionType: questionType,
      entrySourceId: _requiredString(json, 'entrySourceId'),
      word: _requiredString(json, 'word'),
      prompt: _requiredString(json, 'prompt'),
      partOfSpeech: _jsonNullableString(json['partOfSpeech']),
      phoneticUs: _jsonNullableString(json['phoneticUs']),
      phoneticUk: _jsonNullableString(json['phoneticUk']),
      exampleSentence: _jsonNullableString(json['exampleSentence']),
      exampleTranslation: _jsonNullableString(json['exampleTranslation']),
      acceptedMeanings: _jsonStringList(json['acceptedMeanings']),
      choices: choices,
      correctChoiceLabel: correctChoiceLabel,
      questionIndex: _jsonInt(json['questionIndex']),
      totalQuestions: _jsonInt(json['totalQuestions']),
      userHint: _jsonNullableString(json['userHint']),
      hasHint: _jsonBool(json['hasHint']),
      hintSuggestions: (json['hintSuggestions'] as List<dynamic>? ?? const [])
          .whereType<Map<String, dynamic>>()
          .map(WordHintSuggestion.fromJson)
          .toList(growable: false),
    );
  }

  /// Whether this is a choice-type question.
  bool get isChoiceType =>
      questionType == 'enToCnChoice' ||
      questionType == 'exampleToCnChoice' ||
      questionType == 'exampleToCnChoiceNoTranslation' ||
      questionType == 'cnToEnChoice';

  StudyQuestion copyWith({
    String? userHint,
    bool? hasHint,
    List<WordHintSuggestion>? hintSuggestions,
  }) =>
      StudyQuestion(
        questionId: questionId,
        questionType: questionType,
        entrySourceId: entrySourceId,
        word: word,
        prompt: prompt,
        partOfSpeech: partOfSpeech,
        phoneticUs: phoneticUs,
        phoneticUk: phoneticUk,
        exampleSentence: exampleSentence,
        exampleTranslation: exampleTranslation,
        acceptedMeanings: acceptedMeanings,
        choices: choices,
        correctChoiceLabel: correctChoiceLabel,
        questionIndex: questionIndex,
        totalQuestions: totalQuestions,
        userHint: userHint ?? this.userHint,
        hasHint: hasHint ?? this.hasHint,
        hintSuggestions: hintSuggestions ?? this.hintSuggestions,
      );
}

class HintPrompt {
  final int entryId;
  final String word;
  final int errorCount;
  final String triggerOutcome;
  final List<WordHintSuggestion> suggestions;

  const HintPrompt({
    required this.entryId,
    required this.word,
    required this.errorCount,
    required this.triggerOutcome,
    required this.suggestions,
  });

  factory HintPrompt.fromJson(Map<String, dynamic> json) => HintPrompt(
        entryId: _jsonInt(json['entryId']),
        word: _jsonString(json['word']),
        errorCount: _jsonInt(json['errorCount']),
        triggerOutcome: _jsonString(json['triggerOutcome']),
        suggestions: (json['suggestions'] as List<dynamic>? ?? const [])
            .whereType<Map<String, dynamic>>()
            .map(WordHintSuggestion.fromJson)
            .toList(growable: false),
      );
}

/// Session progress.
class SessionProgress {
  final int current;
  final int total;

  const SessionProgress({required this.current, required this.total});

  factory SessionProgress.fromJson(Map<String, dynamic> json) => SessionProgress(
        current: _jsonInt(json['current']),
        total: _jsonInt(json['total']),
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
        questionId: _jsonString(json['questionId']),
        entrySourceId: _jsonString(json['entrySourceId']),
        questionType: _jsonString(json['questionType'], fallback: 'unknown'),
        userResponse: _jsonString(json['userResponse']),
        normalizedResponse: json['normalizedResponse'] as String?,
        correctAnswer: _jsonString(json['correctAnswer']),
        outcome: _parseOutcome(_jsonString(json['outcome'])),
        responseTimeMs: _jsonInt(json['responseTimeMs']),
        answeredAt: _jsonString(
          json['answeredAt'],
          fallback: DateTime.now().toIso8601String(),
        ),
      );
}

class AnsweredStudyQuestion {
  final StudyQuestion question;
  final StudyResult result;

  const AnsweredStudyQuestion({
    required this.question,
    required this.result,
  });

  factory AnsweredStudyQuestion.fromJson(Map<String, dynamic> json) =>
      AnsweredStudyQuestion(
        question: StudyQuestion.fromJson(
          json['question'] as Map<String, dynamic>,
        ),
        result: StudyResult.fromJson(json['result'] as Map<String, dynamic>),
      );
}

String _jsonString(Object? value, {String fallback = ''}) {
  if (value == null) return fallback;
  if (value is String) return value;
  return value.toString();
}

String? _jsonNullableString(Object? value) {
  if (value == null) return null;
  if (value is String) return value;
  return value.toString();
}

int _jsonInt(Object? value, {int fallback = 0}) {
  if (value == null) return fallback;
  if (value is int) return value;
  if (value is num) return value.toInt();
  return int.tryParse(value.toString()) ?? fallback;
}

double _jsonDouble(Object? value, {double fallback = 0}) {
  if (value == null) return fallback;
  if (value is num) return value.toDouble();
  return double.tryParse(value.toString()) ?? fallback;
}

bool _jsonBool(Object? value, {bool fallback = false}) {
  if (value == null) return fallback;
  if (value is bool) return value;
  if (value is num) return value != 0;
  final normalized = value.toString().toLowerCase();
  if (normalized == 'true' || normalized == '1') return true;
  if (normalized == 'false' || normalized == '0') return false;
  return fallback;
}

List<String> _jsonStringList(Object? value) {
  if (value is! List) return const [];
  return value
      .map(_jsonString)
      .where((item) => item.isNotEmpty)
      .toList(growable: false);
}

List<Map<String, dynamic>>? _jsonMapListOrNull(Object? value) {
  if (value is! List) return null;
  return value
      .whereType<Map<String, dynamic>>()
      .toList(growable: false);
}

List<Map<String, dynamic>>? _choiceListOrNull(Object? value) {
  final choices = _jsonMapListOrNull(value);
  if (choices == null) return null;
  return choices
      .map((choice) => {
            ...choice,
            'label': _requiredString(choice, 'label'),
            'text': _requiredString(choice, 'text'),
          })
      .toList(growable: false);
}

String _requiredString(Map<String, dynamic> json, String key) {
  final value = json[key];
  if (value == null) {
    throw BridgeError.protocol(
      'STUDY_DTO_FIELD_MISSING',
      'Study DTO is missing required field $key.',
    );
  }
  final text = value is String ? value : value.toString();
  if (text.trim().isEmpty) {
    throw BridgeError.protocol(
      'STUDY_DTO_FIELD_EMPTY',
      'Study DTO field $key is empty.',
    );
  }
  return text;
}

bool _isChoiceQuestionType(String questionType) =>
    questionType == 'enToCnChoice' ||
    questionType == 'exampleToCnChoice' ||
    questionType == 'exampleToCnChoiceNoTranslation' ||
    questionType == 'cnToEnChoice';

bool _hasCompleteStudyQuestion(Object? value) {
  if (value is! Map<String, dynamic>) return false;
  return _jsonString(value['questionId']).isNotEmpty &&
      _jsonString(value['questionType']).isNotEmpty &&
      _jsonString(value['entrySourceId']).isNotEmpty;
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
        sessionId: _jsonString(json['sessionId']),
        totalQuestions: _jsonInt(json['totalQuestions']),
        correctCount: _jsonInt(json['correctCount']),
        fuzzyCorrectCount: _jsonInt(json['fuzzyCorrectCount']),
        incorrectCount: _jsonInt(json['incorrectCount']),
        skippedCount: _jsonInt(json['skippedCount']),
        totalWords: _jsonInt(json['totalWords']),
        wrongWordCount: _jsonInt(json['wrongWordCount']),
        accuracyPercent: _jsonDouble(json['accuracyPercent']),
        totalTimeMs: _jsonInt(json['totalTimeMs']),
        completedAt: _jsonString(
          json['completedAt'],
          fallback: DateTime.now().toIso8601String(),
        ),
      );
}

/// Response from starting a study session.
class StartSessionResponse {
  final StudySession session;
  final StudyQuestion currentQuestion;
  final SessionProgress progress;
  final List<AnsweredStudyQuestion> answeredQuestions;

  const StartSessionResponse({
    required this.session,
    required this.currentQuestion,
    required this.progress,
    this.answeredQuestions = const [],
  });

  factory StartSessionResponse.fromJson(Map<String, dynamic> json) =>
      StartSessionResponse(
        session: StudySession.fromJson(json['session'] as Map<String, dynamic>),
        currentQuestion:
            StudyQuestion.fromJson(json['currentQuestion'] as Map<String, dynamic>),
        progress:
            SessionProgress.fromJson(json['progress'] as Map<String, dynamic>),
        answeredQuestions: _answeredStudyQuestionList(json['answeredQuestions']),
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
  final HintPrompt? hintPrompt;
  final List<AnsweredStudyQuestion> answeredQuestions;

  const SubmitAnswerResponse({
    required this.result,
    required this.isComplete,
    this.currentQuestion,
    this.summary,
    this.nextAction,
    required this.progress,
    this.hintPrompt,
    this.answeredQuestions = const [],
  });

  factory SubmitAnswerResponse.fromJson(Map<String, dynamic> json) =>
      SubmitAnswerResponse(
        result: StudyResult.fromJson(json['result'] as Map<String, dynamic>),
        isComplete: _jsonBool(json['isComplete']),
        currentQuestion: _hasCompleteStudyQuestion(json['currentQuestion'])
            ? StudyQuestion.fromJson(json['currentQuestion'] as Map<String, dynamic>)
            : null,
        summary: json['summary'] is Map<String, dynamic>
            ? SessionSummary.fromJson(json['summary'] as Map<String, dynamic>)
            : null,
        nextAction: json['nextAction'] == null
            ? null
            : _jsonString(json['nextAction']),
        progress:
            SessionProgress.fromJson(json['progress'] as Map<String, dynamic>),
        hintPrompt: json['hintPrompt'] is Map<String, dynamic>
            ? HintPrompt.fromJson(json['hintPrompt'] as Map<String, dynamic>)
            : null,
        answeredQuestions: _answeredStudyQuestionList(json['answeredQuestions']),
      );
}

List<AnsweredStudyQuestion> _answeredStudyQuestionList(Object? value) {
  if (value is! List) return const [];
  return value
      .whereType<Map<String, dynamic>>()
      .map(AnsweredStudyQuestion.fromJson)
      .toList(growable: false);
}

class MarkStudyEntryMasteredResponse {
  final String entrySourceId;
  final int? entryId;
  final int prunedQuestionCount;
  final bool isComplete;
  final StudyQuestion? currentQuestion;
  final SessionSummary? summary;
  final String? nextAction;
  final SessionProgress progress;
  final List<AnsweredStudyQuestion> answeredQuestions;

  const MarkStudyEntryMasteredResponse({
    required this.entrySourceId,
    this.entryId,
    required this.prunedQuestionCount,
    required this.isComplete,
    this.currentQuestion,
    this.summary,
    this.nextAction,
    required this.progress,
    this.answeredQuestions = const [],
  });

  factory MarkStudyEntryMasteredResponse.fromJson(Map<String, dynamic> json) =>
      MarkStudyEntryMasteredResponse(
        entrySourceId: _jsonString(json['entrySourceId']),
        entryId: json['entryId'] is int ? json['entryId'] as int : null,
        prunedQuestionCount: _jsonInt(json['prunedQuestionCount']),
        isComplete: _jsonBool(json['isComplete']),
        currentQuestion: _hasCompleteStudyQuestion(json['currentQuestion'])
            ? StudyQuestion.fromJson(json['currentQuestion'] as Map<String, dynamic>)
            : null,
        summary: json['summary'] is Map<String, dynamic>
            ? SessionSummary.fromJson(json['summary'] as Map<String, dynamic>)
            : null,
        nextAction: json['nextAction'] == null
            ? null
            : _jsonString(json['nextAction']),
        progress:
            SessionProgress.fromJson(json['progress'] as Map<String, dynamic>),
        answeredQuestions: _answeredStudyQuestionList(json['answeredQuestions']),
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
        nextAction: _jsonString(json['nextAction'], fallback: 'Return to today'),
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
    List<Map<String, dynamic>>? questionTypeWeights,
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
    if (questionTypeWeights != null) {
      request['questionTypeWeights'] = questionTypeWeights;
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

  Future<MarkStudyEntryMasteredResponse> markEntryMastered({
    required String entrySourceId,
    String reason = 'mastered',
  }) async {
    final request = <String, dynamic>{
      'entrySourceId': entrySourceId,
      'reason': reason,
    };
    final raw = await _bridge.call(
      'markStudyEntryMastered',
      _codec.encodeRequest(request),
    );
    final json = _codec.decodeResponse(raw);
    return MarkStudyEntryMasteredResponse.fromJson(json);
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

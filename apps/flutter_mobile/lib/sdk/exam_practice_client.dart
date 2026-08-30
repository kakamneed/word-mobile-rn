library;

import 'dart:convert';

import '../bridge/bridge.dart';

class ExamQuestionCapabilities {
  const ExamQuestionCapabilities({
    required this.browsable,
    required this.answerable,
    required this.autoGradable,
    required this.causalAnalyzable,
  });

  final bool browsable;
  final bool answerable;
  final bool autoGradable;
  final bool causalAnalyzable;

  factory ExamQuestionCapabilities.fromJson(Map<String, dynamic> json) =>
      ExamQuestionCapabilities(
        browsable: json['browsable'] == true,
        answerable: json['answerable'] == true,
        autoGradable: json['autoGradable'] == true,
        causalAnalyzable: json['causalAnalyzable'] == true,
      );
}

class ExamChoice {
  const ExamChoice({required this.label, required this.text});

  final String label;
  final String text;

  factory ExamChoice.fromJson(Map<String, dynamic> json) =>
      ExamChoice(label: _string(json['label']), text: _string(json['text']));
}

class ExamQuestion {
  const ExamQuestion({
    required this.id,
    required this.number,
    required this.kind,
    required this.stem,
    required this.choices,
    required this.answer,
    required this.explanation,
    required this.capabilities,
  });

  final String id;
  final int number;
  final String kind;
  final String stem;
  final List<ExamChoice> choices;
  final String? answer;
  final String explanation;
  final ExamQuestionCapabilities capabilities;

  bool get hasAnswer => answer?.trim().isNotEmpty == true;

  factory ExamQuestion.fromJson(Map<String, dynamic> json) => ExamQuestion(
    id: _string(json['id']),
    number: _integer(json['number']),
    kind: _string(json['kind'], fallback: 'subjective'),
    stem: _string(json['stem']),
    choices: _maps(
      json['choices'],
    ).map(ExamChoice.fromJson).toList(growable: false),
    answer: _nullableString(json['answer']),
    explanation: _string(json['explanation']),
    capabilities: ExamQuestionCapabilities.fromJson(_map(json['capabilities'])),
  );
}

class ExamSection {
  const ExamSection({
    required this.id,
    required this.type,
    required this.title,
    required this.instructions,
    required this.passage,
    this.paragraphTranslations = const [],
    required this.questions,
  });

  final String id;
  final String type;
  final String title;
  final String instructions;
  final String passage;
  final List<String> paragraphTranslations;
  final List<ExamQuestion> questions;

  factory ExamSection.fromJson(Map<String, dynamic> json) => ExamSection(
    id: _string(json['id']),
    type: _string(json['type']),
    title: _string(json['title']),
    instructions: _string(json['instructions']),
    passage: _string(json['passage']),
    paragraphTranslations:
        (json['paragraphTranslations'] as List<dynamic>? ?? const [])
            .map((item) => '$item'.trim())
            .where((item) => item.isNotEmpty)
            .toList(growable: false),
    questions: _maps(
      json['questions'],
    ).map(ExamQuestion.fromJson).toList(growable: false),
  );
}

class ExamPaper {
  const ExamPaper({
    required this.id,
    required this.exam,
    required this.title,
    required this.year,
    required this.month,
    required this.setNumber,
    required this.sections,
  });

  final String id;
  final String exam;
  final String title;
  final int year;
  final int? month;
  final int? setNumber;
  final List<ExamSection> sections;

  factory ExamPaper.fromJson(Map<String, dynamic> json) => ExamPaper(
    id: _string(json['id']),
    exam: _string(json['exam']),
    title: _string(json['title']),
    year: _integer(json['year']),
    month: _nullableInteger(json['month']),
    setNumber: _nullableInteger(json['set']),
    sections: _maps(
      json['sections'],
    ).map(ExamSection.fromJson).toList(growable: false),
  );
}

class ExamSectionSummary {
  const ExamSectionSummary({
    required this.id,
    required this.type,
    required this.title,
    required this.questionCount,
    required this.autoGradableCount,
  });

  final String id;
  final String type;
  final String title;
  final int questionCount;
  final int autoGradableCount;

  factory ExamSectionSummary.fromJson(Map<String, dynamic> json) =>
      ExamSectionSummary(
        id: _string(json['id']),
        type: _string(json['sectionType']),
        title: _string(json['title']),
        questionCount: _integer(json['questionCount']),
        autoGradableCount: _integer(json['autoGradableCount']),
      );
}

class ExamPaperSummary {
  const ExamPaperSummary({
    required this.id,
    required this.title,
    required this.year,
    required this.month,
    required this.setNumber,
    required this.questionCount,
    required this.answerBearingCount,
    required this.autoGradableCount,
    required this.origin,
    required this.sections,
  });

  final String id;
  final String title;
  final int year;
  final int? month;
  final int? setNumber;
  final int questionCount;
  final int answerBearingCount;
  final int autoGradableCount;
  final String origin;
  final List<ExamSectionSummary> sections;

  factory ExamPaperSummary.fromJson(Map<String, dynamic> json) =>
      ExamPaperSummary(
        id: _string(json['id']),
        title: _string(json['title']),
        year: _integer(json['year']),
        month: _nullableInteger(json['month']),
        setNumber: _nullableInteger(json['set']),
        questionCount: _integer(json['questionCount']),
        answerBearingCount: _integer(json['answerBearingCount']),
        autoGradableCount: _integer(json['autoGradableCount']),
        origin: _string(json['origin'], fallback: 'bundled'),
        sections: _maps(
          json['sections'],
        ).map(ExamSectionSummary.fromJson).toList(growable: false),
      );
}

class ExamCatalogExam {
  const ExamCatalogExam({required this.exam, required this.papers});

  final String exam;
  final List<ExamPaperSummary> papers;

  factory ExamCatalogExam.fromJson(Map<String, dynamic> json) =>
      ExamCatalogExam(
        exam: _string(json['exam']),
        papers: _maps(
          json['papers'],
        ).map(ExamPaperSummary.fromJson).toList(growable: false),
      );
}

class ExamCatalog {
  const ExamCatalog({required this.schemaVersion, required this.exams});

  final int schemaVersion;
  final List<ExamCatalogExam> exams;

  factory ExamCatalog.fromJson(Map<String, dynamic> json) => ExamCatalog(
    schemaVersion: _integer(json['schemaVersion']),
    exams: _maps(
      json['exams'],
    ).map(ExamCatalogExam.fromJson).toList(growable: false),
  );
}

class ExamAttempt {
  const ExamAttempt({
    required this.attemptId,
    required this.paperId,
    required this.sectionId,
    required this.questionId,
    required this.selectedAnswer,
    required this.isCorrect,
    required this.answerHistory,
    required this.status,
    required this.startedAt,
    required this.updatedAt,
    this.vocabularyEvidence = const [],
  });

  final String attemptId;
  final String paperId;
  final String sectionId;
  final String questionId;
  final String? selectedAnswer;
  final bool? isCorrect;
  final List<String> answerHistory;
  final String status;
  final String startedAt;
  final String updatedAt;
  final List<Map<String, dynamic>> vocabularyEvidence;

  factory ExamAttempt.fromJson(Map<String, dynamic> json) {
    final historyRaw = _string(json['answerHistoryJson'], fallback: '[]');
    dynamic decodedHistory;
    try {
      decodedHistory = jsonDecode(historyRaw);
    } on FormatException {
      decodedHistory = const [];
    }
    return ExamAttempt(
      attemptId: _string(json['attemptId']),
      paperId: _string(json['paperId']),
      sectionId: _string(json['sectionId']),
      questionId: _string(json['questionId']),
      selectedAnswer: _nullableString(json['selectedAnswer']),
      isCorrect: json['isCorrect'] is bool ? json['isCorrect'] as bool : null,
      answerHistory: decodedHistory is List
          ? decodedHistory.whereType<String>().toList(growable: false)
          : const [],
      status: _string(json['status'], fallback: 'in_progress'),
      startedAt: _string(json['startedAt']),
      updatedAt: _string(json['updatedAt']),
      vocabularyEvidence: _maps(
        json['vocabularyEvidence'],
      ).toList(growable: false),
    );
  }
}

class ExamWordToken {
  const ExamWordToken({
    required this.text,
    required this.normalized,
    required this.startOffset,
    required this.endOffset,
  });

  final String text;
  final String normalized;
  final int startOffset;
  final int endOffset;

  factory ExamWordToken.fromJson(Map<String, dynamic> json) => ExamWordToken(
    text: _string(json['text']),
    normalized: _string(json['normalized']),
    startOffset: _integer(json['startOffset']),
    endOffset: _integer(json['endOffset']),
  );
}

List<ExamWordToken> _normalizeExamWordTokenOffsets(
  String source,
  List<ExamWordToken> tokens, {
  required bool declaredUtf16,
}) {
  bool matches(ExamWordToken token, int start, int end) =>
      start >= 0 &&
      end >= start &&
      end <= source.length &&
      source.substring(start, end) == token.text;

  if (declaredUtf16 ||
      tokens.every(
        (token) => matches(token, token.startOffset, token.endOffset),
      )) {
    return tokens
        .where((token) => matches(token, token.startOffset, token.endOffset))
        .toList(growable: false);
  }

  var byteOffset = 0;
  var codeUnitOffset = 0;
  final codeUnitByUtf8Byte = <int, int>{0: 0};
  for (final rune in source.runes) {
    byteOffset += utf8.encode(String.fromCharCode(rune)).length;
    codeUnitOffset += rune > 0xFFFF ? 2 : 1;
    codeUnitByUtf8Byte[byteOffset] = codeUnitOffset;
  }
  return tokens
      .map((token) {
        final start = codeUnitByUtf8Byte[token.startOffset];
        final end = codeUnitByUtf8Byte[token.endOffset];
        if (start == null || end == null || !matches(token, start, end)) {
          return null;
        }
        return ExamWordToken(
          text: token.text,
          normalized: token.normalized,
          startOffset: start,
          endOffset: end,
        );
      })
      .whereType<ExamWordToken>()
      .toList(growable: false);
}

class ExamWordInspection {
  const ExamWordInspection({
    required this.occurrenceId,
    required this.entryId,
    required this.word,
    required this.normalized,
    required this.meanings,
    required this.userMark,
    required this.isUnknown,
  });

  final int occurrenceId;
  final int? entryId;
  final String word;
  final String normalized;
  final List<String> meanings;
  final String userMark;
  final bool isUnknown;

  factory ExamWordInspection.fromJson(Map<String, dynamic> json) =>
      ExamWordInspection(
        occurrenceId: _integer(json['occurrenceId']),
        entryId: _nullableInteger(json['entryId']),
        word: _string(json['word']),
        normalized: _string(json['normalized']),
        meanings:
            (json['meanings'] is List ? json['meanings'] as List : const [])
                .whereType<String>()
                .toList(growable: false),
        userMark: _string(json['userMark'], fallback: 'none'),
        isUnknown: json['isUnknown'] == true,
      );
}

class ExamTextAnnotation {
  const ExamTextAnnotation({
    required this.annotationId,
    required this.questionId,
    required this.scope,
    required this.startOffset,
    required this.endOffset,
    required this.selectedText,
    required this.noteText,
    required this.color,
  });

  final String annotationId;
  final String? questionId;
  final String scope;
  final int startOffset;
  final int endOffset;
  final String selectedText;
  final String noteText;
  final String color;

  factory ExamTextAnnotation.fromJson(Map<String, dynamic> json) =>
      ExamTextAnnotation(
        annotationId: _string(json['annotationId']),
        questionId: _nullableString(json['questionId']),
        scope: _string(json['scope']),
        startOffset: _integer(json['startOffset']),
        endOffset: _integer(json['endOffset']),
        selectedText: _string(json['selectedText']),
        noteText: _string(json['noteText']),
        color: _string(json['color'], fallback: 'yellow'),
      );
}

class ExamAnnotationState {
  const ExamAnnotationState({
    required this.currentMeanings,
    required this.currentMarks,
    required this.priorMarks,
    required this.priorWords,
    required this.causalWords,
    required this.annotations,
  });

  final Map<String, String> currentMeanings;
  final Map<String, String> currentMarks;
  final Map<String, String> priorMarks;
  Set<String> get currentWords => currentMeanings.keys.toSet();
  final Set<String> priorWords;
  final Set<String> causalWords;
  final List<ExamTextAnnotation> annotations;

  factory ExamAnnotationState.fromJson(Map<String, dynamic> json) {
    Map<String, String> meaningsFrom(dynamic value) => {
      for (final item in _maps(value))
        if (_string(item['normalized']).isNotEmpty)
          _string(item['normalized']): _string(item['meaning']),
    };
    final current = meaningsFrom(json['currentMarks']);
    final marks = {
      for (final item in _maps(json['currentMarks']))
        if (_string(item['normalized']).isNotEmpty)
          _string(item['normalized']): _string(
            item['mark'],
            fallback: 'unknown',
          ),
    };
    final prior = meaningsFrom(json['priorMarks']);
    final priorMarks = {
      for (final item in _maps(json['priorMarks']))
        if (_string(item['normalized']).isNotEmpty)
          _string(item['normalized']): _string(
            item['mark'],
            fallback: 'unknown',
          ),
    };
    return ExamAnnotationState(
      currentMeanings: current,
      currentMarks: marks,
      priorMarks: priorMarks,
      priorWords: prior.keys.toSet(),
      causalWords: (json['causalWords'] as List<dynamic>? ?? const [])
          .map(_string)
          .where((word) => word.isNotEmpty)
          .toSet(),
      annotations: _maps(
        json['annotations'],
      ).map(ExamTextAnnotation.fromJson).toList(growable: false),
    );
  }
}

class ExamAnalysisFinding {
  const ExamAnalysisFinding({
    required this.questionNumber,
    required this.word,
    required this.reasoning,
    required this.confidence,
  });

  final int questionNumber;
  final String word;
  final String reasoning;
  final double confidence;

  factory ExamAnalysisFinding.fromJson(Map<String, dynamic> json) =>
      ExamAnalysisFinding(
        questionNumber: _integer(json['questionNumber']),
        word: _string(json['word']),
        reasoning: _string(json['reasoning']),
        confidence: (json['confidence'] as num?)?.toDouble() ?? 0,
      );

  Map<String, dynamic> toJson() => {
    'questionNumber': questionNumber,
    'word': word,
    'reasoning': reasoning,
    'confidence': confidence,
  };
}

class ExamAnalysisAttempt {
  const ExamAnalysisAttempt({
    required this.questionId,
    required this.attemptId,
  });

  final String questionId;
  final String attemptId;

  Map<String, String> toJson() => {
    'questionId': questionId,
    'attemptId': attemptId,
  };
}

class ExamAnalysisTask {
  const ExamAnalysisTask({
    required this.taskId,
    required this.exam,
    required this.paperId,
    required this.paperTitle,
    required this.sectionId,
    required this.sectionTitle,
    required this.status,
    required this.unread,
    required this.createdAt,
    required this.completedAt,
    required this.findings,
    required this.review,
    required this.error,
  });

  final String taskId;
  final String exam;
  final String paperId;
  final String paperTitle;
  final String sectionId;
  final String sectionTitle;
  final String status;
  final bool unread;
  final String createdAt;
  final String completedAt;
  final List<ExamAnalysisFinding> findings;
  final Map<String, dynamic> review;
  final String error;

  bool get isCompleted => status == 'completed';
  bool get isRunning => status == 'running';

  factory ExamAnalysisTask.fromJson(Map<String, dynamic> json) =>
      ExamAnalysisTask(
        taskId: _string(json['taskId']),
        exam: _string(json['exam']),
        paperId: _string(json['paperId']),
        paperTitle: _string(json['paperTitle']),
        sectionId: _string(json['sectionId']),
        sectionTitle: _string(json['sectionTitle']),
        status: _string(json['status'], fallback: 'failed'),
        unread: json['unread'] == true,
        createdAt: _string(json['createdAt']),
        completedAt: _string(json['completedAt']),
        findings: _maps(
          json['findings'],
        ).map(ExamAnalysisFinding.fromJson).toList(growable: false),
        review: _normalizeExamAnalysisReview(json['review']),
        error: _string(json['error']),
      );
}

class ExamAnalysisInbox {
  const ExamAnalysisInbox({required this.items, required this.unreadCount});

  final List<ExamAnalysisTask> items;
  final int unreadCount;

  factory ExamAnalysisInbox.fromJson(Map<String, dynamic> json) =>
      ExamAnalysisInbox(
        items: _maps(
          json['items'],
        ).map(ExamAnalysisTask.fromJson).toList(growable: false),
        unreadCount: _integer(json['unreadCount']),
      );
}

class ExamPracticeClient {
  const ExamPracticeClient(this._bridge, this._codec);

  final RustBridge _bridge;
  final BridgeCodec _codec;

  Future<ExamCatalog> getCatalog() async {
    final raw = await _bridge.call('getExamCatalog');
    return ExamCatalog.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamPaper> getPaper({
    required String exam,
    required String paperId,
  }) async {
    final raw = await _bridge.call(
      'getExamPaper',
      _codec.encodeRequest({'exam': exam, 'paperId': paperId}),
    );
    return ExamPaper.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamPaperImportDraft> analyzeImport({
    required String sourceType,
    required String sourceName,
    String? textContent,
    String? bytesBase64,
    String? mimeType,
  }) async {
    final raw = await _bridge.call(
      'analyzeExamPaperImport',
      _codec.encodeRequest({
        'sourceType': sourceType,
        'sourceName': sourceName,
        'textContent': ?textContent,
        'bytesBase64': ?bytesBase64,
        'mimeType': ?mimeType,
      }),
    );
    return ExamPaperImportDraft.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamPaper> saveImportedPaper(ExamPaperImportDraft draft) async {
    final raw = await _bridge.call(
      'saveUserExamPaper',
      _codec.encodeRequest({'paper': draft.paper, 'warnings': draft.warnings}),
    );
    return ExamPaper.fromJson(_map(_codec.decodeResponse(raw)['paper']));
  }

  Future<List<ExamVocabularyPriority>> getVocabularyPriority() async {
    final raw = await _bridge.call('getExamVocabularyPriority');
    final json = _codec.decodeResponse(raw);
    return _maps(
      json['items'],
    ).map(ExamVocabularyPriority.fromJson).toList(growable: false);
  }

  Future<ExamPracticeReport> getPracticeReport() async {
    final raw = await _bridge.call('getExamPracticeReport');
    return ExamPracticeReport.fromJson(_codec.decodeResponse(raw));
  }

  Future<Map<String, dynamic>> analyzeQuestionVocabulary({
    required String exam,
    required String paperId,
    required String sectionId,
    required String questionId,
    required String attemptId,
  }) async {
    final raw = await _bridge.call(
      'analyzeExamQuestionVocabulary',
      _codec.encodeRequest({
        'exam': exam,
        'paperId': paperId,
        'sectionId': sectionId,
        'questionId': questionId,
        'attemptId': attemptId,
      }),
    );
    return _codec.decodeResponse(raw);
  }

  Future<Map<String, dynamic>> analyzeSectionVocabulary({
    required String exam,
    required String paperId,
    required String sectionId,
    required List<ExamAnalysisAttempt> attempts,
    Set<String> purePurpleWords = const {},
  }) async {
    final raw = await _bridge.call(
      'analyzeExamSectionVocabulary',
      _codec.encodeRequest({
        'exam': exam,
        'paperId': paperId,
        'sectionId': sectionId,
        'attempts': attempts
            .map((attempt) => attempt.toJson())
            .toList(growable: false),
        'purePurpleWords': purePurpleWords.toList(growable: false)..sort(),
      }),
    );
    return _codec.decodeResponse(raw);
  }

  Future<ExamAnalysisTask> saveAnalysisTask({
    required String taskId,
    required String exam,
    required String paperId,
    required String paperTitle,
    required String sectionId,
    required String sectionTitle,
    required String status,
    required String createdAt,
    String completedAt = '',
    List<ExamAnalysisFinding> findings = const [],
    Map<String, dynamic> review = const {},
    String error = '',
  }) async {
    final raw = await _bridge.call(
      'saveExamAnalysisTask',
      _codec.encodeRequest({
        'taskId': taskId,
        'exam': exam,
        'paperId': paperId,
        'paperTitle': paperTitle,
        'sectionId': sectionId,
        'sectionTitle': sectionTitle,
        'status': status,
        'createdAt': createdAt,
        'completedAt': completedAt,
        'findings': findings.map((finding) => finding.toJson()).toList(),
        'review': review,
        'error': error,
      }),
    );
    return ExamAnalysisTask.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamAnalysisInbox> getAnalysisTasks() async {
    final raw = await _bridge.call('getExamAnalysisTasks');
    return ExamAnalysisInbox.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamAnalysisInbox> markAnalysisTasksRead() async {
    final raw = await _bridge.call('markExamAnalysisTasksRead');
    return ExamAnalysisInbox.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamAttempt> saveAttempt({
    required String attemptId,
    required String paperId,
    required String sectionId,
    required String questionId,
    String? selectedAnswer,
    bool? isCorrect,
    List<String> answerHistory = const [],
    String status = 'in_progress',
  }) async {
    final raw = await _bridge.call(
      'saveExamAttempt',
      _codec.encodeRequest({
        'attemptId': attemptId,
        'paperId': paperId,
        'sectionId': sectionId,
        'questionId': questionId,
        'selectedAnswer': selectedAnswer,
        'isCorrect': isCorrect,
        'answerHistory': answerHistory,
        'status': status,
      }),
    );
    return ExamAttempt.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamAttempt?> getAttempt(String attemptId) async {
    final raw = await _bridge.call(
      'getExamAttempt',
      _codec.encodeRequest({'attemptId': attemptId}),
    );
    final json = _codec.decodeResponse(raw);
    final attempt = json['attempt'];
    return attempt is Map<String, dynamic>
        ? ExamAttempt.fromJson(attempt)
        : null;
  }

  Future<List<ExamWordToken>> tokenizeText(String text) async {
    final raw = await _bridge.call(
      'tokenizeExamText',
      _codec.encodeRequest({'text': text}),
    );
    final json = _codec.decodeResponse(raw);
    final tokens = _maps(
      json['tokens'],
    ).map(ExamWordToken.fromJson).toList(growable: false);
    return _normalizeExamWordTokenOffsets(
      text,
      tokens,
      declaredUtf16: json['offsetEncoding'] == 'utf16',
    );
  }

  Future<ExamWordInspection> inspectWord({
    required String articleId,
    required String title,
    required String body,
    required ExamWordToken token,
    required String sentenceText,
    required Map<String, dynamic> metadata,
    String sourceType = 'builtin',
    String? userMark,
  }) async {
    final raw = await _bridge.call(
      'inspectExamWord',
      _codec.encodeRequest({
        'articleId': articleId,
        'sourceType': sourceType,
        'title': title,
        'body': body,
        'word': token.text,
        'startOffset': token.startOffset,
        'endOffset': token.endOffset,
        'sentenceText': sentenceText,
        'metadata': metadata,
        'userMark': ?userMark,
      }),
    );
    return ExamWordInspection.fromJson(_codec.decodeResponse(raw));
  }

  Future<ExamAnnotationState> getAnnotationState({
    required String articleId,
    required Set<String> words,
  }) async {
    final raw = await _bridge.call(
      'getExamAnnotationState',
      _codec.encodeRequest({'articleId': articleId, 'words': words.toList()}),
    );
    return ExamAnnotationState.fromJson(_codec.decodeResponse(raw));
  }

  Future<List<ExamTextAnnotation>> saveAnnotation({
    required String annotationId,
    required String articleId,
    required String title,
    required String body,
    required String scope,
    required int startOffset,
    required int endOffset,
    required String selectedText,
    required String noteText,
    required Map<String, dynamic> metadata,
    String? questionId,
    String sourceType = 'builtin',
  }) async {
    final raw = await _bridge.call(
      'saveExamAnnotation',
      _codec.encodeRequest({
        'annotationId': annotationId,
        'articleId': articleId,
        'sourceType': sourceType,
        'title': title,
        'body': body,
        'questionId': questionId,
        'scope': scope,
        'startOffset': startOffset,
        'endOffset': endOffset,
        'selectedText': selectedText,
        'noteText': noteText,
        'metadata': metadata,
      }),
    );
    return _maps(
      _codec.decodeResponse(raw)['annotations'],
    ).map(ExamTextAnnotation.fromJson).toList(growable: false);
  }
}

class ExamPaperImportDraft {
  const ExamPaperImportDraft({
    required this.paper,
    required this.warnings,
    required this.provenance,
    required this.rawMediaRetained,
  });

  final Map<String, dynamic> paper;
  final List<String> warnings;
  final Map<String, dynamic> provenance;
  final bool rawMediaRetained;

  factory ExamPaperImportDraft.fromJson(Map<String, dynamic> json) =>
      ExamPaperImportDraft(
        paper: Map<String, dynamic>.from(_map(json['paper'])),
        warnings: (json['warnings'] as List<dynamic>? ?? const [])
            .map((item) => '$item')
            .toList(growable: false),
        provenance: Map<String, dynamic>.from(_map(json['provenance'])),
        rawMediaRetained: json['rawMediaRetained'] == true,
      );
}

class ExamVocabularyPriority {
  const ExamVocabularyPriority({
    required this.word,
    required this.priorityScore,
    required this.paperCount,
    required this.articleCount,
    required this.occurrenceCount,
    this.fuzzyMarkCount = 0,
    this.familiarMarkCount = 0,
    required this.unknownMarkCount,
    required this.wrongAssociationCount,
    required this.masteredMarkCount,
    required this.factors,
    required this.sources,
  });

  final String word;
  final double priorityScore;
  final int paperCount;
  final int articleCount;
  final int occurrenceCount;
  final int fuzzyMarkCount;
  final int familiarMarkCount;
  final int unknownMarkCount;
  final int wrongAssociationCount;
  final int masteredMarkCount;
  final List<Map<String, dynamic>> factors;
  final List<ExamVocabularySource> sources;

  factory ExamVocabularyPriority.fromJson(Map<String, dynamic> json) =>
      ExamVocabularyPriority(
        word: _string(json['word']),
        priorityScore: (json['priorityScore'] as num?)?.toDouble() ?? 0,
        paperCount: _integer(json['paperCount']),
        articleCount: _integer(json['articleCount']),
        occurrenceCount: _integer(json['occurrenceCount']),
        fuzzyMarkCount: _integer(
          json['fuzzyMarkCount'] ?? json['uncertainMarkCount'],
        ),
        familiarMarkCount: _integer(json['familiarMarkCount']),
        unknownMarkCount: _integer(json['unknownMarkCount']),
        wrongAssociationCount: _integer(json['wrongAssociationCount']),
        masteredMarkCount: _integer(json['masteredMarkCount']),
        factors: _maps(json['factors']).toList(growable: false),
        sources: _maps(
          json['sources'],
        ).map(ExamVocabularySource.fromJson).toList(growable: false),
      );
}

class ExamVocabularySource {
  const ExamVocabularySource({
    required this.paperId,
    required this.paperTitle,
    required this.sectionId,
    required this.sectionTitle,
    this.fuzzyMarkCount = 0,
    this.familiarMarkCount = 0,
    required this.unknownMarkCount,
    required this.wrongAssociationCount,
  });

  final String paperId;
  final String paperTitle;
  final String sectionId;
  final String sectionTitle;
  final int fuzzyMarkCount;
  final int familiarMarkCount;
  final int unknownMarkCount;
  final int wrongAssociationCount;

  factory ExamVocabularySource.fromJson(Map<String, dynamic> json) =>
      ExamVocabularySource(
        paperId: _string(json['paperId']),
        paperTitle: _string(json['paperTitle']),
        sectionId: _string(json['sectionId']),
        sectionTitle: _string(json['sectionTitle']),
        fuzzyMarkCount: _integer(
          json['fuzzyMarkCount'] ?? json['uncertainMarkCount'],
        ),
        familiarMarkCount: _integer(json['familiarMarkCount']),
        unknownMarkCount: _integer(json['unknownMarkCount']),
        wrongAssociationCount: _integer(json['wrongAssociationCount']),
      );
}

class ExamPracticeAccuracy {
  const ExamPracticeAccuracy({
    required this.correctCount,
    required this.totalQuestions,
    required this.accuracyPercent,
  });

  final int correctCount;
  final int totalQuestions;
  final double accuracyPercent;

  factory ExamPracticeAccuracy.fromJson(Map<String, dynamic> json) =>
      ExamPracticeAccuracy(
        correctCount: _integer(json['correctCount']),
        totalQuestions: _integer(json['totalQuestions']),
        accuracyPercent: (json['accuracyPercent'] as num?)?.toDouble() ?? 0,
      );
}

class ExamPracticePaperReport extends ExamPracticeAccuracy {
  const ExamPracticePaperReport({
    required this.paperId,
    required this.title,
    required this.year,
    required super.correctCount,
    required super.totalQuestions,
    required super.accuracyPercent,
    required this.questionTypes,
  });

  final String paperId;
  final String title;
  final int year;
  final Map<String, ExamPracticeAccuracy> questionTypes;

  factory ExamPracticePaperReport.fromJson(Map<String, dynamic> json) {
    final rawTypes = _map(json['questionTypes']);
    return ExamPracticePaperReport(
      paperId: _string(json['paperId']),
      title: _string(json['title']),
      year: _integer(json['year']),
      correctCount: _integer(json['correctCount']),
      totalQuestions: _integer(json['totalQuestions']),
      accuracyPercent: (json['accuracyPercent'] as num?)?.toDouble() ?? 0,
      questionTypes: {
        for (final entry in rawTypes.entries)
          if (entry.value is Map<String, dynamic>)
            entry.key: ExamPracticeAccuracy.fromJson(
              entry.value as Map<String, dynamic>,
            ),
      },
    );
  }
}

class ExamPracticeExamReport {
  const ExamPracticeExamReport({required this.exam, required this.papers});

  final String exam;
  final List<ExamPracticePaperReport> papers;

  factory ExamPracticeExamReport.fromJson(Map<String, dynamic> json) =>
      ExamPracticeExamReport(
        exam: _string(json['exam']),
        papers: _maps(
          json['papers'],
        ).map(ExamPracticePaperReport.fromJson).toList(growable: false),
      );
}

class ExamPracticeReport {
  const ExamPracticeReport({required this.exams});

  final List<ExamPracticeExamReport> exams;

  factory ExamPracticeReport.fromJson(Map<String, dynamic> json) =>
      ExamPracticeReport(
        exams: _maps(
          json['exams'],
        ).map(ExamPracticeExamReport.fromJson).toList(growable: false),
      );
}

Map<String, dynamic> _map(dynamic value) =>
    value is Map<String, dynamic> ? value : const <String, dynamic>{};

Map<String, dynamic> _normalizeExamAnalysisReview(dynamic value) {
  final source = _map(value);
  if (source.isEmpty) return const <String, dynamic>{};
  final review = Map<String, dynamic>.from(source);
  for (final key in const [
    'questions',
    'correctMarkedQuestions',
    'vocabularyPriority',
    'limitations',
  ]) {
    if (review[key] is! List) review[key] = <dynamic>[];
  }
  review['questions'] = (review['questions'] as List<dynamic>)
      .whereType<Map<String, dynamic>>()
      .map((question) {
        final normalized = Map<String, dynamic>.from(question);
        if (normalized['optionAnalysis'] is! List) {
          normalized['optionAnalysis'] = <dynamic>[];
        }
        if (normalized['candidates'] is! List) {
          normalized['candidates'] = <dynamic>[];
        }
        return normalized;
      })
      .toList(growable: false);
  return review;
}

Iterable<Map<String, dynamic>> _maps(dynamic value) =>
    value is List ? value.whereType<Map<String, dynamic>>() : const [];

String _string(dynamic value, {String fallback = ''}) =>
    value is String ? value : fallback;

String? _nullableString(dynamic value) =>
    value is String && value.trim().isNotEmpty ? value : null;

int _integer(dynamic value) =>
    value is num ? value.toInt() : int.tryParse('$value') ?? 0;

int? _nullableInteger(dynamic value) => value == null ? null : _integer(value);

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'exam_analysis_task_notifications.dart';

enum TodayLearningContent { words, practice }

enum ExamPracticeMode { doing, analysis }

enum ExamReadingPane { passage, questions }

const examSectionAnalysisTimeout = Duration(seconds: 360);

enum ExamWordHighlight { none, priorArticle, fuzzy, familiar, unknown, mixed }

const examFamiliarityLevels = ['fuzzy', 'familiar', 'unknown'];

Color examCurrentHighlightColor(String level) => switch (level) {
  'fuzzy' => const Color(0xFFFFF3BF),
  'familiar' => const Color(0xFFFFDF70),
  _ => const Color(0xFFFFB84D),
};

Color examPriorHighlightColor(String level) => switch (level) {
  'fuzzy' => const Color(0xFFEDE2F7),
  'familiar' => const Color(0xFFD9BDEA),
  _ => const Color(0xFFBE8ADB),
};

String pickExamFamiliarityLevel(Rect paletteRect, Offset globalPosition) {
  final relative = (globalPosition.dx - paletteRect.left).clamp(
    0.0,
    paletteRect.width,
  );
  final index = (relative / (paletteRect.width / 3)).floor().clamp(0, 2);
  return examFamiliarityLevels[index];
}

String normalizeExamWordFamily(String value) {
  final word = normalizeExamPhraseSelection(value);
  if (word.contains(' ')) return word;
  const irregular = {
    'children': 'child',
    'people': 'person',
    'men': 'man',
    'women': 'woman',
    'mice': 'mouse',
    'feet': 'foot',
    'teeth': 'tooth',
    'geese': 'goose',
    'went': 'go',
    'gone': 'go',
    'saw': 'see',
    'seen': 'see',
    'made': 'make',
    'took': 'take',
    'taken': 'take',
    'gave': 'give',
    'given': 'give',
    'found': 'find',
    'thought': 'think',
    'bought': 'buy',
    'brought': 'bring',
    'wrote': 'write',
    'written': 'write',
  };
  if (irregular[word] case final family?) return family;
  const sExceptions = {'news', 'series', 'species', 'means', 'analysis'};
  if (word.endsWith('ies') && word.length > 3) {
    return '${word.substring(0, word.length - 3)}y';
  }
  if (word.endsWith('ing') && word.length > 4) {
    final stem = word.substring(0, word.length - 3);
    if (stem.endsWith('at') || stem.endsWith('iz') || stem.endsWith('bl')) {
      return '${stem}e';
    }
    if (stem.length > 2 && stem[stem.length - 1] == stem[stem.length - 2]) {
      return stem.substring(0, stem.length - 1);
    }
    return stem;
  }
  if (word.endsWith('ed') && word.length > 3) {
    final stem = word.substring(0, word.length - 2);
    if (stem.endsWith('at') || stem.endsWith('iz') || stem.endsWith('or')) {
      return '${stem}e';
    }
    if (stem.length > 2 && stem[stem.length - 1] == stem[stem.length - 2]) {
      return stem.substring(0, stem.length - 1);
    }
    return stem;
  }
  if (word.endsWith('es') && word.length > 3) {
    final withoutEs = word.substring(0, word.length - 2);
    if (RegExp(r'(ss|x|z|ch|sh)$').hasMatch(withoutEs)) return withoutEs;
  }
  if (word.endsWith('s') &&
      word.length > 2 &&
      !word.endsWith('ss') &&
      !sExceptions.contains(word)) {
    return word.substring(0, word.length - 1);
  }
  return word;
}

/// A mark is stored by word family, while older local state and bridge
/// responses can still carry the exact inflected form. Read both forms so a
/// successful lookup is never lost merely because its storage key differs.
String resolveExamMarkedMeaning(
  Map<String, String> meanings,
  String normalized,
) {
  final exact = normalizeExamPhraseSelection(normalized);
  final family = normalizeExamWordFamily(exact);
  final direct = meanings[family] ?? meanings[exact];
  if (direct != null) return direct;
  for (final entry in meanings.entries) {
    if (examWordFamiliesMatch(entry.key, family)) return entry.value;
  }
  return '';
}

Map<String, String> normalizeExamMeaningKeys(Map<String, String> meanings) {
  final normalized = <String, String>{};
  for (final entry in meanings.entries) {
    final key = normalizeExamWordFamily(entry.key);
    if (entry.value.isNotEmpty || !normalized.containsKey(key)) {
      normalized[key] = entry.value;
    }
  }
  return normalized;
}

Set<String> examWordFamilyAliases(String value) {
  final exact = normalizeExamPhraseSelection(value);
  final family = normalizeExamWordFamily(exact);
  // English -ing and -ed forms can omit a silent final e (arouse/arousing).
  // Preserve a reversible lookup key even when the lightweight normalizer
  // cannot know whether a bare stem is a dictionary word by itself.
  return {
    exact,
    family,
    if (family.endsWith('e'))
      family.substring(0, family.length - 1)
    else
      '${family}e',
  };
}

bool examWordFamiliesMatch(String left, String right) => examWordFamilyAliases(
  left,
).intersection(examWordFamilyAliases(right)).isNotEmpty;

String? resolveExamMarkLevel(Map<String, String> marks, String normalized) {
  final family = normalizeExamWordFamily(normalized);
  final direct = marks[family];
  if (direct != null) return direct;
  for (final entry in marks.entries) {
    if (examWordFamiliesMatch(entry.key, family)) return entry.value;
  }
  return null;
}

String normalizeExamPhraseSelection(String value) => value
    .trim()
    .replaceAll(RegExp(r'\s+'), ' ')
    .replaceAll('’', "'")
    .replaceFirst(
      RegExp(r'^[^A-Za-z\u00C0-\u024F\u0300-\u036F\uFB00-\uFB06]+'),
      '',
    )
    .replaceFirst(
      RegExp(r'[^A-Za-z\u00C0-\u024F\u0300-\u036F\uFB00-\uFB06]+$'),
      '',
    )
    .toLowerCase();

List<ExamWordToken> tokenizeExamTextForInteraction(String text) {
  bool isLetter(int codeUnit) =>
      (codeUnit >= 0x41 && codeUnit <= 0x5A) ||
      (codeUnit >= 0x61 && codeUnit <= 0x7A) ||
      (codeUnit >= 0x00C0 && codeUnit <= 0x024F) ||
      (codeUnit >= 0xFB00 && codeUnit <= 0xFB06);
  bool isCombiningMark(int codeUnit) =>
      codeUnit >= 0x0300 && codeUnit <= 0x036F;
  bool isJoiner(int codeUnit) =>
      codeUnit == 0x27 || codeUnit == 0x2D || codeUnit == 0x2019;

  final tokens = <ExamWordToken>[];
  var index = 0;
  while (index < text.length) {
    if (!isLetter(text.codeUnitAt(index))) {
      index += 1;
      continue;
    }
    final start = index;
    index += 1;
    while (index < text.length) {
      final codeUnit = text.codeUnitAt(index);
      if (isLetter(codeUnit) || isCombiningMark(codeUnit)) {
        index += 1;
        continue;
      }
      if (isJoiner(codeUnit) &&
          index + 1 < text.length &&
          isLetter(text.codeUnitAt(index + 1))) {
        index += 1;
        continue;
      }
      break;
    }
    final value = text.substring(start, index);
    tokens.add(
      ExamWordToken(
        text: value,
        normalized: normalizeExamPhraseSelection(value),
        startOffset: start,
        endOffset: index,
      ),
    );
  }
  return tokens;
}

class ExamWordPresentation {
  const ExamWordPresentation({
    required this.highlight,
    required this.showMeaning,
    this.currentMarkLevel,
    this.priorMarkLevel,
  });

  final ExamWordHighlight highlight;
  final bool showMeaning;
  final String? currentMarkLevel;
  final String? priorMarkLevel;

  @override
  bool operator ==(Object other) =>
      other is ExamWordPresentation &&
      other.highlight == highlight &&
      other.showMeaning == showMeaning &&
      other.currentMarkLevel == currentMarkLevel &&
      other.priorMarkLevel == priorMarkLevel;

  @override
  int get hashCode =>
      Object.hash(highlight, showMeaning, currentMarkLevel, priorMarkLevel);
}

ExamWordPresentation resolveExamWordPresentation({
  required String normalized,
  required ExamPracticeMode mode,
  required Set<String> currentArticleMarks,
  required Set<String> priorArticleMarks,
  Map<String, String> currentMarkKinds = const {},
  Map<String, String> priorMarkKinds = const {},
}) {
  final family = normalizeExamWordFamily(normalized);
  final current = currentArticleMarks.any(
    (marked) => examWordFamiliesMatch(marked, family),
  );
  final prior = priorArticleMarks.any(
    (marked) => examWordFamiliesMatch(marked, family),
  );
  final currentLevel =
      resolveExamMarkLevel(currentMarkKinds, family) ?? 'unknown';
  final priorLevel = resolveExamMarkLevel(priorMarkKinds, family) ?? 'unknown';
  if (current && prior) {
    return ExamWordPresentation(
      highlight: ExamWordHighlight.mixed,
      currentMarkLevel: currentLevel,
      priorMarkLevel: priorLevel,
      showMeaning: mode == ExamPracticeMode.analysis,
    );
  }
  if (current) {
    return ExamWordPresentation(
      highlight: switch (currentLevel) {
        'fuzzy' => ExamWordHighlight.fuzzy,
        'familiar' => ExamWordHighlight.familiar,
        _ => ExamWordHighlight.unknown,
      },
      currentMarkLevel: currentLevel,
      showMeaning: mode == ExamPracticeMode.analysis,
    );
  }
  return ExamWordPresentation(
    highlight: prior ? ExamWordHighlight.priorArticle : ExamWordHighlight.none,
    priorMarkLevel: prior ? priorLevel : null,
    showMeaning: false,
  );
}

String nextExamWordMark(String normalized, Set<String> currentArticleMarks) =>
    currentArticleMarks.contains(normalized) ? 'none' : 'unknown';

String pickExamContextMeaning(
  List<String> meanings, {
  String normalized = '',
  String context = '',
  String translation = '',
}) {
  final family = normalizeExamWordFamily(normalized);
  final lowerContext = context.toLowerCase();
  if (family == 'circuit' &&
      RegExp(r'\b(court|federal|appeals?)\b').hasMatch(lowerContext)) {
    return '巡回上诉法院';
  }
  if (family == 'conduct' &&
      RegExp(r'\bconduct(?:ed|ing)?\b').hasMatch(lowerContext) &&
      RegExp(
        r'\b(review|study|analysis|research|experiment|investigation|survey|assessment)\b',
      ).hasMatch(lowerContext)) {
    return '开展；进行';
  }
  final translatedMeaning = _pickMeaningFromTranslation(meanings, translation);
  if (translatedMeaning.isNotEmpty) return translatedMeaning;
  final grammaticalMeaning = _pickMeaningFromGrammar(
    meanings,
    normalized: family,
    context: lowerContext,
  );
  if (grammaticalMeaning.isNotEmpty) return grammaticalMeaning;
  for (final meaning in meanings) {
    final compact = meaning
        .split(RegExp(r'[；;]'))
        .map((part) => part.trim())
        .firstWhere((part) => part.isNotEmpty, orElse: () => '');
    if (compact.isNotEmpty) return compact;
  }
  return '';
}

String _pickMeaningFromGrammar(
  List<String> meanings, {
  required String normalized,
  required String context,
}) {
  if (normalized.isEmpty || context.isEmpty) return '';
  RegExpMatch? match;
  for (final candidate in RegExp(
    r"[a-z]+(?:['’][a-z]+)?",
  ).allMatches(context)) {
    if (normalizeExamWordFamily(candidate.group(0)!) == normalized) {
      match = candidate;
      break;
    }
  }
  if (match == null) return '';
  final before = context.substring(0, match.start).trimRight();
  final after = context.substring(match.end).trimLeft();
  final desired = switch ((before, after, normalized)) {
    (final prefix, _, _)
        when RegExp(
          r'\b(?:to|can|could|will|would|shall|should|may|might|must|do|does|did)\s*$',
        ).hasMatch(prefix) =>
      'verb',
    (final prefix, _, _)
        when RegExp(
          r'\b(?:a|an|the|this|that|these|those|each|every)\s*$',
        ).hasMatch(prefix) =>
      'noun',
    (final prefix, _, _)
        when RegExp(
          r'\b(?:be|am|is|are|was|were|been|being|seem|seems|very|more|most)\s*$',
        ).hasMatch(prefix) =>
      'adjective',
    (_, _, final word) when word.endsWith('ly') => 'adverb',
    (_, final suffix, _) when RegExp(r'^[a-z]+').hasMatch(suffix) =>
      'adjective',
    _ => '',
  };
  if (desired.isEmpty) return '';
  for (final meaning in meanings) {
    for (final part in meaning.split(RegExp(r'[\n;\uFF1B]'))) {
      final candidate = part.trim();
      if (candidate.isEmpty) continue;
      final lower = candidate.toLowerCase();
      final matches = switch (desired) {
        'verb' => RegExp(r'^(?:v|vt|vi)\.').hasMatch(lower),
        'noun' => lower.startsWith('n.'),
        'adjective' => RegExp(r'^(?:a|adj)\.').hasMatch(lower),
        'adverb' => RegExp(r'^(?:ad|adv)\.').hasMatch(lower),
        _ => false,
      };
      if (!matches) continue;
      return candidate.replaceFirst(RegExp(r'^[a-z]+\.\s*'), '').trim();
    }
  }
  return '';
}

String _pickMeaningFromTranslation(List<String> meanings, String translation) {
  final translated = translation.replaceAll(RegExp(r'\s+'), '');
  if (translated.isEmpty) return '';
  final candidates = meanings
      .expand((meaning) => meaning.split(RegExp(r'[；;，,、]')))
      .map(
        (part) => part
            .replaceFirst(RegExp(r'^\s*(?:\[[^]]+\]|[a-z]+\.)\s*'), '')
            .replaceAll(RegExp(r'[（）()\[\]]'), '')
            .trim(),
      )
      .where((part) => part.runes.length >= 2)
      .toList(growable: false);
  String best = '';
  var bestScore = 0;
  for (final candidate in candidates) {
    final compact = candidate.replaceAll(RegExp(r'\s+'), '');
    if (translated.contains(compact)) return candidate;
    var score = 0;
    final chars = compact.runes.toList(growable: false);
    for (var index = 0; index + 1 < chars.length; index++) {
      final bigram = String.fromCharCodes(chars.sublist(index, index + 2));
      if (translated.contains(bigram)) score += 2;
    }
    if (score > bestScore) {
      best = candidate;
      bestScore = score;
    }
  }
  return bestScore > 0 ? best : '';
}

String formatExamPassage(String source) {
  final sentenceCounter = RegExp(r'''([.!?]\s+)\d{1,2}\s+(?=[A-Z"])''');
  return source
      .split(RegExp(r'\r?\n'))
      .map((rawParagraph) {
        var paragraph = rawParagraph.trim();
        paragraph = paragraph.replaceAll(RegExp(r'\.\?'), '.');
        paragraph = paragraph.replaceFirst(RegExp(r'^\d{1,2}\s+'), '');
        paragraph = paragraph.replaceAllMapped(
          sentenceCounter,
          (match) => match.group(1)!,
        );
        return paragraph.isEmpty ? '' : '\u2003\u2003$paragraph';
      })
      .where((paragraph) => paragraph.isNotEmpty)
      .join('\n\n');
}

bool isExamClozeSection(ExamSection section) {
  final type = section.type.toLowerCase();
  return type.contains('cloze') ||
      section.title.contains('完型') ||
      section.title.contains('完形');
}

bool isExamReadingSection(ExamSection section) {
  final type = section.type.toLowerCase();
  return type.contains('reading') || section.title.contains('阅读');
}

String formatClozePassage(
  String source, {
  required Set<int> blankNumbers,
  Map<int, String> submittedAnswers = const {},
}) {
  final passage = formatExamPassage(source);
  return passage.replaceAllMapped(RegExp(r'(?<![\d(])(\d{1,2})(?![\d)])'), (
    match,
  ) {
    final number = int.parse(match.group(1)!);
    if (!blankNumbers.contains(number) ||
        _isOrdinaryClozeNumber(passage, match)) {
      return match.group(0)!;
    }
    final answer = submittedAnswers[number]?.trim() ?? '';
    return '(${answer.isEmpty ? number : answer})';
  });
}

bool _isOrdinaryClozeNumber(String passage, Match match) {
  final suffix = passage.substring(match.end);
  if (RegExp(r'^(?:st|nd|rd|th)\b', caseSensitive: false).hasMatch(suffix)) {
    return true;
  }
  final nextWord = RegExp(r'^\s+([A-Za-z%]+)').firstMatch(suffix)?.group(1);
  if (nextWord == null) return false;
  return const {
    'year',
    'years',
    'month',
    'months',
    'week',
    'weeks',
    'day',
    'days',
    'hour',
    'hours',
    'minute',
    'minutes',
    'second',
    'seconds',
    'percent',
    'percentage',
    'times',
    'people',
    'dollars',
    'miles',
    'kilometers',
    'metres',
    'meters',
  }.contains(nextWord.toLowerCase());
}

String cleanExamExplanation(String source) {
  var value = source
      .replaceAll(RegExp(r'<br\s*/?>', caseSensitive: false), '\n')
      .replaceAll(RegExp(r'</p\s*>', caseSensitive: false), '\n')
      .replaceAll(RegExp(r'<[^>]+>'), '')
      .replaceAll('\uFFFD', '')
      .replaceAll('&nbsp;', ' ')
      .replaceAll('&amp;', '&')
      .replaceAll('&quot;', '"')
      .replaceAll('&#39;', "'")
      .replaceAll('&lt;', '<')
      .replaceAll('&gt;', '>');
  value = value.replaceAllMapped(RegExp(r'&#(x?[0-9a-fA-F]+);'), (match) {
    final raw = match.group(1)!;
    final codePoint = raw.startsWith('x') || raw.startsWith('X')
        ? int.tryParse(raw.substring(1), radix: 16)
        : int.tryParse(raw);
    return codePoint == null ? '' : String.fromCharCode(codePoint);
  });
  return value
      .split(RegExp(r'\r?\n'))
      .map((line) => line.replaceAll(RegExp(r'\s+'), ' ').trim())
      .where((line) => line.isNotEmpty)
      .join('\n');
}

class ExamWrongAnswerReportDetail {
  const ExamWrongAnswerReportDetail({
    required this.questionNumber,
    required this.question,
    required this.selectedOption,
    required this.correctOption,
    required this.passageLocation,
    required this.passageExcerpt,
    required this.explanation,
  });

  final int questionNumber;
  final String question;
  final String selectedOption;
  final String correctOption;
  final String passageLocation;
  final String passageExcerpt;
  final String explanation;
}

const _examReportStopWords = {
  'about',
  'after',
  'also',
  'answer',
  'because',
  'before',
  'between',
  'choice',
  'correct',
  'does',
  'from',
  'have',
  'into',
  'most',
  'only',
  'question',
  'that',
  'their',
  'there',
  'these',
  'this',
  'those',
  'under',
  'what',
  'when',
  'which',
  'with',
  'would',
};

ExamWrongAnswerReportDetail? buildExamWrongAnswerReportDetail({
  required ExamSection section,
  required ExamQuestion question,
  required Map<String, String> selections,
}) {
  final selectedLabel = selections[question.id]?.trim();
  final correctLabel = question.answer?.trim();
  if (!question.capabilities.autoGradable ||
      selectedLabel == null ||
      selectedLabel.isEmpty ||
      correctLabel == null ||
      correctLabel.isEmpty ||
      selectedLabel == correctLabel) {
    return null;
  }

  final selectedOption = _examReportOption(question, selectedLabel);
  final correctOption = _examReportOption(question, correctLabel);
  final reference = _locateExamReportPassage(
    section.passage,
    '${question.stem} $correctOption',
  );
  final sourceExplanation = cleanExamExplanation(question.explanation);
  return ExamWrongAnswerReportDetail(
    questionNumber: question.number,
    question: question.stem.trim().isEmpty
        ? '第 ${question.number} 题'
        : question.stem.trim(),
    selectedOption: selectedOption,
    correctOption: correctOption,
    passageLocation: reference.location,
    passageExcerpt: reference.excerpt,
    explanation: sourceExplanation.isEmpty
        ? '本题应以原文定位为依据，比较各选项与原文的表述范围和逻辑关系后排除错项。'
        : sourceExplanation,
  );
}

String _examReportOption(ExamQuestion question, String label) {
  for (final choice in question.choices) {
    if (choice.label.trim() == label) {
      final text = choice.text.trim();
      return text.isEmpty ? label : '$label · $text';
    }
  }
  return label;
}

class _ExamReportPassageReference {
  const _ExamReportPassageReference({
    required this.location,
    required this.excerpt,
  });

  final String location;
  final String excerpt;
}

_ExamReportPassageReference _locateExamReportPassage(
  String passage,
  String evidence,
) {
  final paragraphs = passage
      .split(RegExp(r'\r?\n\s*\r?\n'))
      .map((paragraph) => paragraph.replaceAll(RegExp(r'\s+'), ' ').trim())
      .where((paragraph) => paragraph.isNotEmpty)
      .toList(growable: false);
  final keywords = RegExp(r"[A-Za-z][A-Za-z'-]*")
      .allMatches(evidence.toLowerCase())
      .map((match) => match.group(0)!)
      .where((word) => word.length >= 4 && !_examReportStopWords.contains(word))
      .toSet();

  var bestIndex = -1;
  var bestScore = 0;
  for (var index = 0; index < paragraphs.length; index += 1) {
    final lower = paragraphs[index].toLowerCase();
    final score = keywords.where((word) => lower.contains(word)).length;
    if (score > bestScore) {
      bestScore = score;
      bestIndex = index;
    }
  }

  if (bestIndex < 0) {
    return const _ExamReportPassageReference(
      location: '全文',
      excerpt: '题干与选项未提供可唯一定位的原文关键词，请结合全文主旨和段落关系判断。',
    );
  }
  return _ExamReportPassageReference(
    location: '第 ${bestIndex + 1} 段',
    excerpt: _truncateExamReportExcerpt(paragraphs[bestIndex]),
  );
}

String _truncateExamReportExcerpt(String value) {
  const maxLength = 220;
  if (value.length <= maxLength) return value;
  return '${value.substring(0, maxLength).trimRight()}...';
}

class ExamSubmissionResult {
  const ExamSubmissionResult({
    required this.gradableCount,
    required this.correctCount,
    required this.incorrectQuestionIds,
    required this.unansweredQuestionIds,
    required this.unsupportedCount,
  });

  final int gradableCount;
  final int correctCount;
  final List<String> incorrectQuestionIds;
  final List<String> unansweredQuestionIds;
  final int unsupportedCount;

  double get score =>
      gradableCount == 0 ? 0 : correctCount / gradableCount * 100;
}

class ExamCausalFinding {
  const ExamCausalFinding({
    required this.questionNumber,
    required this.word,
    required this.reasoning,
    required this.confidence,
  });

  final int questionNumber;
  final String word;
  final String reasoning;
  final double confidence;
}

class ExamClozeOptionReview {
  const ExamClozeOptionReview({
    required this.label,
    required this.meaning,
    required this.analysis,
  });

  final String label;
  final String meaning;
  final String analysis;
}

class ExamClozeQuestionReview {
  const ExamClozeQuestionReview({
    required this.questionNumber,
    this.stem = '',
    this.selectedAnswer = '',
    this.correctAnswer = '',
    this.evidenceLocation = '',
    required this.contextSentence,
    required this.annotatedContext,
    required this.options,
    required this.analysis,
    required this.knowledgeGap,
  });

  final int questionNumber;
  final String stem;
  final String selectedAnswer;
  final String correctAnswer;
  final String evidenceLocation;
  final String contextSentence;
  final String annotatedContext;
  final List<ExamClozeOptionReview> options;
  final String analysis;
  final String knowledgeGap;
}

class ExamClozeCorrectReview {
  const ExamClozeCorrectReview({
    required this.questionNumber,
    required this.distinction,
  });

  final int questionNumber;
  final String distinction;
}

class ExamClozeVocabularyPriority {
  const ExamClozeVocabularyPriority({
    required this.priority,
    required this.word,
    required this.meaning,
    required this.mark,
    required this.examFrequency,
    required this.strictExamFrequency,
    required this.examFamilyRoot,
    required this.examRank,
    required this.reason,
  });

  final int priority;
  final String word;
  final String meaning;
  final String mark;
  final int examFrequency;
  final int strictExamFrequency;
  final String examFamilyRoot;
  final int? examRank;
  final String reason;
}

class ExamSectionAiAnalysis {
  const ExamSectionAiAnalysis({
    required this.findings,
    this.reviewFormat = '',
    this.clozeQuestions = const [],
    this.correctMarkedQuestions = const [],
    this.vocabularyPriority = const [],
  });

  final List<ExamCausalFinding> findings;
  final String reviewFormat;
  final List<ExamClozeQuestionReview> clozeQuestions;
  final List<ExamClozeCorrectReview> correctMarkedQuestions;
  final List<ExamClozeVocabularyPriority> vocabularyPriority;

  bool get isClozeReview =>
      clozeQuestions.isNotEmpty ||
      correctMarkedQuestions.isNotEmpty ||
      vocabularyPriority.isNotEmpty;

  bool get isStructuredReview =>
      reviewFormat == 'cloze-review-v1' || reviewFormat == 'reading-review-v1';
}

enum ExamCausalAnalysisFailureKind {
  providerUnavailable,
  timeout,
  incomplete,
  insufficientEvidence,
}

class ExamCausalAnalysisException extends StateError {
  ExamCausalAnalysisException({required this.kind, required String message})
    : super(message);

  final ExamCausalAnalysisFailureKind kind;

  String get displayMessage => message.toString();
}

String formatExamCausalAnalysisError(Object error) {
  if (error is ExamCausalAnalysisException) return error.displayMessage;
  return '$error';
}

List<ExamCausalFinding> parseExamCausalFindings(
  Map<String, dynamic> analysis, {
  required int questionNumber,
}) {
  final providerStatus = '${analysis['providerStatus'] ?? ''}'.trim();
  final limitations = (analysis['limitations'] as List<dynamic>? ?? const [])
      .map((value) => '$value'.trim())
      .where((value) => value.isNotEmpty)
      .toList(growable: false);
  if (providerStatus.isNotEmpty && providerStatus != 'completed') {
    throw ExamCausalAnalysisException(
      kind: ExamCausalAnalysisFailureKind.providerUnavailable,
      message: limitations.isEmpty ? 'AI 服务暂时不可用' : limitations.join('；'),
    );
  }
  if (analysis['eligible'] == false) {
    final missing = (analysis['missingEvidence'] as List<dynamic>? ?? const [])
        .map((value) => '$value'.trim())
        .where((value) => value.isNotEmpty)
        .toList(growable: false);
    throw ExamCausalAnalysisException(
      kind: ExamCausalAnalysisFailureKind.insufficientEvidence,
      message: missing.isEmpty ? '当前作答证据不满足分析条件' : missing.join('；'),
    );
  }
  final findings = <ExamCausalFinding>[];
  for (final candidate
      in (analysis['candidates'] as List<dynamic>? ?? const [])) {
    if (candidate is! Map<String, dynamic>) continue;
    final word = '${candidate['word'] ?? ''}'.trim();
    if (word.isEmpty) continue;
    findings.add(
      ExamCausalFinding(
        questionNumber: questionNumber,
        word: word,
        reasoning: '${candidate['reasoning'] ?? ''}'.trim(),
        confidence: ((candidate['confidence'] as num?) ?? 0).toDouble(),
      ),
    );
  }
  return findings;
}

List<ExamCausalFinding> parseExamSectionCausalFindings(
  Map<String, dynamic> analysis, {
  required List<ExamQuestion> questions,
}) {
  if (questions.isEmpty) return const [];
  // Reuse the single-question validation so provider and evidence failures
  // remain user-visible instead of silently producing an empty report.
  if (analysis['providerStatus'] != null || analysis['eligible'] == false) {
    parseExamCausalFindings(analysis, questionNumber: questions.first.number);
  }
  final questionById = {
    for (final question in questions) question.id: question,
  };
  final findings = <ExamCausalFinding>[];
  for (final item in analysis['questions'] as List<dynamic>? ?? const []) {
    if (item is! Map<String, dynamic>) continue;
    final question = questionById['${item['questionId'] ?? ''}'];
    if (question == null) continue;
    findings.addAll(
      parseExamCausalFindings(<String, dynamic>{
        ...analysis,
        'candidates': item['candidates'] ?? const [],
      }, questionNumber: question.number),
    );
  }
  return findings;
}

ExamSectionAiAnalysis parseExamSectionAiAnalysis(
  Map<String, dynamic> analysis, {
  required List<ExamQuestion> questions,
  Set<String>? requiredWrongQuestionIds,
}) {
  final findings = parseExamSectionCausalFindings(
    analysis,
    questions: questions,
  );
  final reviewFormat = '${analysis['reviewFormat'] ?? ''}'.trim();
  if (reviewFormat != 'cloze-review-v1' &&
      reviewFormat != 'reading-review-v1') {
    return ExamSectionAiAnalysis(findings: findings);
  }
  final questionById = {
    for (final question in questions) question.id: question,
  };
  Map<String, dynamic> mapValue(dynamic value) =>
      value is Map<String, dynamic> ? value : const {};
  final wrongReviews = <ExamClozeQuestionReview>[];
  for (final raw in analysis['questions'] as List<dynamic>? ?? const []) {
    final item = mapValue(raw);
    final question = questionById['${item['questionId'] ?? ''}'];
    if (question == null) continue;
    final options = <ExamClozeOptionReview>[];
    for (final rawOption
        in item['optionAnalysis'] as List<dynamic>? ?? const []) {
      final option = mapValue(rawOption);
      options.add(
        ExamClozeOptionReview(
          label: '${option['label'] ?? ''}'.trim(),
          meaning: '${option['meaning'] ?? ''}'.trim(),
          analysis: '${option['analysis'] ?? ''}'.trim(),
        ),
      );
    }
    wrongReviews.add(
      ExamClozeQuestionReview(
        questionNumber: question.number,
        stem: '${item['stem'] ?? question.stem}'.trim(),
        selectedAnswer: '${item['selectedAnswer'] ?? ''}'.trim(),
        correctAnswer: '${item['correctAnswer'] ?? question.answer ?? ''}'
            .trim(),
        evidenceLocation: '${item['evidenceLocation'] ?? ''}'.trim(),
        contextSentence: '${item['contextSentence'] ?? ''}'.trim(),
        annotatedContext: '${item['annotatedContext'] ?? ''}'.trim(),
        options: options,
        analysis: '${item['analysis'] ?? ''}'.trim(),
        knowledgeGap: '${item['knowledgeGap'] ?? ''}'.trim(),
      ),
    );
  }
  final requiredIds = requiredWrongQuestionIds ?? const <String>{};
  if (requiredIds.isNotEmpty) {
    final isReadingReview = reviewFormat == 'reading-review-v1';
    final reviewedIds = (analysis['questions'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .where((item) {
          final id = '${item['questionId'] ?? ''}';
          final options = item['optionAnalysis'] as List<dynamic>? ?? const [];
          return requiredIds.contains(id) &&
              (!isReadingReview ||
                  ('${item['stem'] ?? ''}'.trim().isNotEmpty &&
                      '${item['selectedAnswer'] ?? ''}'.trim().isNotEmpty &&
                      '${item['correctAnswer'] ?? ''}'.trim().isNotEmpty &&
                      '${item['evidenceLocation'] ?? ''}'.trim().isNotEmpty)) &&
              '${item['contextSentence'] ?? ''}'.trim().isNotEmpty &&
              options.isNotEmpty &&
              '${item['analysis'] ?? ''}'.trim().isNotEmpty &&
              '${item['knowledgeGap'] ?? ''}'.trim().isNotEmpty;
        })
        .map((item) => '${item['questionId']}')
        .toSet();
    if (!reviewedIds.containsAll(requiredIds)) {
      throw ExamCausalAnalysisException(
        kind: ExamCausalAnalysisFailureKind.incomplete,
        message: 'AI 完型复盘未覆盖全部错题，请重试。',
      );
    }
  }
  final correctReviews = <ExamClozeCorrectReview>[];
  for (final raw
      in analysis['correctMarkedQuestions'] as List<dynamic>? ?? const []) {
    final item = mapValue(raw);
    final question = questionById['${item['questionId'] ?? ''}'];
    final distinction = '${item['distinction'] ?? ''}'.trim();
    if (question != null && distinction.isNotEmpty) {
      correctReviews.add(
        ExamClozeCorrectReview(
          questionNumber: question.number,
          distinction: distinction,
        ),
      );
    }
  }
  final priorities = <ExamClozeVocabularyPriority>[];
  for (final raw
      in analysis['vocabularyPriority'] as List<dynamic>? ?? const []) {
    final item = mapValue(raw);
    final word = '${item['word'] ?? ''}'.trim();
    if (word.isEmpty) continue;
    final strictExamFrequency = (item['examFrequency'] as num?)?.toInt() ?? 0;
    final examFrequency =
        (item['displayExamFrequency'] as num?)?.toInt() ??
        (item['examFamilyFrequency'] as num?)?.toInt() ??
        strictExamFrequency;
    priorities.add(
      ExamClozeVocabularyPriority(
        priority: (item['priority'] as num?)?.toInt() ?? priorities.length + 1,
        word: word,
        meaning: '${item['meaning'] ?? ''}'.trim(),
        mark: '${item['mark'] ?? ''}'.trim(),
        examFrequency: examFrequency,
        strictExamFrequency: strictExamFrequency,
        examFamilyRoot: '${item['examFamilyRoot'] ?? ''}'.trim(),
        examRank: (item['examRank'] as num?)?.toInt(),
        reason: '${item['priorityReason'] ?? ''}'.trim(),
      ),
    );
  }
  return ExamSectionAiAnalysis(
    findings: findings,
    reviewFormat: reviewFormat,
    clozeQuestions: wrongReviews,
    correctMarkedQuestions: correctReviews,
    vocabularyPriority: priorities,
  );
}

Future<List<ExamCausalFinding>> analyzeExamWrongQuestionsConcurrently(
  List<ExamQuestion> questions, {
  required Future<Map<String, dynamic>> Function(ExamQuestion question) analyze,
  Duration timeout = const Duration(seconds: 45),
  int maxConcurrency = 2,
}) async {
  if (questions.isEmpty) return const [];
  final perQuestionFindings = List<List<ExamCausalFinding>?>.filled(
    questions.length,
    null,
  );
  var nextIndex = 0;
  var successfulQuestions = 0;
  var providerUnavailableQuestions = 0;
  var timedOutQuestions = 0;
  Object? firstError;
  StackTrace? firstErrorStack;
  Future<void> worker() async {
    while (nextIndex < questions.length) {
      final index = nextIndex++;
      final question = questions[index];
      try {
        final analysis = await analyze(question).timeout(timeout);
        perQuestionFindings[index] = parseExamCausalFindings(
          analysis,
          questionNumber: question.number,
        );
        successfulQuestions += 1;
      } catch (error, stackTrace) {
        if (error is ExamCausalAnalysisException &&
            error.kind == ExamCausalAnalysisFailureKind.providerUnavailable) {
          providerUnavailableQuestions += 1;
        } else if (error is TimeoutException) {
          timedOutQuestions += 1;
        }
        firstError ??= error;
        firstErrorStack ??= stackTrace;
      }
    }
  }

  await Future.wait(
    List.generate(maxConcurrency.clamp(1, questions.length), (_) => worker()),
  );
  if (firstError != null) {
    if (successfulQuestions == 0 &&
        providerUnavailableQuestions == questions.length) {
      throw ExamCausalAnalysisException(
        kind: ExamCausalAnalysisFailureKind.providerUnavailable,
        message: 'AI 服务暂时不可用，未生成本大题 ${questions.length} 道错题的分析。请稍后在大题报告中重试。',
      );
    }
    if (successfulQuestions == 0 && timedOutQuestions == questions.length) {
      throw ExamCausalAnalysisException(
        kind: ExamCausalAnalysisFailureKind.timeout,
        message:
            'AI 服务响应超时，未生成本大题 ${questions.length} 道错题的分析。请稍后在大题报告中重试（TimeoutException）。',
      );
    }
    Error.throwWithStackTrace(
      ExamCausalAnalysisException(
        kind: ExamCausalAnalysisFailureKind.incomplete,
        message:
            'AI 分析仅完成 $successfulQuestions / ${questions.length} 道错题，结果未保存。请在大题报告中重试。原因：${formatExamCausalAnalysisError(firstError!)}',
      ),
      firstErrorStack!,
    );
  }
  return perQuestionFindings
      .expand((value) => value ?? const <ExamCausalFinding>[])
      .toList(growable: false);
}

ExamSubmissionResult gradeExamSubmission(
  List<ExamQuestion> questions,
  Map<String, String> selections,
) {
  var gradable = 0;
  var correct = 0;
  var unsupported = 0;
  final incorrect = <String>[];
  final unanswered = <String>[];
  for (final question in questions) {
    if (!question.capabilities.autoGradable) {
      unsupported++;
      continue;
    }
    gradable++;
    final selected = selections[question.id];
    if (selected == null) {
      unanswered.add(question.id);
    } else if (selected == question.answer) {
      correct++;
    } else {
      incorrect.add(question.id);
    }
  }
  return ExamSubmissionResult(
    gradableCount: gradable,
    correctCount: correct,
    incorrectQuestionIds: incorrect,
    unansweredQuestionIds: unanswered,
    unsupportedCount: unsupported,
  );
}

class ExamReadingPositionMemory {
  ExamReadingPositionMemory({
    this.active = ExamReadingPane.passage,
    this.passageOffset = 0,
    this.questionOffset = 0,
  });

  ExamReadingPane active;
  double passageOffset;
  double questionOffset;

  double switchFrom(double currentOffset) {
    if (active == ExamReadingPane.passage) {
      passageOffset = currentOffset;
      active = ExamReadingPane.questions;
      return questionOffset;
    }
    questionOffset = currentOffset;
    active = ExamReadingPane.passage;
    return passageOffset;
  }
}

class ExamReadingPositionBubble extends StatelessWidget {
  const ExamReadingPositionBubble({
    super.key,
    required this.active,
    required this.bubbleAlignment,
    required this.onSwitch,
    required this.onBubbleAlignmentChanged,
    required this.child,
  });

  final ExamReadingPane active;
  final double bubbleAlignment;
  final VoidCallback onSwitch;
  final ValueChanged<double> onBubbleAlignmentChanged;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        const bubbleSize = 44.0;
        final travel = (constraints.maxHeight - bubbleSize).clamp(
          0.0,
          double.infinity,
        );
        final bubbleTop = (bubbleAlignment.clamp(-1.0, 1.0) + 1) / 2 * travel;
        return Stack(
          clipBehavior: Clip.hardEdge,
          children: [
            Positioned.fill(child: child),
            Positioned(
              right: -20,
              top: bubbleTop,
              child: GestureDetector(
                key: const ValueKey('exam-pane-focus-bubble'),
                onTap: onSwitch,
                onVerticalDragUpdate: (details) {
                  if (travel <= 0) return;
                  onBubbleAlignmentChanged(
                    (bubbleAlignment + details.delta.dy / travel * 2).clamp(
                      -1.0,
                      1.0,
                    ),
                  );
                },
                child: Opacity(
                  opacity: 0.62,
                  child: Semantics(
                    button: true,
                    label: active == ExamReadingPane.passage
                        ? '跳到题目位置'
                        : '跳到原文位置',
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.primary,
                      shape: const CircleBorder(),
                      child: SizedBox.square(
                        dimension: bubbleSize,
                        child: Align(
                          alignment: Alignment.centerLeft,
                          child: Padding(
                            padding: const EdgeInsets.only(left: 4),
                            child: Text(
                              active == ExamReadingPane.passage ? '↓题' : '↑文',
                              style: TextStyle(
                                color: Theme.of(context).colorScheme.onPrimary,
                                fontSize: 12,
                                fontWeight: FontWeight.w700,
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
  }
}

class ExamContinuousReader extends StatelessWidget {
  const ExamContinuousReader({
    super.key,
    required this.controller,
    required this.padding,
    required this.children,
  });

  final ScrollController controller;
  final EdgeInsetsGeometry padding;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      controller: controller,
      padding: padding,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      ),
    );
  }
}

class TodayLearningModeSelector extends StatelessWidget {
  const TodayLearningModeSelector({
    super.key,
    required this.selected,
    required this.onChanged,
  });

  final TodayLearningContent selected;
  final ValueChanged<TodayLearningContent> onChanged;

  @override
  Widget build(BuildContext context) {
    return SegmentedButton<TodayLearningContent>(
      segments: const [
        ButtonSegment(
          value: TodayLearningContent.words,
          icon: Icon(Icons.spellcheck_rounded, size: 15),
          label: Text('\u5355\u8bcd\u5b66\u4e60'),
        ),
        ButtonSegment(
          value: TodayLearningContent.practice,
          icon: Icon(Icons.assignment_outlined, size: 15),
          label: Text('\u6a21\u62df\u7ec3\u4e60'),
        ),
      ],
      selected: {selected},
      onSelectionChanged: (values) => onChanged(values.first),
      showSelectedIcon: false,
      style: ButtonStyle(
        visualDensity: VisualDensity.compact,
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
        textStyle: WidgetStateProperty.all(
          const TextStyle(fontSize: 12, fontWeight: FontWeight.w600),
        ),
        padding: WidgetStateProperty.all(
          const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        ),
      ),
    );
  }
}

class ExamPracticeHome extends StatefulWidget {
  const ExamPracticeHome({super.key, required this.sdk});

  final WordSdk sdk;

  @override
  State<ExamPracticeHome> createState() => _ExamPracticeHomeState();
}

class _ExamPracticeHomeState extends State<ExamPracticeHome> {
  final Map<String, ExamPaper> _paperCache = {};
  ExamCatalog? _catalog;
  ExamPaper? _paper;
  Object? _error;
  bool _loadingCatalog = true;
  bool _loadingPaper = false;
  String? _exam;
  int? _year;
  String? _paperId;
  String? _sectionId;

  @override
  void initState() {
    super.initState();
    _loadCatalog();
  }

  Future<void> _loadCatalog() async {
    setState(() {
      _loadingCatalog = true;
      _error = null;
    });
    try {
      final catalog = await widget.sdk.examPractice.getCatalog();
      if (!mounted) return;
      setState(() {
        _catalog = catalog;
        _exam = catalog.exams.isEmpty ? null : catalog.exams.first.exam;
        _chooseDefaults();
      });
    } catch (error) {
      if (mounted) setState(() => _error = error);
    } finally {
      if (mounted) setState(() => _loadingCatalog = false);
    }
  }

  ExamCatalogExam? get _selectedExam {
    for (final exam in _catalog?.exams ?? const <ExamCatalogExam>[]) {
      if (exam.exam == _exam) return exam;
    }
    return null;
  }

  List<int> get _years {
    final years =
        (_selectedExam?.papers ?? const <ExamPaperSummary>[])
            .map((paper) => paper.year)
            .toSet()
            .toList()
          ..sort((left, right) => right.compareTo(left));
    return years;
  }

  List<ExamPaperSummary> get _papers =>
      (_selectedExam?.papers ?? const <ExamPaperSummary>[])
          .where((paper) => paper.year == _year)
          .toList(growable: false);

  ExamPaperSummary? get _paperSummary {
    for (final paper in _papers) {
      if (paper.id == _paperId) return paper;
    }
    return null;
  }

  ExamSection? get _section {
    for (final section in _paper?.sections ?? const <ExamSection>[]) {
      if (section.id == _sectionId) return section;
    }
    return null;
  }

  void _chooseDefaults() {
    _year = _years.isEmpty ? null : _years.first;
    _paperId = _papers.isEmpty ? null : _papers.first.id;
    _paper = null;
    _sectionId = null;
    if (_paperId != null) _loadPaper();
  }

  Future<void> _loadPaper() async {
    final exam = _exam;
    final paperId = _paperId;
    if (exam == null || paperId == null) return;
    final cacheKey = '$exam:$paperId';
    final cached = _paperCache[cacheKey];
    if (cached != null) {
      final sections = cached.sections
          .where((section) => section.questions.isNotEmpty)
          .toList(growable: false);
      setState(() {
        _paper = cached;
        _sectionId = sections.isEmpty ? null : sections.first.id;
        _loadingPaper = false;
        _error = null;
      });
      return;
    }
    setState(() {
      _loadingPaper = true;
      _paper = null;
      _sectionId = null;
      _error = null;
    });
    try {
      final paper = await widget.sdk.examPractice.getPaper(
        exam: exam,
        paperId: paperId,
      );
      if (!mounted || _paperId != paperId) return;
      final sections = paper.sections
          .where((section) => section.questions.isNotEmpty)
          .toList(growable: false);
      setState(() {
        _paperCache[cacheKey] = paper;
        _paper = paper;
        _sectionId = sections.isEmpty ? null : sections.first.id;
      });
    } catch (error) {
      if (mounted) setState(() => _error = error);
    } finally {
      if (mounted) setState(() => _loadingPaper = false);
    }
  }

  void _changeExam(String? value) {
    setState(() {
      _exam = value;
      _chooseDefaults();
    });
  }

  void _changeYear(int? value) {
    setState(() {
      _year = value;
      _paperId = _papers.isEmpty ? null : _papers.first.id;
      _paper = null;
      _sectionId = null;
    });
    _loadPaper();
  }

  void _changePaper(String? value) {
    setState(() => _paperId = value);
    _loadPaper();
  }

  @override
  Widget build(BuildContext context) {
    if (_loadingCatalog) {
      return const CrocodileLoadingAnimation(label: '\u52a0\u8f7d\u4e2d...');
    }
    if (_catalog == null) {
      return _ExamLoadMessage(error: _error, onRetry: _loadCatalog);
    }
    return RefreshIndicator(
      onRefresh: _loadCatalog,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        children: [
          Text(
            '\u9009\u62e9\u7ec3\u4e60\u5185\u5bb9',
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 4),
          Text(
            '\u6309\u8003\u8bd5\u3001\u5e74\u4efd\u548c\u9898\u76ee\u5b9a\u4f4d\u5230\u672c\u6b21\u7ec3\u4e60\u3002',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 16),
          _SelectorField<String>(
            label: '\u8003\u8bd5\u7c7b\u578b',
            value: _exam,
            items: [
              for (final exam in _catalog!.exams)
                DropdownMenuItem(
                  value: exam.exam,
                  child: Text(_examLabel(exam.exam)),
                ),
            ],
            onChanged: _changeExam,
          ),
          const SizedBox(height: 12),
          _SelectorField<int>(
            label: '\u5e74\u4efd',
            value: _year,
            items: [
              for (final year in _years)
                DropdownMenuItem(value: year, child: Text('$year')),
            ],
            onChanged: _changeYear,
          ),
          const SizedBox(height: 12),
          _SelectorField<String>(
            label: '\u8bd5\u5377',
            value: _paperId,
            items: [
              for (final paper in _papers)
                DropdownMenuItem(
                  value: paper.id,
                  child: Text(paper.title, overflow: TextOverflow.ellipsis),
                ),
            ],
            onChanged: _changePaper,
          ),
          const SizedBox(height: 12),
          if (_loadingPaper) const LinearProgressIndicator(minHeight: 2),
          if (_error != null && !_loadingPaper)
            _ExamLoadMessage(error: _error, onRetry: _loadPaper),
          if (_paper != null) ...[
            _SelectorField<String>(
              label: '\u9898\u578b / \u7bc7\u7ae0',
              value: _sectionId,
              items: [
                for (final section in _paper!.sections.where(
                  (section) => section.questions.isNotEmpty,
                ))
                  DropdownMenuItem(
                    value: section.id,
                    child: Text(section.title, overflow: TextOverflow.ellipsis),
                  ),
              ],
              onChanged: (value) {
                setState(() {
                  _sectionId = value;
                });
              },
            ),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _section?.questions.isEmpty != false
                  ? null
                  : () => Navigator.of(context).push(
                      MaterialPageRoute(
                        builder: (_) => ExamPracticeScreen(
                          client: widget.sdk.examPractice,
                          paper: _paper!,
                          initialSectionId: _sectionId!,
                        ),
                      ),
                    ),
              icon: const Icon(Icons.play_arrow_rounded),
              label: const Text('\u5f00\u59cb\u7ec3\u4e60'),
            ),
            if (_paperSummary != null) ...[
              const SizedBox(height: 10),
              Text(
                '${_paperSummary!.autoGradableCount} / ${_paperSummary!.questionCount} '
                '\u9898\u53ef\u81ea\u52a8\u5224\u5206',
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ],
        ],
      ),
    );
  }
}

class ExamPracticeScreen extends StatefulWidget {
  const ExamPracticeScreen({
    super.key,
    required this.client,
    required this.paper,
    required this.initialSectionId,
  });

  final ExamPracticeClient client;
  final ExamPaper paper;
  final String initialSectionId;

  @override
  State<ExamPracticeScreen> createState() => _ExamPracticeScreenState();
}

class _ExamPracticeScreenState extends State<ExamPracticeScreen> {
  late int _sectionIndex;
  final ScrollController _readerController = ScrollController();
  final GlobalKey _questionStartKey = GlobalKey();
  final ExamReadingPositionMemory _positionMemory = ExamReadingPositionMemory();
  final Map<String, String> _selections = {};
  final Map<String, List<String>> _answerHistories = {};
  Map<String, String> _currentMeanings = {};
  Map<String, String> _currentMarks = {};
  Set<String> _priorWords = {};
  Map<String, String> _priorMarks = {};
  List<ExamTextAnnotation> _annotations = const [];
  ExamPracticeMode _mode = ExamPracticeMode.doing;
  ExamSubmissionResult? _submission;
  double _bubbleAlignment = 0;
  bool _saving = false;
  bool _restoringInitialState = true;
  bool _questionBookmarkSeeded = false;
  bool _switchingReadingPosition = false;
  double _questionBaselineOffset = 0;
  Set<String> _causalWords = {};
  bool _showParagraphTranslations = false;

  ExamSection get _section => widget.paper.sections[_sectionIndex];
  bool get _isClozeSection => isExamClozeSection(_section);
  bool get _isReadingSection => isExamReadingSection(_section);
  bool get _isTranslationSection =>
      _section.questions.any((question) => question.kind == 'translation') ||
      _section.title.contains('翻译');
  bool get _isWritingSection =>
      _section.questions.any((question) => question.kind == 'writing') ||
      _section.title.contains('写作');
  bool get _isSubjectiveSection => _isTranslationSection || _isWritingSection;
  String _attemptIdFor(ExamQuestion question) =>
      'exam:${widget.paper.id}:${question.id}';

  @override
  void initState() {
    super.initState();
    _sectionIndex = widget.paper.sections.indexWhere(
      (section) => section.id == widget.initialSectionId,
    );
    if (_sectionIndex < 0) _sectionIndex = 0;
    _readerController.addListener(_trackReadingPosition);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _seedQuestionBookmark();
    });
    _restoreInitialState();
  }

  Future<void> _restoreInitialState() async {
    await Future.wait([_restoreAttempts(), _hydrateAnnotations()]);
    if (mounted) setState(() => _restoringInitialState = false);
  }

  @override
  void dispose() {
    _readerController.removeListener(_trackReadingPosition);
    _readerController.dispose();
    super.dispose();
  }

  Future<void> _restoreAttempts() async {
    try {
      final attempts = await Future.wait(
        _section.questions.map(
          (question) => widget.client.getAttempt(_attemptIdFor(question)),
        ),
      );
      if (!mounted) return;
      setState(() {
        for (var index = 0; index < attempts.length; index++) {
          final attempt = attempts[index];
          if (attempt == null) continue;
          final questionId = _section.questions[index].id;
          if (attempt.selectedAnswer != null) {
            _selections[questionId] = attempt.selectedAnswer!;
          }
          _answerHistories[questionId] = attempt.answerHistory;
        }
        final submitted = attempts.whereType<ExamAttempt>().any(
          (attempt) =>
              attempt.status == 'answered' || attempt.isCorrect != null,
        );
        if (submitted) {
          _submission = gradeExamSubmission(_section.questions, _selections);
        }
      });
    } catch (_) {
      // A missing or stale attempt must not block offline paper reading.
    }
  }

  Future<void> _selectAnswer(ExamQuestion question, String answer) async {
    if (_submission != null || _restoringInitialState) return;
    final history = [...?_answerHistories[question.id]];
    if (history.isEmpty || history.last != answer) history.add(answer);
    setState(() {
      _selections[question.id] = answer;
      _answerHistories[question.id] = history;
      _saving = true;
    });
    try {
      await widget.client.saveAttempt(
        attemptId: _attemptIdFor(question),
        paperId: widget.paper.id,
        sectionId: _section.id,
        questionId: question.id,
        selectedAnswer: answer,
        isCorrect: null,
        answerHistory: history,
        status: 'in_progress',
      );
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _submit() async {
    if (_restoringInitialState) return;
    final result = gradeExamSubmission(_section.questions, _selections);
    setState(() => _saving = true);
    var saved = false;
    try {
      await Future.wait(
        _section.questions.map((question) {
          final selected = _selections[question.id];
          final isCorrect =
              question.capabilities.autoGradable && selected != null
              ? selected == question.answer
              : null;
          return widget.client.saveAttempt(
            attemptId: _attemptIdFor(question),
            paperId: widget.paper.id,
            sectionId: _section.id,
            questionId: question.id,
            selectedAnswer: selected,
            isCorrect: isCorrect,
            answerHistory: _answerHistories[question.id] ?? const [],
            status: 'answered',
          );
        }),
      );
      if (mounted) {
        setState(() => _submission = result);
        saved = true;
      }
    } finally {
      if (mounted) setState(() => _saving = false);
    }
    if (saved && mounted) await _openReport(result);
  }

  Future<ExamSectionAiAnalysis> _analyzeSectionWrongAnswers() async {
    final result = _submission;
    if (result == null) return const ExamSectionAiAnalysis(findings: []);
    final wrongQuestions = _section.questions
        .where((question) => result.incorrectQuestionIds.contains(question.id))
        .toList(growable: false);
    final analyzedQuestions = _isClozeSection || _isReadingSection
        ? _section.questions
              .where((question) => _selections.containsKey(question.id))
              .toList(growable: false)
        : wrongQuestions;
    final createdAt = DateTime.now().toUtc().toIso8601String();
    final taskId = '${widget.paper.id}:${_section.id}:$createdAt';
    await widget.client.saveAnalysisTask(
      taskId: taskId,
      exam: widget.paper.exam,
      paperId: widget.paper.id,
      paperTitle: widget.paper.title,
      sectionId: _section.id,
      sectionTitle: _section.title,
      status: 'running',
      createdAt: createdAt,
    );
    await ExamAnalysisTaskNotifications.refresh(widget.client);
    late final ExamSectionAiAnalysis sectionAnalysis;
    late final Map<String, dynamic> rawAnalysis;
    try {
      rawAnalysis = await widget.client
          .analyzeSectionVocabulary(
            exam: widget.paper.exam,
            paperId: widget.paper.id,
            sectionId: _section.id,
            attempts: analyzedQuestions
                .map(
                  (question) => ExamAnalysisAttempt(
                    questionId: question.id,
                    attemptId: _attemptIdFor(question),
                  ),
                )
                .toList(growable: false),
            purePurpleWords: _priorWords.difference(_currentMarks.keys.toSet()),
          )
          .timeout(examSectionAnalysisTimeout);
      sectionAnalysis = parseExamSectionAiAnalysis(
        rawAnalysis,
        questions: analyzedQuestions,
        requiredWrongQuestionIds: _isClozeSection || _isReadingSection
            ? wrongQuestions.map((question) => question.id).toSet()
            : null,
      );
      await widget.client.saveAnalysisTask(
        taskId: taskId,
        exam: widget.paper.exam,
        paperId: widget.paper.id,
        paperTitle: widget.paper.title,
        sectionId: _section.id,
        sectionTitle: _section.title,
        status: 'completed',
        createdAt: createdAt,
        completedAt: DateTime.now().toUtc().toIso8601String(),
        findings: sectionAnalysis.findings
            .map(
              (finding) => ExamAnalysisFinding(
                questionNumber: finding.questionNumber,
                word: finding.word,
                reasoning: finding.reasoning,
                confidence: finding.confidence,
              ),
            )
            .toList(growable: false),
        review: rawAnalysis,
      );
    } catch (error) {
      await widget.client.saveAnalysisTask(
        taskId: taskId,
        exam: widget.paper.exam,
        paperId: widget.paper.id,
        paperTitle: widget.paper.title,
        sectionId: _section.id,
        sectionTitle: _section.title,
        status: 'failed',
        createdAt: createdAt,
        completedAt: DateTime.now().toUtc().toIso8601String(),
        error: formatExamCausalAnalysisError(error),
      );
      await ExamAnalysisTaskNotifications.refresh(widget.client);
      rethrow;
    }
    await ExamAnalysisTaskNotifications.refresh(widget.client);
    if (mounted) {
      setState(() {
        _causalWords = {
          for (final finding in sectionAnalysis.findings) ...[
            finding.word.toLowerCase(),
            ...finding.word
                .toLowerCase()
                .split(RegExp(r'\s+'))
                .where((word) => word.isNotEmpty),
          ],
        };
      });
    }
    return sectionAnalysis;
  }

  Future<void> _openReport([ExamSubmissionResult? result]) async {
    final report = result ?? _submission;
    if (report == null) return;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => ExamSectionReportScreen(
          section: _section,
          selections: Map<String, String>.from(_selections),
          result: report,
          onAnalyze: _analyzeSectionWrongAnswers,
          currentWords: _currentMeanings.keys.toSet(),
          repeatedWords: _currentMeanings.keys.toSet().intersection(
            _priorWords,
          ),
        ),
      ),
    );
  }

  String get _articleId => '${widget.paper.id}:${_section.id}';

  Map<int, String> get _submittedClozeAnswers {
    if (_mode != ExamPracticeMode.analysis || _submission == null) {
      return const {};
    }
    return {
      for (final question in _section.questions)
        if (question.answer case final answerLabel?)
          question.number:
              question.choices
                  .where((choice) => choice.label == answerLabel)
                  .map((choice) => choice.text)
                  .firstOrNull ??
              answerLabel,
    };
  }

  String get _displayPassage => _isClozeSection
      ? formatClozePassage(
          _section.passage,
          blankNumbers: _section.questions
              .map((question) => question.number)
              .toSet(),
          submittedAnswers: _submittedClozeAnswers,
        )
      : formatExamPassage(_section.passage);

  List<String> get _passageParagraphs => _displayPassage
      .split(RegExp(r'\n\s*\n'))
      .where((paragraph) => paragraph.trim().isNotEmpty)
      .toList(growable: false);

  int _passageParagraphOffset(int index) {
    var offset = 0;
    for (var current = 0; current < index; current++) {
      offset += _passageParagraphs[current].length + 2;
    }
    return offset;
  }

  String get _articleBody => [
    _displayPassage,
    for (final question in _section.questions) ...[
      question.stem,
      for (final choice in question.choices) choice.text,
    ],
  ].where((value) => value.trim().isNotEmpty).join('\n');

  int _scopeOffset(ExamQuestion question, String scope) {
    final questionIndex = _section.questions.indexOf(question);
    final questionBase = (questionIndex + 1) * 1000000;
    if (scope == 'stem') return questionBase;
    final label = scope.startsWith('choice:') ? scope.substring(7) : scope;
    final choiceIndex = question.choices.indexWhere(
      (choice) => choice.label == label,
    );
    return questionBase + 100000 + (choiceIndex < 0 ? 0 : choiceIndex * 10000);
  }

  Future<void> _hydrateAnnotations() async {
    try {
      final tokens = await widget.client.tokenizeText(_articleBody);
      var state = await widget.client.getAnnotationState(
        articleId: _articleId,
        words: tokens.map((token) => token.normalized).toSet(),
      );
      final phraseWords = state.annotations
          .map(
            (annotation) =>
                normalizeExamPhraseSelection(annotation.selectedText),
          )
          .where((text) => text.contains(RegExp(r'\s+')))
          .toSet();
      if (phraseWords.isNotEmpty) {
        state = await widget.client.getAnnotationState(
          articleId: _articleId,
          words: {...tokens.map((token) => token.normalized), ...phraseWords},
        );
      }
      if (!mounted) return;
      setState(() {
        _currentMeanings = normalizeExamMeaningKeys(state.currentMeanings);
        _currentMarks = Map<String, String>.from(state.currentMarks);
        _priorWords = state.priorWords;
        _priorMarks = Map<String, String>.from(state.priorMarks);
        _causalWords = state.causalWords;
        _annotations = state.annotations;
      });
    } catch (_) {
      // Annotation hydration is additive and must not block offline practice.
    }
  }

  void _onInspectionChanged(ExamWordInspection inspection) {
    setState(() {
      final next = Map<String, String>.from(_currentMeanings);
      final nextMarks = Map<String, String>.from(_currentMarks);
      final family = normalizeExamWordFamily(inspection.normalized);
      if (inspection.isUnknown) {
        next[family] = inspection.meanings.join('；');
        nextMarks[family] = inspection.userMark;
      } else {
        next.remove(family);
        nextMarks.remove(family);
      }
      _currentMeanings = next;
      _currentMarks = nextMarks;
    });
  }

  void _onWordMarkChanged(String normalized, String mark) {
    setState(() {
      final nextMarks = Map<String, String>.from(_currentMarks);
      if (mark == 'none') {
        nextMarks.remove(normalized);
      } else {
        nextMarks[normalized] = mark;
      }
      _currentMarks = nextMarks;
    });
  }

  void _toggleReadingContext() {
    if (_section.paragraphTranslations.isEmpty) return;
    setState(() {
      _showParagraphTranslations = !_showParagraphTranslations;
    });
  }

  void _seedQuestionBookmark() {
    if (!mounted || !_readerController.hasClients) return;
    final renderObject = _questionStartKey.currentContext?.findRenderObject();
    if (renderObject == null) return;
    final viewport = RenderAbstractViewport.maybeOf(renderObject);
    if (viewport == null) return;
    final offset = viewport
        .getOffsetToReveal(renderObject, 0.05)
        .offset
        .clamp(0.0, _readerController.position.maxScrollExtent)
        .toDouble();
    _questionBaselineOffset = offset;
    _positionMemory.questionOffset = offset;
    _questionBookmarkSeeded = true;
  }

  void _trackReadingPosition() {
    if (!_questionBookmarkSeeded ||
        _switchingReadingPosition ||
        !_readerController.hasClients) {
      return;
    }
    final offset = _readerController.offset;
    final active = offset + 8 >= _questionBaselineOffset
        ? ExamReadingPane.questions
        : ExamReadingPane.passage;
    if (active == ExamReadingPane.passage) {
      _positionMemory.passageOffset = offset;
    } else {
      _positionMemory.questionOffset = offset;
    }
    if (_positionMemory.active != active && mounted) {
      setState(() => _positionMemory.active = active);
    }
  }

  Future<void> _switchReadingPosition() async {
    if (!_readerController.hasClients) return;
    if (!_questionBookmarkSeeded) _seedQuestionBookmark();
    if (!_questionBookmarkSeeded) return;
    final target = _positionMemory.switchFrom(_readerController.offset);
    _switchingReadingPosition = true;
    if (mounted) setState(() {});
    try {
      await _readerController.animateTo(
        target.clamp(0, _readerController.position.maxScrollExtent),
        duration: const Duration(milliseconds: 260),
        curve: Curves.easeOutCubic,
      );
    } finally {
      _switchingReadingPosition = false;
    }
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.paper.title, overflow: TextOverflow.ellipsis),
        actions: [
          IconButton(
            tooltip: _mode == ExamPracticeMode.doing ? '切换到分析模式' : '切换到做题模式',
            onPressed: () => setState(() {
              _mode = _mode == ExamPracticeMode.doing
                  ? ExamPracticeMode.analysis
                  : ExamPracticeMode.doing;
            }),
            icon: Icon(
              _mode == ExamPracticeMode.doing
                  ? Icons.edit_note_rounded
                  : Icons.analytics_outlined,
            ),
          ),
          if (_submission != null)
            IconButton(
              tooltip: '大题报告',
              onPressed: _openReport,
              icon: const Icon(Icons.summarize_outlined),
            ),
          if (_saving)
            const Padding(
              padding: EdgeInsets.all(16),
              child: SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            ),
        ],
      ),
      body: AbsorbPointer(
        absorbing: _restoringInitialState,
        child: ExamReadingPositionBubble(
          active: _positionMemory.active,
          bubbleAlignment: _bubbleAlignment,
          onSwitch: _switchReadingPosition,
          onBubbleAlignmentChanged: (value) =>
              setState(() => _bubbleAlignment = value),
          child: ExamContinuousReader(
            controller: _readerController,
            padding: const EdgeInsets.fromLTRB(16, 16, 16, 96),
            children: [
              if (_restoringInitialState) ...[
                const LinearProgressIndicator(),
                const SizedBox(height: 8),
                const Text('正在恢复作答记录…'),
                const SizedBox(height: 12),
              ],
              SegmentedButton<ExamPracticeMode>(
                segments: const [
                  ButtonSegment(
                    value: ExamPracticeMode.doing,
                    icon: Icon(Icons.edit_note_rounded),
                    label: Text('做题'),
                  ),
                  ButtonSegment(
                    value: ExamPracticeMode.analysis,
                    icon: Icon(Icons.analytics_outlined),
                    label: Text('分析'),
                  ),
                ],
                selected: {_mode},
                onSelectionChanged: (value) =>
                    setState(() => _mode = value.first),
                showSelectedIcon: false,
              ),
              if (_mode == ExamPracticeMode.analysis) ...[
                const SizedBox(height: 8),
                Align(
                  alignment: Alignment.centerRight,
                  child: FilledButton.tonalIcon(
                    onPressed: _section.paragraphTranslations.isEmpty
                        ? null
                        : _toggleReadingContext,
                    icon: Icon(
                      _showParagraphTranslations
                          ? Icons.translate_rounded
                          : Icons.g_translate_rounded,
                    ),
                    label: Text(
                      _section.paragraphTranslations.isEmpty
                          ? '暂无内置译文'
                          : _showParagraphTranslations
                          ? '隐藏段落译文'
                          : '显示段落译文',
                    ),
                  ),
                ),
              ],
              const SizedBox(height: 12),
              Text(
                _section.title,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              if (_section.instructions.isNotEmpty) ...[
                const SizedBox(height: 8),
                Text(_section.instructions),
              ],
              if (_section.passage.isNotEmpty && !_isWritingSection) ...[
                const SizedBox(height: 16),
                for (
                  var index = 0;
                  index < _passageParagraphs.length;
                  index++
                ) ...[
                  ExamInteractiveText(
                    client: widget.client,
                    text: _passageParagraphs[index],
                    articleId: _articleId,
                    title: '${widget.paper.title} / ${_section.title}',
                    articleBody: _articleBody,
                    offsetBase: _passageParagraphOffset(index),
                    metadata: {
                      'paperId': widget.paper.id,
                      'sectionId': _section.id,
                      'scope': 'passage',
                      'paragraphIndex': index,
                    },
                    mode: _mode,
                    currentMeanings: _currentMeanings,
                    currentMarks: _currentMarks,
                    priorWords: _priorWords,
                    priorMarks: _priorMarks,
                    contextTranslation:
                        index < _section.paragraphTranslations.length
                        ? _section.paragraphTranslations[index]
                        : '',
                    annotations: _annotations,
                    compactClozeMarkers: _isClozeSection,
                    emphasisWords: _causalWords,
                    showInlineMeanings: _mode == ExamPracticeMode.analysis,
                    onInspectionChanged: _onInspectionChanged,
                    onWordMarkChanged: _onWordMarkChanged,
                    onAnnotationsChanged: (value) =>
                        setState(() => _annotations = value),
                    style: const TextStyle(fontSize: 16, height: 1.65),
                  ),
                  if (_mode == ExamPracticeMode.analysis &&
                      _showParagraphTranslations &&
                      index < _section.paragraphTranslations.length)
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 6, 4, 14),
                      child: Text(
                        _section.paragraphTranslations[index],
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                          height: 1.55,
                        ),
                      ),
                    )
                  else
                    const SizedBox(height: 12),
                ],
              ],
              const SizedBox(height: 16),
              Container(key: _questionStartKey),
              if (_isSubjectiveSection)
                ExamSubjectiveReferencePanel(
                  kind: _isTranslationSection ? 'translation' : 'writing',
                  answers: _isTranslationSection
                      ? _section.questions
                            .map((question) => question.answer?.trim() ?? '')
                            .where((answer) => answer.isNotEmpty)
                            .toList(growable: false)
                      : [
                          if (_section.passage.trim().isNotEmpty)
                            _section.passage.trim(),
                          ..._section.questions
                              .map((question) => question.answer?.trim() ?? '')
                              .where((answer) => answer.isNotEmpty),
                        ],
                )
              else
                for (final question in _section.questions) ...[
                  ExamQuestionCard(
                    question: question,
                    compact: _isClozeSection,
                    selectedAnswer: _selections[question.id],
                    submitted: _submission != null,
                    onSelect: (answer) => _selectAnswer(question, answer),
                    interactiveTextBuilder: (text, scope) =>
                        ExamInteractiveText(
                          client: widget.client,
                          text: text,
                          articleId: _articleId,
                          title: '${widget.paper.title} / ${_section.title}',
                          articleBody: _articleBody,
                          offsetBase: _scopeOffset(question, scope),
                          metadata: {
                            'paperId': widget.paper.id,
                            'sectionId': _section.id,
                            'questionId': question.id,
                            'scope': scope,
                          },
                          mode: _mode,
                          currentMeanings: _currentMeanings,
                          currentMarks: _currentMarks,
                          priorWords: _priorWords,
                          priorMarks: _priorMarks,
                          annotations: _annotations,
                          emphasisWords: _causalWords,
                          showInlineMeanings:
                              _mode == ExamPracticeMode.analysis,
                          onInspectionChanged: _onInspectionChanged,
                          onWordMarkChanged: _onWordMarkChanged,
                          onAnnotationsChanged: (value) =>
                              setState(() => _annotations = value),
                        ),
                  ),
                  const SizedBox(height: 16),
                ],
            ],
          ),
        ),
      ),
      bottomNavigationBar: _isSubjectiveSection
          ? null
          : SafeArea(
              minimum: const EdgeInsets.fromLTRB(16, 8, 16, 12),
              child: FilledButton.icon(
                onPressed: _saving || _restoringInitialState
                    ? null
                    : _submission == null
                    ? _submit
                    : _openReport,
                icon: Icon(
                  _submission == null
                      ? Icons.check_circle_outline
                      : Icons.verified_rounded,
                ),
                label: Text(_submission == null ? '提交并统一评判' : '查看大题报告'),
              ),
            ),
    );
  }
}

class ExamSubjectiveReferencePanel extends StatefulWidget {
  const ExamSubjectiveReferencePanel({
    super.key,
    required this.kind,
    required this.answers,
  });

  final String kind;
  final List<String> answers;

  @override
  State<ExamSubjectiveReferencePanel> createState() =>
      _ExamSubjectiveReferencePanelState();
}

class _ExamSubjectiveReferencePanelState
    extends State<ExamSubjectiveReferencePanel> {
  final Set<int> _revealed = {};

  @override
  Widget build(BuildContext context) {
    final label = widget.kind == 'writing' ? '参考范文' : '参考译文';
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surfaceContainerLow,
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
          ),
          child: const Padding(
            padding: EdgeInsets.all(12),
            child: Row(
              children: [
                Icon(Icons.info_outline_rounded, size: 18),
                SizedBox(width: 8),
                Expanded(child: Text('主观题暂不评分，请完成后自行对照参考答案。')),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        if (widget.answers.isEmpty)
          const Text('当前试卷未内置参考答案。')
        else
          for (var index = 0; index < widget.answers.length; index++) ...[
            OutlinedButton.icon(
              onPressed: () => setState(() {
                if (!_revealed.add(index)) _revealed.remove(index);
              }),
              icon: Icon(
                _revealed.contains(index)
                    ? Icons.visibility_off_outlined
                    : Icons.visibility_outlined,
              ),
              label: Text(
                _revealed.contains(index)
                    ? '隐藏第 ${index + 1} 题答案'
                    : '显示第 ${index + 1} 题答案',
              ),
            ),
            if (_revealed.contains(index))
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 8, 12, 16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      '$label ${index + 1}',
                      style: Theme.of(context).textTheme.labelLarge,
                    ),
                    const SizedBox(height: 6),
                    Text(widget.answers[index]),
                  ],
                ),
              ),
            const SizedBox(height: 4),
          ],
      ],
    );
  }
}

class ExamQuestionCard extends StatelessWidget {
  const ExamQuestionCard({
    super.key,
    required this.question,
    required this.selectedAnswer,
    this.submitted = false,
    this.compact = false,
    required this.onSelect,
    this.interactiveTextBuilder,
  });

  final ExamQuestion question;
  final String? selectedAnswer;
  final bool submitted;
  final bool compact;
  final ValueChanged<String> onSelect;
  final Widget Function(String text, String scope)? interactiveTextBuilder;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: Theme.of(context).colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: EdgeInsets.all(compact ? 12 : 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  compact ? '(${question.number}) ' : '${question.number}. ',
                  style: compact
                      ? Theme.of(context).textTheme.bodySmall
                      : Theme.of(context).textTheme.titleMedium,
                ),
                Expanded(
                  child:
                      compact &&
                          RegExp(
                            r'^Question\s+\d+$',
                            caseSensitive: false,
                          ).hasMatch(question.stem.trim())
                      ? const SizedBox.shrink()
                      : interactiveTextBuilder?.call(question.stem, 'stem') ??
                            Text(
                              question.stem,
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                ),
              ],
            ),
            SizedBox(height: compact ? 4 : 12),
            if (question.choices.isEmpty)
              Text(
                question.hasAnswer
                    ? '\u672c\u9898\u6709\u53c2\u8003\u7b54\u6848\uff0c\u5f53\u524d\u7248\u672c\u6682\u4e0d\u652f\u6301\u81ea\u52a8\u5224\u5206\u3002'
                    : '\u672c\u9898\u6682\u65e0\u53ef\u7528\u7b54\u6848\uff0c\u4e0d\u4f1a\u8bb0\u5f55\u4e3a\u9519\u9898\u3002',
              )
            else
              RadioGroup<String>(
                groupValue: selectedAnswer,
                onChanged: (value) {
                  if (value != null) onSelect(value);
                },
                child: Column(
                  children: [
                    for (final choice in question.choices)
                      Padding(
                        padding: EdgeInsets.only(bottom: compact ? 2 : 6),
                        child: RadioListTile<String>(
                          value: choice.label,
                          title: Row(
                            children: [
                              SizedBox(
                                width: 26,
                                child: Text(
                                  choice.label,
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w700,
                                  ),
                                ),
                              ),
                              Expanded(
                                child:
                                    interactiveTextBuilder?.call(
                                      choice.text,
                                      'choice:${choice.label}',
                                    ) ??
                                    Text(choice.text),
                              ),
                            ],
                          ),
                          dense: compact,
                          visualDensity: compact
                              ? const VisualDensity(vertical: -4)
                              : const VisualDensity(vertical: -2),
                          contentPadding: EdgeInsets.zero,
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(6),
                            side: BorderSide(
                              color: Theme.of(
                                context,
                              ).colorScheme.outlineVariant,
                            ),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class ExamSectionReportScreen extends StatefulWidget {
  const ExamSectionReportScreen({
    super.key,
    required this.section,
    required this.selections,
    required this.result,
    required this.onAnalyze,
    this.currentWords = const {},
    this.repeatedWords = const {},
  });

  final ExamSection section;
  final Map<String, String> selections;
  final ExamSubmissionResult result;
  final Future<ExamSectionAiAnalysis> Function() onAnalyze;
  final Set<String> currentWords;
  final Set<String> repeatedWords;

  @override
  State<ExamSectionReportScreen> createState() =>
      _ExamSectionReportScreenState();
}

class _ExamSectionReportScreenState extends State<ExamSectionReportScreen> {
  String? _selectedQuestionId;
  bool _analyzing = false;
  ExamSectionAiAnalysis? _analysis;
  Object? _analysisError;

  ExamQuestion? get _selectedQuestion {
    for (final question in widget.section.questions) {
      if (question.id == _selectedQuestionId) return question;
    }
    return null;
  }

  Future<void> _analyze() async {
    setState(() {
      _analyzing = true;
      _analysisError = null;
    });
    try {
      final analysis = await widget.onAnalyze();
      if (mounted) setState(() => _analysis = analysis);
    } catch (error) {
      if (mounted) setState(() => _analysisError = error);
    } finally {
      if (mounted) setState(() => _analyzing = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final selected = _selectedQuestion;
    return Scaffold(
      appBar: AppBar(title: Text('${widget.section.title} · 大题报告')),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              Text(
                '${widget.result.correctCount} / ${widget.result.gradableCount}',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(width: 10),
              Padding(
                padding: const EdgeInsets.only(bottom: 3),
                child: Text('${widget.result.score.round()} 分'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Text(
            '本篇标记：${widget.currentWords.isEmpty ? '无' : (widget.currentWords.toList()..sort()).join('、')}',
          ),
          if (widget.repeatedWords.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text('跨篇重复：${(widget.repeatedWords.toList()..sort()).join('、')}'),
          ],
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final question in widget.section.questions)
                _QuestionStatusChip(
                  question: question,
                  selected: question.id == _selectedQuestionId,
                  incorrect: widget.result.incorrectQuestionIds.contains(
                    question.id,
                  ),
                  unanswered: widget.result.unansweredQuestionIds.contains(
                    question.id,
                  ),
                  onTap: () => setState(() {
                    _selectedQuestionId = question.id;
                  }),
                ),
            ],
          ),
          const SizedBox(height: 16),
          FilledButton.tonalIcon(
            onPressed: _analyzing || widget.result.incorrectQuestionIds.isEmpty
                ? null
                : _analyze,
            icon: _analyzing
                ? const SizedBox.square(
                    dimension: 17,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.psychology_outlined),
            label: Text(
              isExamClozeSection(widget.section) ? '生成 AI 完型复盘' : 'AI 分析全部错题',
            ),
          ),
          if (_analyzing) ...[
            const SizedBox(height: 6),
            const Text('正在后台生成，离开本页后可在 AI 页查看进度与结果。'),
          ],
          if (_analysisError != null) ...[
            const SizedBox(height: 8),
            Text(
              '分析失败：$_analysisError',
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          if (_analysis != null && _analysis!.isStructuredReview) ...[
            const SizedBox(height: 18),
            _ExamStructuredReviewView(analysis: _analysis!),
          ] else if (_analysis != null) ...[
            const SizedBox(height: 12),
            const Text('已检查本大题全部错题；下方只列真正影响选项判断的高亮词。'),
            const SizedBox(height: 6),
            if (_analysis!.findings.isEmpty)
              const Text('现有标记与作答证据不足以确定致错词。')
            else
              for (final finding in _analysis!.findings)
                ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: CircleAvatar(
                    radius: 16,
                    child: Text('${finding.questionNumber}'),
                  ),
                  title: Text(
                    finding.word,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  subtitle: Text(finding.reasoning),
                  trailing: Text('${(finding.confidence * 100).round()}%'),
                ),
          ],
          if (selected != null &&
              buildExamWrongAnswerReportDetail(
                    section: widget.section,
                    question: selected,
                    selections: widget.selections,
                  ) ==
                  null) ...[
            const Divider(height: 32),
            Text(
              '第 ${selected.number} 题',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            Text('你的答案：${widget.selections[selected.id] ?? '未作答'}'),
            Text('正确答案：${selected.answer ?? '暂无'}'),
            if (selected.explanation.trim().isNotEmpty) ...[
              const SizedBox(height: 10),
              Text(cleanExamExplanation(selected.explanation)),
            ],
          ],
          if (selected != null)
            _ExamWrongAnswerReportDetailView(
              detail: buildExamWrongAnswerReportDetail(
                section: widget.section,
                question: selected,
                selections: widget.selections,
              )!,
            ),
        ],
      ),
    );
  }
}

class _ExamStructuredReviewView extends StatelessWidget {
  const _ExamStructuredReviewView({required this.analysis});

  final ExamSectionAiAnalysis analysis;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('错题逐题复盘', style: theme.textTheme.titleLarge),
        const SizedBox(height: 10),
        for (final question in analysis.clozeQuestions) ...[
          Text(
            '第 ${question.questionNumber} 题',
            style: theme.textTheme.titleMedium,
          ),
          if (analysis.reviewFormat == 'reading-review-v1') ...[
            if (question.stem.isNotEmpty) ...[
              const SizedBox(height: 6),
              Text('题目：${question.stem}'),
            ],
            Text(
              '你的答案：${question.selectedAnswer.isEmpty ? '未作答' : question.selectedAnswer}  '
              '正确答案：${question.correctAnswer.isEmpty ? '暂无' : question.correctAnswer}',
            ),
            if (question.evidenceLocation.isNotEmpty)
              Text('定位：${question.evidenceLocation}'),
          ],
          if (question.annotatedContext.isNotEmpty) ...[
            const SizedBox(height: 6),
            Text('所需原文：${question.annotatedContext}'),
          ] else if (question.contextSentence.isNotEmpty) ...[
            const SizedBox(height: 6),
            Text('所需原文：${question.contextSentence}'),
          ],
          if (question.options.isNotEmpty) ...[
            const SizedBox(height: 8),
            const Text('选项辨析'),
            for (final option in question.options)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Text(
                  '${option.label} ${option.meaning}${option.analysis.isEmpty ? '' : '：${option.analysis}'}',
                ),
              ),
          ],
          if (question.analysis.isNotEmpty) ...[
            const SizedBox(height: 8),
            Text('解析：${question.analysis}'),
          ],
          if (question.knowledgeGap.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text('真正缺口：${question.knowledgeGap}'),
          ],
          const Divider(height: 28),
        ],
        if (analysis.correctMarkedQuestions.isNotEmpty) ...[
          Text('答对题中的标记选项', style: theme.textTheme.titleLarge),
          const SizedBox(height: 8),
          for (final question in analysis.correctMarkedQuestions)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text(
                '第 ${question.questionNumber} 题：${question.distinction}',
              ),
            ),
          const SizedBox(height: 12),
        ],
        Text('标记词重要程度', style: theme.textTheme.titleLarge),
        const SizedBox(height: 8),
        for (final item in analysis.vocabularyPriority)
          Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: Text(
              '${item.priority}. ${item.word}  ${item.meaning}\n'
              '${item.examFamilyRoot.isEmpty ? '真题出现 ${item.examFrequency} 次' : '同源词族真题出现 ${item.examFrequency} 次 · 本词条单独 ${item.strictExamFrequency} 次'}'
              '${item.examRank == null
                  ? ''
                  : item.examFamilyRoot.isEmpty
                  ? ' · 排名 ${item.examRank}'
                  : ' · 本词严格排名 ${item.examRank}'}'
              '${item.reason.isEmpty ? '' : '\n${item.reason}'}',
            ),
          ),
      ],
    );
  }
}

class _ExamWrongAnswerReportDetailView extends StatelessWidget {
  const _ExamWrongAnswerReportDetailView({required this.detail});

  final ExamWrongAnswerReportDetail detail;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '第 ${detail.questionNumber} 题错题解析',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        Text('题干\n${detail.question}'),
        const SizedBox(height: 12),
        Text('你的错选\n${detail.selectedOption}'),
        const SizedBox(height: 12),
        Text('正确答案\n${detail.correctOption}'),
        const SizedBox(height: 12),
        Text('原文定位\n${detail.passageLocation} · ${detail.passageExcerpt}'),
        const SizedBox(height: 12),
        Text('参考解析\n${detail.explanation}'),
      ],
    );
  }
}

class _QuestionStatusChip extends StatelessWidget {
  const _QuestionStatusChip({
    required this.question,
    required this.selected,
    required this.incorrect,
    required this.unanswered,
    required this.onTap,
  });

  final ExamQuestion question;
  final bool selected;
  final bool incorrect;
  final bool unanswered;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final unsupported = !question.capabilities.autoGradable;
    final color = unsupported || unanswered
        ? Theme.of(context).colorScheme.outline
        : incorrect
        ? Theme.of(context).colorScheme.error
        : Theme.of(context).colorScheme.primary;
    final icon = unsupported || unanswered
        ? Icons.remove_circle_outline
        : incorrect
        ? Icons.cancel_outlined
        : Icons.check_circle_outline;
    return FilterChip(
      key: ValueKey('report-question-${question.id}'),
      selected: selected,
      onSelected: (_) => onTap(),
      avatar: Icon(icon, size: 17, color: color),
      label: Text('${question.number}'),
      side: BorderSide(color: color.withValues(alpha: 0.55)),
    );
  }
}

class ExamInteractiveText extends StatefulWidget {
  const ExamInteractiveText({
    super.key,
    required this.client,
    required this.text,
    required this.articleId,
    required this.title,
    required this.metadata,
    this.articleBody,
    this.offsetBase = 0,
    this.style,
    this.mode = ExamPracticeMode.doing,
    this.currentMeanings = const {},
    this.currentMarks = const {},
    this.priorWords = const {},
    this.priorMarks = const {},
    this.contextTranslation = '',
    this.annotations = const [],
    this.compactClozeMarkers = false,
    this.emphasisWords = const {},
    this.showInlineMeanings = true,
    this.onInspectionChanged,
    this.onWordMarkChanged,
    this.onAnnotationsChanged,
  });

  final ExamPracticeClient client;
  final String text;
  final String articleId;
  final String title;
  final Map<String, dynamic> metadata;
  final String? articleBody;
  final int offsetBase;
  final TextStyle? style;
  final ExamPracticeMode mode;
  final Map<String, String> currentMeanings;
  final Map<String, String> currentMarks;
  final Set<String> priorWords;
  final Map<String, String> priorMarks;
  final String contextTranslation;
  final List<ExamTextAnnotation> annotations;
  final bool compactClozeMarkers;
  final Set<String> emphasisWords;
  final bool showInlineMeanings;
  final ValueChanged<ExamWordInspection>? onInspectionChanged;
  final void Function(String normalized, String mark)? onWordMarkChanged;
  final ValueChanged<List<ExamTextAnnotation>>? onAnnotationsChanged;

  @override
  State<ExamInteractiveText> createState() => _ExamInteractiveTextState();
}

class _ExamRenderedTapRange {
  const _ExamRenderedTapRange({
    required this.start,
    required this.end,
    required this.token,
  });

  final int start;
  final int end;
  final ExamWordToken token;
}

class _ExamPendingPointerTap {
  _ExamPendingPointerTap({
    required this.position,
    required this.globalPosition,
    required this.startedAt,
    required this.markSerial,
  });

  final Offset position;
  final Offset globalPosition;
  final DateTime startedAt;
  final int markSerial;
  Timer? longPressTimer;
  bool longPressTriggered = false;
}

class _ExamInteractiveTextState extends State<ExamInteractiveText> {
  List<ExamWordToken> _tokens = const [];
  final List<GestureRecognizer> _recognizers = [];
  final GlobalKey _selectableTextKey = GlobalKey();
  Timer? _selectionLookupTimer;
  Timer? _nativeToolbarDismissTimer;
  int _selectionLookupGeneration = 0;
  bool _selectionSheetOpen = false;
  OverlayEntry? _markPaletteEntry;
  Rect? _markPaletteRect;
  ExamWordToken? _markPaletteToken;
  String _markPaletteLevel = 'familiar';
  final Map<int, _ExamPendingPointerTap> _pendingPointerTaps = {};
  int _markRequestSerial = 0;

  @override
  void initState() {
    super.initState();
    _useInteractionTokens(widget.text);
    _loadTokens();
  }

  @override
  void didUpdateWidget(covariant ExamInteractiveText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text ||
        oldWidget.articleId != widget.articleId) {
      _useInteractionTokens(widget.text);
      _loadTokens();
    } else if (oldWidget.currentMarks != widget.currentMarks) {
      _rebuildRecognizers();
    }
  }

  @override
  void dispose() {
    _selectionLookupTimer?.cancel();
    _nativeToolbarDismissTimer?.cancel();
    for (final pending in _pendingPointerTaps.values) {
      pending.longPressTimer?.cancel();
    }
    _pendingPointerTaps.clear();
    _hideMarkPalette();
    _disposeRecognizers();
    super.dispose();
  }

  void _disposeRecognizers() {
    for (final recognizer in _recognizers) {
      recognizer.dispose();
    }
    _recognizers.clear();
  }

  void _useInteractionTokens(String text) {
    _tokens = tokenizeExamTextForInteraction(text);
    _rebuildRecognizers(notify: false);
  }

  Future<void> _loadTokens() async {
    final text = widget.text;
    try {
      final tokens = await widget.client.tokenizeText(text);
      if (!mounted || text != widget.text) return;
      final nativeCoverage = tokens.fold<int>(
        0,
        (total, token) => total + token.text.length,
      );
      final localCoverage = _tokens.fold<int>(
        0,
        (total, token) => total + token.text.length,
      );
      if (nativeCoverage < localCoverage) return;
      setState(() {
        _tokens = tokens;
        _rebuildRecognizers(notify: false);
      });
    } catch (_) {}
  }

  void _rebuildRecognizers({bool notify = true}) {
    _disposeRecognizers();
    for (final token in _tokens) {
      final family = normalizeExamWordFamily(token.normalized);
      _recognizers.add(
        TapGestureRecognizer()
          ..onTap = () =>
              _setWordMark(token, widget.currentMarks[family] ?? 'familiar'),
      );
    }
    if (notify && mounted) setState(() {});
  }

  Future<ExamWordInspection> _inspect(ExamWordToken token, {String? userMark}) {
    final persistedToken = ExamWordToken(
      text: token.text,
      normalized: token.normalized,
      startOffset: token.startOffset + widget.offsetBase,
      endOffset: token.endOffset + widget.offsetBase,
    );
    return widget.client.inspectWord(
      articleId: widget.articleId,
      title: widget.title,
      body: widget.articleBody ?? widget.text,
      token: persistedToken,
      sentenceText: widget.text,
      metadata: widget.metadata,
      userMark: userMark,
    );
  }

  void _showMarkPalette(ExamWordToken token, Offset globalPosition) {
    _hideMarkPalette();
    const width = 132.0;
    const height = 44.0;
    final screenWidth = MediaQuery.sizeOf(context).width;
    final left = (globalPosition.dx - width / 2).clamp(
      8.0,
      screenWidth - width - 8,
    );
    final top = (globalPosition.dy - height - 18).clamp(8.0, double.infinity);
    _markPaletteRect = Rect.fromLTWH(left, top, width, height);
    _markPaletteToken = token;
    _markPaletteLevel = 'familiar';
    _markPaletteEntry = OverlayEntry(builder: _buildMarkPalette);
    Overlay.of(context, rootOverlay: true).insert(_markPaletteEntry!);
  }

  void _commitMarkPalette() {
    final token = _markPaletteToken;
    final level = _markPaletteLevel;
    _hideMarkPalette();
    if (token != null) _setWordMark(token, level);
  }

  void _hideMarkPalette() {
    _markPaletteEntry?.remove();
    _markPaletteEntry = null;
    _markPaletteRect = null;
    _markPaletteToken = null;
  }

  Widget _buildMarkPalette(BuildContext context) {
    final rect = _markPaletteRect!;
    const labels = {'fuzzy': '释义模糊', 'familiar': '眼熟', 'unknown': '完全不会'};
    return Stack(
      children: [
        Positioned.fill(
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            onTap: _hideMarkPalette,
          ),
        ),
        Positioned(
          key: const ValueKey('exam-mark-palette'),
          left: rect.left,
          top: rect.top,
          width: rect.width,
          height: rect.height,
          child: Material(
            elevation: 4,
            color: Theme.of(context).colorScheme.surface,
            borderRadius: BorderRadius.circular(6),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: [
                for (final level in examFamiliarityLevels)
                  Semantics(
                    label: labels[level],
                    selected: level == _markPaletteLevel,
                    child: GestureDetector(
                      key: ValueKey('exam-mark-$level'),
                      behavior: HitTestBehavior.opaque,
                      onTap: () {
                        _markPaletteLevel = level;
                        _commitMarkPalette();
                      },
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 80),
                        width: 34,
                        height: 28,
                        decoration: BoxDecoration(
                          color: examCurrentHighlightColor(level),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: level == _markPaletteLevel
                                ? Theme.of(context).colorScheme.onSurface
                                : Theme.of(context).colorScheme.outlineVariant,
                            width: level == _markPaletteLevel ? 2 : 1,
                          ),
                        ),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }

  Future<void> _setWordMark(ExamWordToken token, String requestedMark) async {
    _markRequestSerial += 1;
    final family = normalizeExamWordFamily(token.normalized);
    final currentMark = widget.currentMarks[family];
    final nextMark = currentMark == requestedMark ? 'none' : requestedMark;
    widget.onWordMarkChanged?.call(family, nextMark);
    try {
      final updated = await _inspect(token, userMark: nextMark);
      if (!mounted) return;
      widget.onInspectionChanged?.call(
        ExamWordInspection(
          occurrenceId: updated.occurrenceId,
          entryId: updated.entryId,
          word: updated.word,
          normalized: family,
          meanings: updated.meanings,
          userMark: updated.userMark,
          isUnknown: updated.isUnknown,
        ),
      );
    } finally {
      if (mounted) widget.onWordMarkChanged?.call(family, nextMark);
    }
  }

  void _rememberPointerDown(PointerDownEvent event) {
    final pending = _ExamPendingPointerTap(
      position: event.localPosition,
      globalPosition: event.position,
      startedAt: DateTime.now(),
      markSerial: _markRequestSerial,
    );
    pending.longPressTimer = Timer(const Duration(milliseconds: 360), () {
      if (!identical(_pendingPointerTaps[event.pointer], pending)) return;
      pending.longPressTriggered = true;
      _handleWordLongPress(pending.globalPosition);
    });
    _pendingPointerTaps[event.pointer] = pending;
  }

  void _cancelPointerTap(PointerEvent event) {
    _pendingPointerTaps.remove(event.pointer)?.longPressTimer?.cancel();
  }

  void _trackPointerMove(PointerMoveEvent event) {
    final pending = _pendingPointerTaps[event.pointer];
    if (pending == null || pending.longPressTriggered) return;
    if ((event.localPosition - pending.position).distance > 10) {
      _cancelPointerTap(event);
    }
  }

  void _handlePointerUp(
    PointerUpEvent event,
    List<_ExamRenderedTapRange> tapRanges,
  ) {
    final pending = _pendingPointerTaps.remove(event.pointer);
    pending?.longPressTimer?.cancel();
    if (pending == null ||
        pending.longPressTriggered ||
        DateTime.now().difference(pending.startedAt) >=
            const Duration(milliseconds: 300) ||
        (event.localPosition - pending.position).distance > 10) {
      return;
    }
    Future<void>.microtask(() async {
      if (!mounted || pending.markSerial != _markRequestSerial) return;
      final editable = _findRenderEditable();
      if (editable == null) return;
      final localPosition = editable.globalToLocal(event.position);
      final renderedTextLength = tapRanges.isEmpty
          ? 0
          : tapRanges.map((range) => range.end).reduce((a, b) => a > b ? a : b);
      _ExamRenderedTapRange? nearest;
      var nearestDistance = double.infinity;
      for (final range in tapRanges) {
        final boxes = editable.getBoxesForSelection(
          TextSelection(baseOffset: range.start, extentOffset: range.end),
        );
        for (final box in boxes) {
          final rawRect = box.toRect();
          final rect = Rect.fromLTRB(
            rawRect.left - 4,
            rawRect.top,
            rawRect.right + 4,
            rawRect.bottom,
          );
          if (!rect.contains(localPosition)) continue;
          final distance = rawRect.contains(localPosition)
              ? 0.0
              : (localPosition.dx - rawRect.center.dx).abs();
          if (distance < nearestDistance) {
            nearest = range;
            nearestDistance = distance;
          }
        }
      }
      if (nearest == null) {
        final offset = editable.getPositionForPoint(event.position).offset;
        for (final range in tapRanges) {
          final containsOffset =
              range.start <= offset &&
              (offset < range.end ||
                  (offset == range.end && range.end == renderedTextLength));
          if (!containsOffset) continue;
          final boxes = editable.getBoxesForSelection(
            TextSelection(baseOffset: range.start, extentOffset: range.end),
          );
          if (boxes.any((box) {
            final rect = box.toRect();
            return localPosition.dy >= rect.top &&
                localPosition.dy <= rect.bottom;
          })) {
            nearest = range;
            break;
          }
        }
      }
      if (nearest == null) return;
      final family = normalizeExamWordFamily(nearest.token.normalized);
      await _setWordMark(
        nearest.token,
        widget.currentMarks[family] ?? 'familiar',
      );
    });
  }

  RenderEditable? _findRenderEditable() {
    final root = _selectableTextKey.currentContext?.findRenderObject();
    if (root == null) return null;
    RenderEditable? result;
    void visit(RenderObject child) {
      if (result != null) return;
      if (child is RenderEditable) {
        result = child;
        return;
      }
      child.visitChildren(visit);
    }

    if (root is RenderEditable) return root;
    root.visitChildren(visit);
    return result;
  }

  EditableTextState? _findEditableTextState() {
    final root = _selectableTextKey.currentContext;
    if (root == null) return null;
    EditableTextState? result;
    void visit(Element child) {
      if (result != null) return;
      if (child is StatefulElement && child.state is EditableTextState) {
        result = child.state as EditableTextState;
        return;
      }
      child.visitChildElements(visit);
    }

    root.visitChildElements(visit);
    return result;
  }

  void _handleWordLongPress(Offset globalPosition) {
    final editable = _findRenderEditable();
    final editableState = _findEditableTextState();
    if (editable == null || editableState == null) return;
    final position = editable.getPositionForPoint(globalPosition);
    final renderedText = editableState.textEditingValue.text;
    final tokens = tokenizeExamTextForInteraction(renderedText);
    ExamWordToken? selectedToken;
    for (final token in tokens) {
      if (token.startOffset <= position.offset &&
          position.offset <= token.endOffset) {
        selectedToken = token;
        break;
      }
    }
    if (selectedToken == null) return;
    final token =
        _tokenForRenderedSelection(
          TextSelection(
            baseOffset: selectedToken.startOffset,
            extentOffset: selectedToken.endOffset,
          ),
          renderedText,
        ) ??
        selectedToken;
    final family = normalizeExamWordFamily(token.normalized);
    if (widget.currentMarks.containsKey(family)) {
      // A native selectable-text long press can finish after our timer. Clear
      // that selection first so Copy/Share cannot cover the color controls.
      editableState.userUpdateTextEditingValue(
        editableState.textEditingValue.copyWith(
          selection: TextSelection.collapsed(offset: position.offset),
        ),
        SelectionChangedCause.longPress,
      );
      editableState.hideToolbar();
      _showMarkPalette(token, globalPosition);
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) editableState.hideToolbar();
      });
      _nativeToolbarDismissTimer?.cancel();
      _nativeToolbarDismissTimer = Timer(const Duration(milliseconds: 220), () {
        if (!mounted || _markPaletteEntry == null) return;
        editableState.userUpdateTextEditingValue(
          editableState.textEditingValue.copyWith(
            selection: TextSelection.collapsed(offset: position.offset),
          ),
          SelectionChangedCause.longPress,
        );
        editableState.hideToolbar();
      });
      return;
    }
    editableState.userUpdateTextEditingValue(
      editableState.textEditingValue.copyWith(
        selection: TextSelection(
          baseOffset: selectedToken.startOffset,
          extentOffset: selectedToken.endOffset,
        ),
      ),
      SelectionChangedCause.longPress,
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) editableState.showToolbar();
    });
  }

  Future<void> _markSelectionUnknown(
    TextSelection selection,
    String selected,
  ) async {
    final start = selection.start.clamp(0, widget.text.length);
    final end = selection.end.clamp(0, widget.text.length);
    if (end <= start) return;
    var annotations = widget.annotations;
    final scope = '${widget.metadata['scope'] ?? 'passage'}';
    final alreadyMarked = annotations.any(
      (annotation) =>
          annotation.scope == scope &&
          annotation.startOffset == widget.offsetBase + start &&
          annotation.endOffset == widget.offsetBase + end,
    );
    if (!alreadyMarked) {
      annotations = await widget.client.saveAnnotation(
        annotationId:
            'annotation:${widget.articleId}:${widget.offsetBase + start}:${DateTime.now().microsecondsSinceEpoch}',
        articleId: widget.articleId,
        title: widget.title,
        body: widget.articleBody ?? widget.text,
        questionId: widget.metadata['questionId'] as String?,
        scope: scope,
        startOffset: widget.offsetBase + start,
        endOffset: widget.offsetBase + end,
        selectedText: selected,
        noteText: '',
        metadata: widget.metadata,
      );
    }
    final phrase = normalizeExamPhraseSelection(selected);
    final inspection = await widget.client.inspectWord(
      articleId: widget.articleId,
      title: widget.title,
      body: widget.articleBody ?? widget.text,
      token: ExamWordToken(
        text: phrase,
        normalized: phrase,
        startOffset: widget.offsetBase + start,
        endOffset: widget.offsetBase + end,
      ),
      sentenceText: widget.text,
      metadata: widget.metadata,
      userMark: 'unknown',
    );
    widget.onInspectionChanged?.call(inspection);
    if (!mounted) return;
    widget.onAnnotationsChanged?.call(annotations);
  }

  void _handleSelectionChanged(TextSelection selection) {
    _selectionLookupTimer?.cancel();
    final generation = ++_selectionLookupGeneration;
    if (_markPaletteEntry != null) return;
    if (selection.isCollapsed) return;
    final selected = widget.text.substring(selection.start, selection.end);
    final phrase = normalizeExamPhraseSelection(selected);
    if (!phrase.contains(RegExp(r'\s'))) return;
    _selectionLookupTimer = Timer(const Duration(milliseconds: 300), () {
      _showSelectionMeaning(selection, selected, generation);
    });
  }

  Future<void> _showSelectionMeaning(
    TextSelection selection,
    String selected,
    int generation,
  ) async {
    if (_selectionSheetOpen || generation != _selectionLookupGeneration) return;
    final phrase = normalizeExamPhraseSelection(selected);
    final inspection = await widget.client.inspectWord(
      articleId: widget.articleId,
      title: widget.title,
      body: widget.articleBody ?? widget.text,
      token: ExamWordToken(
        text: phrase,
        normalized: phrase,
        startOffset: widget.offsetBase + selection.start,
        endOffset: widget.offsetBase + selection.end,
      ),
      sentenceText: widget.text,
      metadata: widget.metadata,
    );
    if (!mounted || generation != _selectionLookupGeneration) return;
    _selectionSheetOpen = true;
    await showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (sheetContext) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(24, 0, 24, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(phrase, style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 12),
              Text(
                inspection.meanings.isEmpty
                    ? '词库中暂无释义'
                    : inspection.meanings.join('；'),
                style: Theme.of(context).textTheme.bodyLarge,
              ),
              const SizedBox(height: 20),
              FilledButton.icon(
                onPressed: inspection.isUnknown
                    ? null
                    : () async {
                        await _markSelectionUnknown(selection, selected);
                        if (sheetContext.mounted) {
                          Navigator.pop(sheetContext);
                        }
                      },
                icon: const Icon(Icons.bookmark_add_outlined),
                label: Text(inspection.isUnknown ? '已标记为不会' : '标记为不会'),
              ),
            ],
          ),
        ),
      ),
    );
    _selectionSheetOpen = false;
  }

  Widget _buildSelectionMenu(
    BuildContext context,
    EditableTextState editableTextState,
  ) {
    if (_markPaletteEntry != null) return const SizedBox.shrink();
    final selection = editableTextState.textEditingValue.selection;
    final token = _tokenForRenderedSelection(
      selection,
      editableTextState.textEditingValue.text,
    );
    final items = <ContextMenuButtonItem>[
      if (token != null)
        ContextMenuButtonItem(
          label: '标记',
          onPressed: () {
            final editable = editableTextState.renderEditable;
            final boxes = editable.getBoxesForSelection(selection);
            final anchor = boxes.isNotEmpty
                ? editable.localToGlobal(boxes.first.toRect().topCenter)
                : const Offset(80, 120);
            editableTextState.hideToolbar();
            _showMarkPalette(token, anchor);
          },
        ),
      ...editableTextState.contextMenuButtonItems.where(
        (item) =>
            item.type == ContextMenuButtonType.copy ||
            item.type == ContextMenuButtonType.share,
      ),
    ];
    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: editableTextState.contextMenuAnchors,
      buttonItems: items,
    );
  }

  ExamWordToken? _tokenForRenderedSelection(
    TextSelection selection,
    String renderedText,
  ) {
    if (selection.isCollapsed) return null;
    final selected = normalizeExamPhraseSelection(
      renderedText.substring(selection.start, selection.end),
    );
    if (selected.isEmpty || selected.contains(' ')) return null;
    for (final token in _tokens) {
      if (normalizeExamPhraseSelection(token.text) == selected) return token;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    if (_tokens.isEmpty || _recognizers.length != _tokens.length) {
      return Text(widget.text, style: widget.style);
    }
    final spans = <InlineSpan>[];
    var cursor = 0;
    for (var index = 0; index < _tokens.length; index++) {
      final token = _tokens[index];
      final displayStart = widget.text.indexOf(token.text, cursor);
      if (displayStart < 0) continue;
      final displayEnd = displayStart + token.text.length;
      if (displayStart > cursor) {
        final separator = widget.text.substring(cursor, displayStart);
        if (index > 0) {
          _appendTapSeparatorSpans(spans, separator, _recognizers[index - 1]);
        } else {
          _appendTapSeparatorSpans(spans, separator, _recognizers[index]);
        }
      }
      final presentation = resolveExamWordPresentation(
        normalized: token.normalized,
        mode: widget.mode,
        currentArticleMarks: widget.currentMarks.keys.toSet(),
        priorArticleMarks: widget.priorWords,
        currentMarkKinds: widget.currentMarks,
        priorMarkKinds: widget.priorMarks,
      );
      final scope = '${widget.metadata['scope'] ?? 'passage'}';
      final persistedStart = widget.offsetBase + displayStart;
      final persistedEnd = widget.offsetBase + displayEnd;
      final rangeMarked = widget.annotations.any(
        (annotation) =>
            annotation.scope == scope &&
            annotation.startOffset < persistedEnd &&
            annotation.endOffset > persistedStart,
      );
      final highlight = rangeMarked
          ? ExamWordHighlight.familiar
          : presentation.highlight;
      final foregroundColor = widget.emphasisWords.contains(token.normalized)
          ? Theme.of(context).colorScheme.error
          : Theme.of(context).colorScheme.onSurface;
      final fontWeight = widget.emphasisWords.contains(token.normalized)
          ? FontWeight.w700
          : null;
      final tokenText = widget.text.substring(displayStart, displayEnd);
      if (highlight == ExamWordHighlight.mixed) {
        final split = (tokenText.length / 2).ceil();
        spans
          ..add(
            TextSpan(
              text: tokenText.substring(0, split),
              recognizer: _recognizers[index],
              style: TextStyle(
                color: foregroundColor,
                backgroundColor: examPriorHighlightColor(
                  presentation.priorMarkLevel ?? 'unknown',
                ),
                fontWeight: fontWeight,
              ),
            ),
          )
          ..add(
            TextSpan(
              text: tokenText.substring(split),
              recognizer: _recognizers[index],
              style: TextStyle(
                color: foregroundColor,
                backgroundColor: examCurrentHighlightColor(
                  presentation.currentMarkLevel ?? 'unknown',
                ),
                fontWeight: fontWeight,
              ),
            ),
          );
      } else {
        final backgroundColor = switch (highlight) {
          ExamWordHighlight.fuzzy => examCurrentHighlightColor('fuzzy'),
          ExamWordHighlight.familiar => examCurrentHighlightColor('familiar'),
          ExamWordHighlight.unknown => examCurrentHighlightColor('unknown'),
          ExamWordHighlight.priorArticle => examPriorHighlightColor(
            presentation.priorMarkLevel ?? 'unknown',
          ),
          ExamWordHighlight.none || ExamWordHighlight.mixed => null,
        };
        spans.add(
          TextSpan(
            text: tokenText,
            recognizer: _recognizers[index],
            style: TextStyle(
              color: foregroundColor,
              backgroundColor: backgroundColor,
              fontWeight: fontWeight,
            ),
          ),
        );
      }
      if (presentation.showMeaning && widget.showInlineMeanings) {
        final family = normalizeExamWordFamily(token.normalized);
        final rawMeaning = resolveExamMarkedMeaning(
          widget.currentMeanings,
          token.normalized,
        );
        final meaning = pickExamContextMeaning(
          [rawMeaning],
          normalized: family,
          context: widget.text.substring(
            (displayStart - 80).clamp(0, widget.text.length),
            (displayEnd + 80).clamp(0, widget.text.length),
          ),
          translation: widget.contextTranslation,
        );
        if (meaning.isNotEmpty) {
          spans.add(
            TextSpan(
              text: '（$meaning）',
              style: TextStyle(
                color: Theme.of(context).colorScheme.tertiary,
                fontWeight: FontWeight.w600,
              ),
            ),
          );
        }
      }
      cursor = displayEnd;
    }
    if (cursor < widget.text.length) {
      final trailing = widget.text.substring(cursor);
      if (_recognizers.isNotEmpty) {
        _appendTapSeparatorSpans(spans, trailing, _recognizers.last);
      } else {
        _appendPlainSpans(spans, trailing);
      }
    }
    final scope = '${widget.metadata['scope'] ?? 'passage'}';
    final phraseAnnotations = widget.annotations
        .where((annotation) {
          final normalized = normalizeExamPhraseSelection(
            annotation.selectedText,
          );
          return widget.mode == ExamPracticeMode.analysis &&
              widget.showInlineMeanings &&
              annotation.scope == scope &&
              normalized.contains(RegExp(r'\s')) &&
              (widget.currentMeanings[normalized] ?? '').isNotEmpty;
        })
        .toList(growable: false);
    final rootSpan = TextSpan(style: widget.style, children: spans);
    final tapRanges = <_ExamRenderedTapRange>[];
    var renderedOffset = 0;
    for (final span in spans.whereType<TextSpan>()) {
      final textLength = span.text?.length ?? 0;
      final recognizerIndex = span.recognizer == null
          ? -1
          : _recognizers.indexWhere(
              (recognizer) => identical(recognizer, span.recognizer),
            );
      if (recognizerIndex >= 0 && textLength > 0) {
        tapRanges.add(
          _ExamRenderedTapRange(
            start: renderedOffset,
            end: renderedOffset + textLength,
            token: _tokens[recognizerIndex],
          ),
        );
      }
      renderedOffset += textLength;
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Listener(
          behavior: HitTestBehavior.translucent,
          onPointerDown: _rememberPointerDown,
          onPointerMove: _trackPointerMove,
          onPointerCancel: _cancelPointerTap,
          onPointerUp: (event) => _handlePointerUp(event, tapRanges),
          child: SelectableText.rich(
            key: _selectableTextKey,
            rootSpan,
            contextMenuBuilder: _buildSelectionMenu,
            onSelectionChanged: (selection, _) =>
                _handleSelectionChanged(selection),
          ),
        ),
        for (final annotation in phraseAnnotations)
          Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: annotation.selectedText,
                    style: const TextStyle(
                      backgroundColor: Color(0xFFFFE082),
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  TextSpan(
                    text:
                        '（${widget.currentMeanings[normalizeExamPhraseSelection(annotation.selectedText)]}）',
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.tertiary,
                    ),
                  ),
                ],
              ),
            ),
          ),
      ],
    );
  }

  void _appendPlainSpans(List<InlineSpan> spans, String text) {
    if (!widget.compactClozeMarkers) {
      spans.add(TextSpan(text: text));
      return;
    }
    final marker = RegExp(r'\(\d{1,2}\)');
    var cursor = 0;
    for (final match in marker.allMatches(text)) {
      if (match.start > cursor) {
        spans.add(TextSpan(text: text.substring(cursor, match.start)));
      }
      spans.add(
        TextSpan(
          text: match.group(0),
          style: TextStyle(
            fontSize: ((widget.style?.fontSize ?? 14) - 2).clamp(10, 14),
            color: Theme.of(context).colorScheme.secondary,
            fontWeight: FontWeight.w600,
          ),
        ),
      );
      cursor = match.end;
    }
    if (cursor < text.length) {
      spans.add(TextSpan(text: text.substring(cursor)));
    }
  }

  void _appendTapSeparatorSpans(
    List<InlineSpan> spans,
    String text,
    GestureRecognizer recognizer,
  ) {
    if (text.contains('\n') ||
        text.contains('\r') ||
        (widget.compactClozeMarkers && RegExp(r'\(\d{1,2}\)').hasMatch(text))) {
      _appendPlainSpans(spans, text);
      return;
    }
    spans.add(TextSpan(text: text, recognizer: recognizer));
  }
}

class _SelectorField<T> extends StatelessWidget {
  const _SelectorField({
    required this.label,
    required this.value,
    required this.items,
    required this.onChanged,
  });

  final String label;
  final T? value;
  final List<DropdownMenuItem<T>> items;
  final ValueChanged<T?> onChanged;

  @override
  Widget build(BuildContext context) {
    return DropdownButtonFormField<T>(
      key: ValueKey('$label:$value'),
      initialValue: value,
      isExpanded: true,
      decoration: InputDecoration(
        labelText: label,
        border: const OutlineInputBorder(),
      ),
      items: items,
      onChanged: items.isEmpty ? null : onChanged,
    );
  }
}

class _ExamLoadMessage extends StatelessWidget {
  const _ExamLoadMessage({required this.error, required this.onRetry});

  final Object? error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.error_outline_rounded, size: 32),
            const SizedBox(height: 8),
            Text(
              '\u8bd5\u5377\u52a0\u8f7d\u5931\u8d25\uff1a${error ?? '\u672a\u77e5\u9519\u8bef'}',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 12),
            OutlinedButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh_rounded),
              label: const Text('\u91cd\u8bd5'),
            ),
          ],
        ),
      ),
    );
  }
}

String _examLabel(String exam) => switch (exam) {
  'cet4' => '\u5927\u5b66\u82f1\u8bed\u56db\u7ea7',
  'cet6' => '\u5927\u5b66\u82f1\u8bed\u516d\u7ea7',
  'kaoyan-english-1' => '\u8003\u7814\u82f1\u8bed\u4e00',
  'kaoyan-english-2' => '\u8003\u7814\u82f1\u8bed\u4e8c',
  _ => exam,
};

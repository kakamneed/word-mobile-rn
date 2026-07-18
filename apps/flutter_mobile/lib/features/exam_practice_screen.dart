import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';

enum TodayLearningContent { words, practice }

enum ExamPracticeMode { doing, analysis }

enum ExamReadingPane { passage, questions }

enum ExamWordHighlight { none, priorArticle, currentArticle }

class ExamWordPresentation {
  const ExamWordPresentation({
    required this.highlight,
    required this.showMeaning,
  });

  final ExamWordHighlight highlight;
  final bool showMeaning;

  @override
  bool operator ==(Object other) =>
      other is ExamWordPresentation &&
      other.highlight == highlight &&
      other.showMeaning == showMeaning;

  @override
  int get hashCode => Object.hash(highlight, showMeaning);
}

ExamWordPresentation resolveExamWordPresentation({
  required String normalized,
  required ExamPracticeMode mode,
  required Set<String> currentArticleMarks,
  required Set<String> priorArticleMarks,
}) {
  final current = currentArticleMarks.contains(normalized);
  if (current) {
    return ExamWordPresentation(
      highlight: ExamWordHighlight.currentArticle,
      showMeaning: mode == ExamPracticeMode.analysis,
    );
  }
  return ExamWordPresentation(
    highlight: priorArticleMarks.contains(normalized)
        ? ExamWordHighlight.priorArticle
        : ExamWordHighlight.none,
    showMeaning: false,
  );
}

String nextExamWordMark(String normalized, Set<String> currentArticleMarks) =>
    currentArticleMarks.contains(normalized) ? 'none' : 'unknown';

String pickExamContextMeaning(List<String> meanings) {
  for (final meaning in meanings) {
    final compact = meaning
        .split(RegExp(r'[；;]'))
        .map((part) => part.trim())
        .firstWhere((part) => part.isNotEmpty, orElse: () => '');
    if (compact.isNotEmpty) return compact;
  }
  return '';
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

String formatClozePassage(String source) {
  final passage = formatExamPassage(source);
  return passage.replaceAllMapped(
    RegExp(r'(?<![\d(])(\d{1,2})(?![\d)])'),
    (match) => '(${match.group(1)})',
  );
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
  Set<String> _priorWords = {};
  List<ExamTextAnnotation> _annotations = const [];
  ExamPracticeMode _mode = ExamPracticeMode.doing;
  ExamSubmissionResult? _submission;
  double _bubbleAlignment = 0;
  bool _saving = false;
  bool _questionBookmarkSeeded = false;
  bool _switchingReadingPosition = false;
  double _questionBaselineOffset = 0;
  Set<String> _causalWords = {};
  bool _showParagraphTranslations = false;

  ExamSection get _section => widget.paper.sections[_sectionIndex];
  bool get _isClozeSection =>
      _section.title.contains('\u5b8c\u578b') ||
      _section.title.contains('\u5b8c\u5f62');
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
    _restoreAttempts();
    _hydrateAnnotations();
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
    if (_submission != null) return;
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

  Future<List<ExamCausalFinding>> _analyzeSectionWrongAnswers() async {
    final result = _submission;
    if (result == null) return const [];
    final wrongQuestions = _section.questions
        .where((question) => result.incorrectQuestionIds.contains(question.id))
        .toList(growable: false);
    final findings = <ExamCausalFinding>[];
    for (final question in wrongQuestions) {
      final analysis = await widget.client.analyzeQuestionVocabulary(
        exam: widget.paper.exam,
        paperId: widget.paper.id,
        sectionId: _section.id,
        questionId: question.id,
        attemptId: _attemptIdFor(question),
      );
      for (final candidate
          in (analysis['candidates'] as List<dynamic>? ?? const [])) {
        if (candidate is! Map<String, dynamic>) continue;
        final word = '${candidate['word'] ?? ''}'.trim();
        if (word.isEmpty) continue;
        findings.add(
          ExamCausalFinding(
            questionNumber: question.number,
            word: word,
            reasoning: '${candidate['reasoning'] ?? ''}'.trim(),
            confidence: ((candidate['confidence'] as num?) ?? 0).toDouble(),
          ),
        );
      }
    }
    if (mounted) {
      setState(() {
        _causalWords = {
          for (final finding in findings) ...[
            finding.word.toLowerCase(),
            ...finding.word
                .toLowerCase()
                .split(RegExp(r'\s+'))
                .where((word) => word.isNotEmpty),
          ],
        };
      });
    }
    return findings;
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

  String get _displayPassage => _isClozeSection
      ? formatClozePassage(_section.passage)
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
          .map((annotation) => annotation.selectedText.trim().toLowerCase())
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
        _currentMeanings = state.currentMeanings.map(
          (word, meaning) => MapEntry(word, pickExamContextMeaning([meaning])),
        );
        _priorWords = state.priorWords;
        _annotations = state.annotations;
      });
    } catch (_) {
      // Annotation hydration is additive and must not block offline practice.
    }
  }

  void _onInspectionChanged(ExamWordInspection inspection) {
    setState(() {
      final next = Map<String, String>.from(_currentMeanings);
      if (inspection.isUnknown) {
        next[inspection.normalized] = pickExamContextMeaning(
          inspection.meanings,
        );
      } else {
        next.remove(inspection.normalized);
      }
      _currentMeanings = next;
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
      body: ExamReadingPositionBubble(
        active: _positionMemory.active,
        bubbleAlignment: _bubbleAlignment,
        onSwitch: _switchReadingPosition,
        onBubbleAlignmentChanged: (value) =>
            setState(() => _bubbleAlignment = value),
        child: ExamContinuousReader(
          controller: _readerController,
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 96),
          children: [
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
                  priorWords: _priorWords,
                  annotations: _annotations,
                  compactClozeMarkers: _isClozeSection,
                  emphasisWords: _causalWords,
                  showInlineMeanings: _mode == ExamPracticeMode.analysis,
                  onInspectionChanged: _onInspectionChanged,
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
                  interactiveTextBuilder: (text, scope) => ExamInteractiveText(
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
                    priorWords: _priorWords,
                    annotations: _annotations,
                    emphasisWords: _causalWords,
                    showInlineMeanings: _mode == ExamPracticeMode.analysis,
                    onInspectionChanged: _onInspectionChanged,
                    onAnnotationsChanged: (value) =>
                        setState(() => _annotations = value),
                  ),
                ),
                const SizedBox(height: 16),
              ],
          ],
        ),
      ),
      bottomNavigationBar: _isSubjectiveSection
          ? null
          : SafeArea(
              minimum: const EdgeInsets.fromLTRB(16, 8, 16, 12),
              child: FilledButton.icon(
                onPressed: _saving
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
  final Future<List<ExamCausalFinding>> Function() onAnalyze;
  final Set<String> currentWords;
  final Set<String> repeatedWords;

  @override
  State<ExamSectionReportScreen> createState() =>
      _ExamSectionReportScreenState();
}

class _ExamSectionReportScreenState extends State<ExamSectionReportScreen> {
  String? _selectedQuestionId;
  bool _analyzing = false;
  List<ExamCausalFinding>? _findings;
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
      final findings = await widget.onAnalyze();
      if (mounted) setState(() => _findings = findings);
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
            label: const Text('AI 分析全部错题'),
          ),
          if (_analysisError != null) ...[
            const SizedBox(height: 8),
            Text(
              '分析失败：$_analysisError',
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ],
          if (_findings != null) ...[
            const SizedBox(height: 12),
            if (_findings!.isEmpty)
              const Text('现有标记与作答证据不足以确定致错词。')
            else
              for (final finding in _findings!)
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
          if (selected != null) ...[
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
        ],
      ),
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
    this.priorWords = const {},
    this.annotations = const [],
    this.compactClozeMarkers = false,
    this.emphasisWords = const {},
    this.showInlineMeanings = true,
    this.onInspectionChanged,
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
  final Set<String> priorWords;
  final List<ExamTextAnnotation> annotations;
  final bool compactClozeMarkers;
  final Set<String> emphasisWords;
  final bool showInlineMeanings;
  final ValueChanged<ExamWordInspection>? onInspectionChanged;
  final ValueChanged<List<ExamTextAnnotation>>? onAnnotationsChanged;

  @override
  State<ExamInteractiveText> createState() => _ExamInteractiveTextState();
}

class _ExamInteractiveTextState extends State<ExamInteractiveText> {
  List<ExamWordToken> _tokens = const [];
  final List<TapGestureRecognizer> _recognizers = [];
  TextSelection? _selection;

  @override
  void initState() {
    super.initState();
    _loadTokens();
  }

  @override
  void didUpdateWidget(covariant ExamInteractiveText oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.text != widget.text ||
        oldWidget.articleId != widget.articleId) {
      _loadTokens();
    }
  }

  @override
  void dispose() {
    _disposeRecognizers();
    super.dispose();
  }

  void _disposeRecognizers() {
    for (final recognizer in _recognizers) {
      recognizer.dispose();
    }
    _recognizers.clear();
  }

  Future<void> _loadTokens() async {
    final text = widget.text;
    try {
      final tokens = await widget.client.tokenizeText(text);
      if (!mounted || text != widget.text) return;
      setState(() {
        _tokens = tokens;
        _disposeRecognizers();
        for (final token in tokens) {
          _recognizers.add(
            TapGestureRecognizer()..onTap = () => _toggleWord(token),
          );
        }
      });
    } catch (_) {
      if (mounted) setState(() => _tokens = const []);
    }
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

  Future<void> _toggleWord(ExamWordToken token) async {
    final updated = await _inspect(
      token,
      userMark: nextExamWordMark(
        token.normalized,
        widget.currentMeanings.keys.toSet(),
      ),
    );
    if (!mounted) return;
    widget.onInspectionChanged?.call(updated);
  }

  Future<void> _saveSelection() async {
    final selection = _selection;
    if (selection == null || selection.isCollapsed) return;
    final start = selection.start.clamp(0, widget.text.length);
    final end = selection.end.clamp(0, widget.text.length);
    if (end <= start) return;
    final selected = widget.text.substring(start, end);
    final controller = TextEditingController();
    final note = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('黄色标记与笔记'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(selected, maxLines: 4, overflow: TextOverflow.ellipsis),
            const SizedBox(height: 12),
            TextField(
              controller: controller,
              minLines: 2,
              maxLines: 4,
              decoration: const InputDecoration(
                labelText: '笔记（可选）',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, controller.text.trim()),
            child: const Text('保存'),
          ),
        ],
      ),
    );
    controller.dispose();
    if (note == null) return;
    final annotations = await widget.client.saveAnnotation(
      annotationId:
          'annotation:${widget.articleId}:${widget.offsetBase + start}:${DateTime.now().microsecondsSinceEpoch}',
      articleId: widget.articleId,
      title: widget.title,
      body: widget.articleBody ?? widget.text,
      questionId: widget.metadata['questionId'] as String?,
      scope: '${widget.metadata['scope'] ?? 'passage'}',
      startOffset: widget.offsetBase + start,
      endOffset: widget.offsetBase + end,
      selectedText: selected,
      noteText: note,
      metadata: widget.metadata,
    );
    final phrase = selected.trim();
    if (phrase.contains(RegExp(r'\s'))) {
      final inspection = await widget.client.inspectWord(
        articleId: widget.articleId,
        title: widget.title,
        body: widget.articleBody ?? widget.text,
        token: ExamWordToken(
          text: phrase,
          normalized: phrase.toLowerCase(),
          startOffset: widget.offsetBase + start,
          endOffset: widget.offsetBase + end,
        ),
        sentenceText: widget.text,
        metadata: widget.metadata,
        userMark: 'unknown',
      );
      widget.onInspectionChanged?.call(inspection);
    }
    if (!mounted) return;
    setState(() => _selection = null);
    widget.onAnnotationsChanged?.call(annotations);
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
        _appendPlainSpans(spans, widget.text.substring(cursor, displayStart));
      }
      final presentation = resolveExamWordPresentation(
        normalized: token.normalized,
        mode: widget.mode,
        currentArticleMarks: widget.currentMeanings.keys.toSet(),
        priorArticleMarks: widget.priorWords,
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
          ? ExamWordHighlight.currentArticle
          : presentation.highlight;
      final backgroundColor = switch (highlight) {
        ExamWordHighlight.currentArticle => const Color(0xFFFFE082),
        ExamWordHighlight.priorArticle => const Color(0xFFE1BEE7),
        ExamWordHighlight.none => null,
      };
      spans.add(
        TextSpan(
          text: widget.text.substring(displayStart, displayEnd),
          recognizer: _recognizers[index],
          style: TextStyle(
            color: widget.emphasisWords.contains(token.normalized)
                ? Theme.of(context).colorScheme.error
                : Theme.of(context).colorScheme.onSurface,
            backgroundColor: backgroundColor,
            fontWeight: widget.emphasisWords.contains(token.normalized)
                ? FontWeight.w700
                : null,
          ),
        ),
      );
      if (presentation.showMeaning && widget.showInlineMeanings) {
        final meaning = widget.currentMeanings[token.normalized] ?? '';
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
      _appendPlainSpans(spans, widget.text.substring(cursor));
    }
    final scope = '${widget.metadata['scope'] ?? 'passage'}';
    final phraseAnnotations = widget.annotations
        .where((annotation) {
          final normalized = annotation.selectedText.trim().toLowerCase();
          return widget.mode == ExamPracticeMode.analysis &&
              widget.showInlineMeanings &&
              annotation.scope == scope &&
              normalized.contains(RegExp(r'\s')) &&
              (widget.currentMeanings[normalized] ?? '').isNotEmpty;
        })
        .toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SelectableText.rich(
          TextSpan(style: widget.style, children: spans),
          onSelectionChanged: (selection, _) {
            if (selection.isCollapsed) return;
            setState(() => _selection = selection);
          },
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
                        '（${widget.currentMeanings[annotation.selectedText.trim().toLowerCase()]}）',
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.tertiary,
                    ),
                  ),
                ],
              ),
            ),
          ),
        if (_selection != null && !_selection!.isCollapsed)
          TextButton.icon(
            onPressed: _saveSelection,
            icon: const Icon(Icons.border_color_outlined, size: 17),
            label: const Text('黄色标记 / 笔记'),
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

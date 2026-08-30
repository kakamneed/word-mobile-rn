import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../sdk/sdk.dart';
import '../supabase/word_comment_service.dart';
import '../widgets/crocodile_frame_animation.dart';

class StudyScreen extends StatefulWidget {
  const StudyScreen({
    super.key,
    required this.sdk,
    required this.mode,
    this.resumeHint,
    this.onCloseToToday,
    this.onOpenStudyMode,
  });

  final WordSdk sdk;
  final String mode;
  final ResumeSessionHint? resumeHint;
  final VoidCallback? onCloseToToday;
  final Future<void> Function(String mode)? onOpenStudyMode;

  @override
  State<StudyScreen> createState() => _StudyScreenState();
}

class _StudyScreenState extends State<StudyScreen> {
  final TextEditingController _answerController = TextEditingController();
  final FocusNode _answerFocusNode = FocusNode();
  final PageController _pageController = PageController();
  final WordCommentService _commentService = WordCommentService();
  final ValueNotifier<bool> _hasTypedAnswer = ValueNotifier(false);
  final Stopwatch _responseStopwatch = Stopwatch();
  String? _selectedChoice;

  StartSessionResponse? _session;
  SubmitAnswerResponse? _latestResponse;
  StudyQuestion? _lastSubmittedQuestion;
  StudyResult? _lastSubmittedResult;
  String? _lastSubmittedResponse;
  String? _activeSubmitQuestionId;
  final Set<String> _masteredEntrySourceIds = <String>{};
  List<StudyQuestion> _newlyMasteredQuestions = const [];
  final Set<String> _hintUsedQuestionIds = <String>{};
  int _masteredCount = 0;
  StudyQuestion? _pendingNextQuestion;
  SessionProgress? _pendingNextProgress;
  bool _pendingNextVisible = false;
  bool _pendingNextTransitioning = false;
  CompleteSessionResponse? _completion;
  bool _completionVisible = false;
  String? _error;
  bool _loading = true;
  bool _submitting = false;
  bool _showingHintPrompt = false;
  int _visiblePageIndex = 0;

  @override
  void initState() {
    super.initState();
    _answerController.addListener(_handleAnswerChanged);
    _start();
  }

  @override
  void didUpdateWidget(covariant StudyScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    final modeChanged = oldWidget.mode != widget.mode;
    final resumeChanged =
        _resumeHintSignature(oldWidget.resumeHint) !=
        _resumeHintSignature(widget.resumeHint);
    if (modeChanged || resumeChanged) {
      _start();
    }
  }

  @override
  void dispose() {
    _answerController.removeListener(_handleAnswerChanged);
    _hasTypedAnswer.dispose();
    _answerFocusNode.dispose();
    _answerController.dispose();
    _pageController.dispose();
    super.dispose();
  }

  void _handleAnswerChanged() {
    final hasAnswer = _answerController.text.trim().isNotEmpty;
    if (_hasTypedAnswer.value != hasAnswer) {
      _hasTypedAnswer.value = hasAnswer;
    }
  }

  String _resumeHintSignature(ResumeSessionHint? hint) {
    if (hint == null) {
      return '';
    }
    return '${hint.hasResume}:${hint.mode}:${hint.current}:${hint.total}:${hint.word}';
  }

  Future<void> _start() async {
    setState(() {
      _loading = true;
      _error = null;
      _latestResponse = null;
      _lastSubmittedQuestion = null;
      _lastSubmittedResult = null;
      _lastSubmittedResponse = null;
      _activeSubmitQuestionId = null;
      _masteredEntrySourceIds.clear();
      _newlyMasteredQuestions = const [];
      _hintUsedQuestionIds.clear();
      _masteredCount = 0;
      _pendingNextQuestion = null;
      _pendingNextProgress = null;
      _pendingNextVisible = false;
      _pendingNextTransitioning = false;
      _completion = null;
      _completionVisible = false;
      _session = null;
      _selectedChoice = null;
      _visiblePageIndex = 0;
      _submitting = false;
    });
    _answerController.clear();
    try {
      final response = await _startWithBestAvailableSeed();
      setState(() {
        _session = response;
        _masteredCount = response.masteredCount;
      });
      _jumpToCurrentFeedPage();
      _restartResponseTimer();
      _focusAnswerInputForCurrentQuestion();
    } catch (error) {
      _error = error.toString();
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  Future<StartSessionResponse> _startWithBestAvailableSeed() async {
    if (widget.resumeHint?.hasResume == true ||
        !_modeUsesQuestionTypeWeights(widget.mode)) {
      return widget.sdk.study.startSession(
        mode: widget.mode,
        entrySourceIds: const [],
      );
    }
    final plan = await widget.sdk.plan.getActivePlan();
    return widget.sdk.study.startSession(
      mode: widget.mode,
      entrySourceIds: const [],
      questionTypeWeights: _questionTypeWeightsForMode(
        plan?.questionTypeWeightsByMode,
        widget.mode,
      ),
    );
  }

  Future<void> _submit({
    StudyQuestion? questionOverride,
    String? explicitResponse,
    bool allowEmpty = false,
  }) async {
    final question = questionOverride ?? _currentQuestion;
    if (question == null) return;
    if (!_shouldAcceptStudySubmit(
      submitting: _submitting,
      activeQuestionId: _activeSubmitQuestionId,
      questionId: question.questionId,
    )) {
      return;
    }

    final responseText =
        (explicitResponse ?? _selectedChoice ?? _answerController.text).trim();
    if (responseText.isEmpty && !allowEmpty) return;
    _activeSubmitQuestionId = question.questionId;
    _settleAnswerInputBeforeSubmit(question: question);
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final response = await widget.sdk.study.submitAnswer(
        questionId: question.questionId,
        response: responseText,
        responseTimeMs: _elapsedResponseMs,
        hintUsed: _hintUsedForStudySubmit(
          mode: widget.mode,
          questionId: question.questionId,
          clickedQuestionIds: _hintUsedQuestionIds,
        ),
        studyMode: widget.mode,
      );
      _answerController.clear();
      final masteredCountBeforeSubmit = _masteredCount;
      final submittedQuestion = submittedQuestionWithFeedbackForTest(
        response.answeredQuestions,
        question,
        response.result,
      );
      setState(() {
        _latestResponse = response;
        _masteredCount = response.masteredCount;
        if (response.masteredCount > masteredCountBeforeSubmit) {
          _masteredEntrySourceIds.add(submittedQuestion.entrySourceId);
        }
        _newlyMasteredQuestions = newlyMasteredQuestionsForTest(
          existing: _newlyMasteredQuestions,
          question: submittedQuestion,
          masteredCountBefore: masteredCountBeforeSubmit,
          masteredCountAfter: response.masteredCount,
        );
        _lastSubmittedQuestion = submittedQuestion;
        _lastSubmittedResult = response.result;
        _lastSubmittedResponse = responseText;
        _pendingNextQuestion = response.isComplete
            ? null
            : response.currentQuestion ?? _pendingNextQuestion;
        _pendingNextProgress = response.isComplete
            ? null
            : response.currentQuestion == null
            ? _pendingNextProgress
            : response.progress;
        _selectedChoice = null;
        if (_session != null) {
          final answeredQuestions = mergeLatestAnsweredQuestionForTest(
            _mergeStableAnsweredFeed(
              _session!.answeredQuestions,
              response.answeredQuestions,
            ),
            submittedQuestion,
            response.result,
          );
          _session = StartSessionResponse(
            session: _session!.session,
            currentQuestion: submittedQuestion,
            progress: response.progress,
            answeredQuestions: answeredQuestions,
            masteredCount: response.masteredCount,
          );
        }
      });
      if (response.isComplete && response.summary != null) {
        setState(() {
          _completion = CompleteSessionResponse(
            summary: response.summary!,
            nextAction: response.nextAction ?? 'Return to today',
          );
          _completionVisible = false;
        });
      }
      await _maybeShowHintPrompt(response);
    } catch (error) {
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _submitting = false;
          if (_activeSubmitQuestionId == question.questionId) {
            _activeSubmitQuestionId = null;
          }
        });
      }
    }
  }

  Future<void> _markEntryMastered(
    StudyQuestion question, {
    required bool wasAnswered,
  }) async {
    if (_submitting) return;
    if (!wasAnswered) {
      _settleAnswerInputBeforeSubmit(question: question);
    }
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final masteredCountBeforeMark = _masteredCount;
      final response = await widget.sdk.study.markEntryMastered(
        entrySourceId: question.entrySourceId,
      );
      if (!mounted) return;
      setState(() {
        _masteredEntrySourceIds.add(question.entrySourceId);
        _masteredCount = response.masteredCount;
        _newlyMasteredQuestions = newlyMasteredQuestionsForTest(
          existing: _newlyMasteredQuestions,
          question: question,
          masteredCountBefore: masteredCountBeforeMark,
          masteredCountAfter: response.masteredCount,
        );
        final session = _session;
        final mergedAnswers = session == null
            ? response.answeredQuestions
            : _mergeStableAnsweredFeed(
                session.answeredQuestions,
                response.answeredQuestions,
              );
        if (wasAnswered && session != null) {
          final isDisplayedSubmittedQuestion =
              session.currentQuestion.questionId == question.questionId;
          _session = StartSessionResponse(
            session: session.session,
            currentQuestion: isDisplayedSubmittedQuestion
                ? session.currentQuestion
                : response.currentQuestion ?? session.currentQuestion,
            progress: response.progress,
            answeredQuestions: mergedAnswers,
            masteredCount: response.masteredCount,
          );
          if (isDisplayedSubmittedQuestion) {
            _pendingNextQuestion = response.currentQuestion;
            _pendingNextProgress = response.currentQuestion == null
                ? null
                : response.progress;
          }
        } else {
          _selectedChoice = null;
          _lastSubmittedQuestion = null;
          _lastSubmittedResult = null;
          _lastSubmittedResponse = null;
          _pendingNextQuestion = null;
          _pendingNextProgress = null;
          _answerController.clear();
        }
        if (!wasAnswered &&
            response.currentQuestion != null &&
            session != null) {
          _session = StartSessionResponse(
            session: session.session,
            currentQuestion: response.currentQuestion!,
            progress: response.progress,
            answeredQuestions: mergedAnswers,
            masteredCount: response.masteredCount,
          );
        }
        if (response.isComplete && response.summary != null) {
          _completion = CompleteSessionResponse(
            summary: response.summary!,
            nextAction: response.nextAction ?? 'Return to today',
          );
          _completionVisible = _shouldRevealCompletionAfterMastered(
            wasAnswered: wasAnswered,
            isComplete: response.isComplete,
          );
        }
      });
      _showStudyActionSnackBar(
        _studyMasteredSuccessMessage(wasAnswered: wasAnswered),
      );
      if (_shouldRevealCompletionAfterMastered(
        wasAnswered: wasAnswered,
        isComplete: response.isComplete,
      )) {
        _advanceToCompletionSummary();
      }
      if (!response.isComplete && !wasAnswered) {
        _restartResponseTimer();
        _focusAnswerInputForCurrentQuestion();
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
        });
        _showStudyActionSnackBar(_studyActionFailureMessage('mastered'));
      }
    } finally {
      if (mounted) {
        setState(() {
          _submitting = false;
        });
      }
    }
  }

  Future<void> _revealCurrentAnswer(StudyQuestion question) async {
    if (_submitting) return;
    _answerController.clear();
    setState(() {
      _selectedChoice = null;
    });
    await _submit(questionOverride: question, allowEmpty: true);
  }

  Future<void> _acceptDispute(_StudyFeedItem item) async {
    final question = item.question;
    final result = item.result;
    final submitted = item.responseOverride ?? result?.userResponse ?? '';
    if (question == null ||
        result == null ||
        _submitting ||
        !_canDisputeAnsweredQuestion(question, result, submitted)) {
      return;
    }
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final response = await widget.sdk.study.acceptDisputedMeaning(
        questionId: question.questionId,
        submittedAnswer: submitted,
      );
      if (!mounted) return;
      setState(() {
        if (_session != null) {
          _session = StartSessionResponse(
            session: _session!.session,
            currentQuestion: _session!.currentQuestion,
            progress: response.progress,
            answeredQuestions: _mergeStableAnsweredFeed(
              _session!.answeredQuestions,
              response.answeredQuestions,
            ),
            masteredCount: _session!.masteredCount,
          );
        }
        _lastSubmittedQuestion = question;
        _lastSubmittedResult = response.result;
        _lastSubmittedResponse = submitted;
      });
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(_studyDisputeAcceptedMessage())));
    } catch (error) {
      if (mounted) {
        _showStudyActionSnackBar(_studyActionFailureMessage('dispute'));
      }
    } finally {
      if (mounted) {
        setState(() {
          _submitting = false;
        });
      }
    }
  }

  void _showStudyActionSnackBar(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  // ignore: unused_element
  Future<void> _advance() async {
    final response = _latestResponse;
    if (response == null) return;
    if (response.isComplete && response.summary != null) {
      setState(() {
        _completion = CompleteSessionResponse(
          summary: response.summary!,
          nextAction: response.nextAction ?? 'Return to today',
        );
        _completionVisible = false;
      });
      return;
    }

    if (response.currentQuestion != null && _session != null) {
      setState(() {
        _session = StartSessionResponse(
          session: _session!.session,
          currentQuestion: response.currentQuestion!,
          progress: response.progress,
          answeredQuestions: _mergeStableAnsweredFeed(
            _session!.answeredQuestions,
            response.answeredQuestions,
          ),
          masteredCount: _session!.masteredCount,
        );
        _latestResponse = null;
        _selectedChoice = null;
      });
      _restartResponseTimer();
      _focusAnswerInputForCurrentQuestion();
    }
  }

  // ignore: unused_element
  Future<void> _skip() async {
    _answerController.clear();
    _selectedChoice = null;
    await _submit(allowEmpty: true);
  }

  Future<void> _maybeShowHintPrompt(SubmitAnswerResponse response) async {
    final prompt = response.hintPrompt;
    if (prompt == null || _showingHintPrompt || !mounted) return;
    _showingHintPrompt = true;
    try {
      final saved = await _showHintEditor(
        entryId: prompt.entryId,
        word: prompt.word,
        suggestions: prompt.suggestions,
        title: 'Add hint',
      );
      if (saved != null && mounted) {
        _applySavedHint(saved);
      }
    } finally {
      _showingHintPrompt = false;
    }
  }

  void _applySavedHint(WordHintState hint) {
    final session = _session;
    if (session == null) return;
    final entrySourceId = '${hint.entryId}';
    setState(() {
      _session = StartSessionResponse(
        session: session.session,
        currentQuestion: _questionWithSavedHint(
          session.currentQuestion,
          entrySourceId,
          hint,
        ),
        progress: session.progress,
        answeredQuestions: session.answeredQuestions
            .map(
              (answered) => AnsweredStudyQuestion(
                question: _questionWithSavedHint(
                  answered.question,
                  entrySourceId,
                  hint,
                ),
                result: answered.result,
              ),
            )
            .toList(growable: false),
        masteredCount: session.masteredCount,
      );
    });
  }

  StudyQuestion _questionWithSavedHint(
    StudyQuestion question,
    String entrySourceId,
    WordHintState hint,
  ) {
    if (question.entrySourceId != entrySourceId) return question;
    return question.copyWith(userHint: hint.userHint, hasHint: hint.hasHint);
  }

  Future<WordHintState?> _showHintEditor({
    required int entryId,
    required String word,
    required List<WordHintSuggestion> suggestions,
    required String title,
    String? initialText,
  }) async {
    final controller = TextEditingController(text: initialText ?? '');
    var selectedSource = 'user';
    var saving = false;
    final result = await showDialog<WordHintState?>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) {
          final mediaQuery = MediaQuery.of(context);
          final dialogHeight = math.max(
            280.0,
            mediaQuery.size.height - mediaQuery.viewInsets.bottom - 48,
          );
          final contentHeight = math.max(96.0, dialogHeight - 196);
          return AlertDialog(
            insetPadding: const EdgeInsets.symmetric(
              horizontal: 24,
              vertical: 24,
            ),
            constraints: BoxConstraints(maxWidth: 480, maxHeight: dialogHeight),
            title: Text(title),
            content: ConstrainedBox(
              constraints: BoxConstraints(maxHeight: contentHeight),
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(word, style: Theme.of(context).textTheme.titleMedium),
                    const SizedBox(height: 12),
                    if (suggestions.isNotEmpty) ...[
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: suggestions
                            .map(
                              (suggestion) => ActionChip(
                                avatar: const Icon(
                                  Icons.auto_awesome,
                                  size: 16,
                                ),
                                label: Text(suggestion.label),
                                onPressed: () {
                                  controller.text = suggestion.text;
                                  selectedSource = 'aiSuggestion';
                                  setDialogState(() {});
                                },
                              ),
                            )
                            .toList(growable: false),
                      ),
                      const SizedBox(height: 12),
                    ],
                    TextField(
                      controller: controller,
                      autofocus: false,
                      minLines: 2,
                      maxLines: 3,
                      decoration: const InputDecoration(
                        labelText: 'Hint',
                        hintText: 'Write a memory hint for yourself.',
                        border: OutlineInputBorder(),
                      ),
                    ),
                  ],
                ),
              ),
            ),
            actions: [
              TextButton(
                onPressed: () {
                  controller.clear();
                  selectedSource = 'user';
                  setDialogState(() {});
                },
                child: const Text('\u6e05\u7a7a'),
              ),
              TextButton(
                onPressed: () => Navigator.of(context).pop(null),
                child: const Text('\u7a0d\u540e\u518d\u8bf4'),
              ),
              FilledButton(
                onPressed: saving
                    ? null
                    : () async {
                        final text = controller.text.trim();
                        setDialogState(() {
                          saving = true;
                        });
                        try {
                          final saved = await widget.sdk.wrongWords
                              .saveWordHint(
                                entryId: entryId,
                                hintText: text,
                                source: selectedSource,
                              );
                          if (context.mounted) Navigator.of(context).pop(saved);
                        } finally {
                          if (context.mounted) {
                            setDialogState(() {
                              saving = false;
                            });
                          }
                        }
                      },
                child: Text(saving ? '保存中...' : '保存'),
              ),
            ],
          );
        },
      ),
    );
    controller.dispose();
    return result;
  }

  void _settleAnswerInputBeforeSubmit({StudyQuestion? question}) {
    if (!_shouldSettleAnswerInputBeforeSubmit(
      hasFocus: _answerFocusNode.hasFocus,
      isChoiceType: (question ?? _currentQuestion)?.isChoiceType ?? false,
    )) {
      return;
    }
    _hideAnswerKeyboard();
  }

  void _hideAnswerKeyboard() {
    _answerFocusNode.unfocus(disposition: UnfocusDisposition.scope);
    final primaryFocus = FocusManager.instance.primaryFocus;
    if (primaryFocus != null && primaryFocus != _answerFocusNode) {
      primaryFocus.unfocus(disposition: UnfocusDisposition.scope);
    }
    SystemChannels.textInput.invokeMethod<void>('TextInput.hide').ignore();
  }

  void _focusAnswerInputForCurrentQuestion() {
    final question = _currentQuestion;
    if (question == null || question.isChoiceType) {
      _settleAnswerInputBeforeSubmit();
      return;
    }
    if (!_shouldAutoOpenKeyboardForStudyInput()) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _submitting) return;
      final current = _currentQuestion;
      if (current == null || current.isChoiceType) return;
      if (!_answerFocusNode.hasFocus) {
        _answerFocusNode.requestFocus();
      }
      SystemChannels.textInput.invokeMethod<void>('TextInput.show').ignore();
    });
  }

  void _advanceToPendingQuestion() {
    final pending = _pendingNextQuestion;
    final pendingProgress = _pendingNextProgress;
    final session = _session;
    if (pending == null || session == null || _submitting) return;
    _settleAnswerInputBeforeSubmit(question: _lastSubmittedQuestion);
    _answerController.clear();
    setState(() {
      _pendingNextQuestion = null;
      _pendingNextProgress = null;
      _pendingNextVisible = false;
      _pendingNextTransitioning = false;
      _selectedChoice = null;
      _session = StartSessionResponse(
        session: session.session,
        currentQuestion: pending,
        progress: pendingProgress ?? session.progress,
        answeredQuestions: session.answeredQuestions,
        masteredCount: session.masteredCount,
      );
    });
    _restartResponseTimer();
    _focusAnswerInputForCurrentQuestion();
  }

  void _revealPendingNextQuestion() {
    if (_pendingNextQuestion == null ||
        _session == null ||
        _submitting ||
        _pendingNextTransitioning) {
      return;
    }
    _settleAnswerInputBeforeSubmit(question: _lastSubmittedQuestion);
    setState(() {
      _pendingNextVisible = true;
      _pendingNextTransitioning = true;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_pageController.hasClients) return;
      final pendingId = _pendingNextQuestion?.questionId;
      if (pendingId == null) return;
      final feedItems = _feedItems;
      final target = feedItems.indexWhere(
        (item) => item.question?.questionId == pendingId,
      );
      if (target < 0) {
        setState(() {
          _pendingNextVisible = false;
          _pendingNextTransitioning = false;
        });
        return;
      }
      setState(() {
        _visiblePageIndex = target;
      });
      _pageController
          .animateToPage(
            target,
            duration: const Duration(milliseconds: 220),
            curve: Curves.easeOutCubic,
          )
          .whenComplete(() {
            if (!mounted) return;
            if (_pendingNextQuestion?.questionId != pendingId) return;
            _advanceToPendingQuestion();
          });
    });
  }

  void _advanceToCompletionSummary() {
    if (_completion == null || _session == null) return;
    setState(() {
      _completionVisible = true;
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_pageController.hasClients) return;
      final feedItems = _feedItems;
      final target = feedItems.indexWhere((item) => item.isCompletion);
      if (target < 0) return;
      if (mounted) {
        setState(() {
          _visiblePageIndex = target;
        });
      }
      _pageController.animateToPage(
        target,
        duration: const Duration(milliseconds: 220),
        curve: Curves.easeOutCubic,
      );
    });
  }

  Future<void> _complete({bool closeAfter = false}) async {
    final session = _session;
    if (session == null) return;

    setState(() {
      _error = null;
    });
    try {
      final result = await widget.sdk.study.completeSession(
        session.session.sessionId,
      );
      setState(() {
        _completion = result;
        _completionVisible = false;
      });
      if (closeAfter && mounted) {
        if (widget.onCloseToToday != null) {
          widget.onCloseToToday!.call();
        } else {
          Navigator.of(context).pop(true);
        }
      }
    } catch (error) {
      setState(() {
        _error = error.toString();
      });
    }
  }

  void _closeToToday() {
    if (!mounted) return;
    if (widget.onCloseToToday != null) {
      widget.onCloseToToday!.call();
    } else {
      Navigator.of(context).pop(true);
    }
  }

  Future<void> _cancel() async {
    final session = _session;
    if (session == null) return;
    try {
      await widget.sdk.study.cancelSession(session.session.sessionId);
      if (mounted) {
        if (widget.onCloseToToday != null) {
          widget.onCloseToToday!.call();
        } else {
          Navigator.of(context).pop(true);
        }
      }
    } catch (error) {
      setState(() {
        _error = error.toString();
      });
    }
  }

  Future<void> _returnToTodayKeepingProgress() async {
    _closeToToday();
  }

  Future<void> _showExitOptions() async {
    final action = await showDialog<String>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Exit study?'),
        content: const Text('Your current progress will be kept.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop('continue'),
            child: const Text('\u7ee7\u7eed\u5b66\u4e60'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop('today'),
            child: const Text('\u8fd4\u56de\u4eca\u65e5'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop('end'),
            child: const Text('\u7ed3\u675f\u672c\u8f6e'),
          ),
        ],
      ),
    );

    switch (action) {
      case 'today':
        await _returnToTodayKeepingProgress();
        return;
      case 'end':
        await _cancel();
        return;
      default:
        return;
    }
  }

  Future<void> _startNextRound() async {
    final nextMode = _nextModeFrom(widget.mode);
    if (nextMode == null) {
      if (mounted) {
        if (widget.onCloseToToday != null) {
          widget.onCloseToToday!.call();
        } else {
          Navigator.of(context).pop(true);
        }
      }
      return;
    }
    if (_completion == null) {
      await _complete();
    }
    if (!mounted) return;
    if (widget.onOpenStudyMode != null) {
      await widget.onOpenStudyMode!.call(nextMode);
      return;
    }
    await Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (_) => StudyScreen(sdk: widget.sdk, mode: nextMode),
      ),
    );
  }

  void _restartResponseTimer() {
    _responseStopwatch
      ..reset()
      ..start();
  }

  int get _elapsedResponseMs {
    if (!_responseStopwatch.isRunning) {
      _responseStopwatch.start();
    }
    return _responseStopwatch.elapsedMilliseconds;
  }

  StudyQuestion? get _currentQuestion => _session?.currentQuestion;

  List<_StudyFeedItem> get _feedItems {
    final session = _session;
    if (session == null) return const [];
    var answeredQuestions = session.answeredQuestions;
    final lastQuestion = _lastSubmittedQuestion;
    final lastResult = _lastSubmittedResult;
    if (lastQuestion != null && lastResult != null) {
      answeredQuestions = mergeLatestAnsweredQuestionForTest(
        answeredQuestions,
        lastQuestion,
        lastResult,
      );
    }
    final answered = answeredQuestions
        .map((answered) {
          final responseOverride =
              lastQuestion != null &&
                  answered.question.questionId == lastQuestion.questionId
              ? _lastSubmittedResponse
              : null;
          return _StudyFeedItem.answered(
            answered,
            responseOverride: responseOverride,
          );
        })
        .toList(growable: true);
    final current = session.currentQuestion;
    final currentAlreadyAnswered = answered.any(
      (item) => item.question?.questionId == current.questionId,
    );
    if (!currentAlreadyAnswered && _completion == null) {
      answered.add(_StudyFeedItem.unanswered(current));
    }
    final pending = _pendingNextQuestion;
    if (_shouldInsertPendingNextPage(
      hasPendingNext: pending != null,
      pendingNextVisible: _pendingNextVisible,
    )) {
      final pendingAlreadyAnswered = answered.any(
        (item) => item.question?.questionId == pending!.questionId,
      );
      if (!pendingAlreadyAnswered) {
        answered.add(_StudyFeedItem.unanswered(pending!));
      }
    }
    final completion = _completion;
    if (_shouldInsertCompletionPage(
      hasCompletion: completion != null,
      completionVisible: _completionVisible,
    )) {
      answered.add(
        _StudyFeedItem.completion(
          completion!,
          answeredQuestions,
          newlyMasteredQuestions: _newlyMasteredQuestions,
          nextMode: _nextModeFrom(widget.mode),
        ),
      );
    }
    return answered;
  }

  void _jumpToCurrentFeedPage() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_shouldAutoJumpToCurrentQuestion(
        pendingNextQuestion: _pendingNextQuestion,
        pendingNextVisible: _pendingNextVisible,
      )) {
        return;
      }
      if (!_pageController.hasClients) return;
      final feedItems = _feedItems;
      final currentQuestionId = _currentQuestion?.questionId;
      final currentIndex = currentQuestionId == null
          ? -1
          : feedItems.indexWhere(
              (item) => item.question?.questionId == currentQuestionId,
            );
      final target =
          (currentIndex >= 0
                  ? currentIndex
                  : (feedItems.length - 1).clamp(0, feedItems.length))
              .toInt();
      if (target <= 0) return;
      setState(() {
        _visiblePageIndex = target;
      });
      _pageController.jumpToPage(target);
    });
  }

  @override
  Widget build(BuildContext context) => _buildFeedScaffold(context);

  Widget _buildFeedScaffold(BuildContext context) {
    final currentQuestion = _currentQuestion;
    final feedItems = _feedItems;
    final showQuestion =
        !_loading &&
        _error == null &&
        currentQuestion != null &&
        feedItems.isNotEmpty;

    return Scaffold(
      resizeToAvoidBottomInset: false,
      extendBodyBehindAppBar: showQuestion,
      body: _loading
          ? const CrocodileLoadingAnimation(label: 'Loading...')
          : _error != null
          ? _StudyMessage(
              title: 'Study error',
              message: _error!,
              actionLabel: 'Retry',
              onAction: _start,
            )
          : _completion != null && feedItems.isEmpty
          ? _StudyCompletion(
              completion: _completion!,
              newlyMasteredQuestions: _newlyMasteredQuestions,
              onRestart: _start,
              onClose: _closeToToday,
              nextMode: _nextModeFrom(widget.mode),
              onContinueNextRound: () => _startNextRound(),
            )
          : !showQuestion
          ? _StudyMessage(
              title: 'No question',
              message: 'No study question is available.',
              actionLabel: 'Retry',
              onAction: _start,
            )
          : Stack(
              children: [
                PageView.builder(
                  controller: _pageController,
                  scrollDirection: Axis.vertical,
                  itemCount: feedItems.length,
                  onPageChanged: (index) {
                    setState(() {
                      _visiblePageIndex = index;
                    });
                    final item = feedItems[index];
                    final itemQuestion = item.question;
                    if (!item.isAnswered &&
                        itemQuestion != null &&
                        itemQuestion.questionId ==
                            _currentQuestion?.questionId) {
                      _restartResponseTimer();
                      _focusAnswerInputForCurrentQuestion();
                    } else {
                      _settleAnswerInputBeforeSubmit();
                    }
                  },
                  itemBuilder: (context, index) {
                    final item = feedItems[index];
                    if (item.isCompletion) {
                      return _StudyCompletionFeedPage(
                        item: item,
                        onRestart: _start,
                        onClose: _closeToToday,
                        onContinueNextRound: _startNextRound,
                      );
                    }
                    final isActiveQuestion =
                        item.question?.questionId ==
                        _currentQuestion?.questionId;
                    return _StudyFeedPage(
                      key: ValueKey(
                        'study-question-${item.question!.questionId}',
                      ),
                      item: item,
                      examPracticeClient: widget.sdk.examPractice,
                      mode: widget.mode,
                      hasPendingNext:
                          _pendingNextQuestion != null &&
                          item.question?.questionId ==
                              _lastSubmittedQuestion?.questionId,
                      showCompletionHint:
                          _completion != null &&
                          !_completionVisible &&
                          item.question?.questionId ==
                              _lastSubmittedQuestion?.questionId,
                      progress: _studyFeedProgress(
                        visibleQuestion: item.question,
                        visibleIndex: index + 1,
                        itemCount: feedItems.length,
                        sessionTotal: _session?.progress.total,
                      ),
                      answerController: _answerController,
                      answerFocusNode: _answerFocusNode,
                      selectedChoice: item.isAnswered || !isActiveQuestion
                          ? null
                          : _selectedChoice,
                      submitting: _submitting,
                      onChoiceSelected: (choice) {
                        if (item.isAnswered || !isActiveQuestion) return;
                        setState(() {
                          _selectedChoice = choice;
                        });
                      },
                      onSubmit: () {
                        if (item.isAnswered || !isActiveQuestion) return;
                        final question = item.question;
                        if (question == null) return;
                        _submit(questionOverride: question);
                      },
                      onSubmitChoice: (choice) {
                        if (item.isAnswered ||
                            !isActiveQuestion ||
                            _submitting) {
                          return;
                        }
                        final question = item.question;
                        if (question == null) return;
                        _submit(
                          questionOverride: question,
                          explicitResponse: choice,
                        );
                      },
                      onHintPressed: questionHasHintForAction(item.question)
                          ? () {
                              final question = item.question;
                              if (question == null) return;
                              if (!item.isAnswered && isActiveQuestion) {
                                _recordStudyHintUse(
                                  mode: widget.mode,
                                  questionId: question.questionId,
                                  clickedQuestionIds: _hintUsedQuestionIds,
                                );
                              }
                              _editHintForQuestion(question);
                            }
                          : null,
                      onComments: () {
                        final question = item.question;
                        if (question == null) return;
                        _showWordComments(question);
                      },
                      onRevealAnswer: () {
                        if (item.isAnswered || !isActiveQuestion) return;
                        final question = item.question;
                        if (question == null) return;
                        _revealCurrentAnswer(question);
                      },
                      onDispute: _canDisputeFeedItem(item)
                          ? () => _acceptDispute(item)
                          : null,
                      onMastered:
                          _canMarkStudyItemMastered(
                            submitting: _submitting,
                            isAnswered: item.isAnswered,
                            isActiveQuestion: isActiveQuestion,
                            isMastered: _masteredEntrySourceIds.contains(
                              item.question?.entrySourceId,
                            ),
                          )
                          ? () {
                              final question = item.question;
                              if (question == null) return;
                              _markEntryMastered(
                                question,
                                wasAnswered: item.isAnswered,
                              );
                            }
                          : null,
                      onPendingNextSwipe: _revealPendingNextQuestion,
                      onCompletionSwipe: _advanceToCompletionSummary,
                      isMastered: _masteredEntrySourceIds.contains(
                        item.question?.entrySourceId,
                      ),
                    );
                  },
                ),
                Positioned(
                  top: MediaQuery.paddingOf(context).top + 8,
                  left: 12,
                  child: Row(
                    children: [
                      _RoundActionButton(
                        tooltip: 'Home',
                        icon: Icons.home_outlined,
                        onPressed: _returnToTodayKeepingProgress,
                      ),
                      const SizedBox(width: 10),
                      _FeedProgressPill(
                        progress: _feedProgressForVisiblePage(feedItems),
                      ),
                      const SizedBox(width: 8),
                      _MasteredCountPill(count: _masteredCount),
                    ],
                  ),
                ),
                Positioned(
                  top: MediaQuery.paddingOf(context).top + 8,
                  right: 12,
                  child: _RoundActionButton(
                    tooltip: 'Exit',
                    icon: Icons.close,
                    onPressed: _showExitOptions,
                  ),
                ),
              ],
            ),
    );
  }

  SessionProgress _feedProgressForVisiblePage(List<_StudyFeedItem> feedItems) {
    final sessionTotal = _session?.progress.total;
    final visibleIndex = ((_visiblePageIndex + 1).clamp(
      1,
      feedItems.length,
    )).toInt();
    final visibleQuestion =
        _visiblePageIndex >= 0 && _visiblePageIndex < feedItems.length
        ? feedItems[_visiblePageIndex].question
        : null;
    final visibleCompletion =
        _visiblePageIndex >= 0 && _visiblePageIndex < feedItems.length
        ? feedItems[_visiblePageIndex].completion
        : null;
    if (visibleCompletion != null) {
      final total = visibleCompletion.summary.totalQuestions;
      return SessionProgress(current: total, total: total);
    }
    return _studyFeedProgress(
      visibleQuestion: visibleQuestion,
      fallbackQuestion: _lastSubmittedQuestion ?? _currentQuestion,
      visibleIndex: visibleIndex,
      itemCount: feedItems.length,
      sessionTotal: sessionTotal,
    );
  }

  bool _canDisputeFeedItem(_StudyFeedItem item) {
    final question = item.question;
    final result = item.result;
    final response = item.responseOverride ?? result?.userResponse ?? '';
    if (question == null || result == null || !item.isAnswered) return false;
    return _canDisputeAnsweredQuestion(question, result, response);
  }

  Future<void> _editHintForQuestion(StudyQuestion question) async {
    final entryId = question.entryId ?? int.tryParse(question.entrySourceId);
    if (questionHasHintForAction(question)) {
      await _showSavedHintPreview(question, entryId: entryId);
      return;
    }
    if (entryId == null || entryId <= 0) return;
    await _openHintEditor(question, entryId);
  }

  Future<void> _openHintEditor(StudyQuestion question, int entryId) async {
    final saved = await _showHintEditor(
      entryId: entryId,
      word: question.word,
      suggestions: question.hintSuggestions,
      title: question.hasHint ? 'Edit hint' : 'Add hint',
      initialText: question.userHint,
    );
    if (saved != null && mounted) {
      _applySavedHint(saved);
    }
  }

  Future<void> _showSavedHintPreview(
    StudyQuestion question, {
    required int? entryId,
  }) async {
    final hint = question.userHint?.trim() ?? '';
    if (hint.isEmpty) return;
    final edit = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(question.word),
        content: Text(hint),
        actions: [
          if (entryId != null && entryId > 0)
            TextButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('修改提示词'),
            ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('关闭'),
          ),
        ],
      ),
    );
    if (edit == true && entryId != null && entryId > 0 && mounted) {
      await _openHintEditor(question, entryId);
    }
  }

  Future<void> _showWordComments(StudyQuestion question) async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (context) =>
          _WordCommentsSheet(service: _commentService, question: question),
    );
  }
}

class _WordCommentsSheet extends StatefulWidget {
  const _WordCommentsSheet({required this.service, required this.question});

  final WordCommentService service;
  final StudyQuestion question;

  @override
  State<_WordCommentsSheet> createState() => _WordCommentsSheetState();
}

class _WordCommentsSheetState extends State<_WordCommentsSheet> {
  final TextEditingController _controller = TextEditingController();
  late Future<List<WordComment>> _commentsFuture;
  int _commentCount = 0;
  bool _sending = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _commentsFuture = _load();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<List<WordComment>> _load() async {
    final comments = await widget.service.fetchComments(
      entrySourceId: widget.question.entrySourceId,
    );
    if (mounted) {
      setState(() {
        _commentCount = comments.length;
      });
    }
    return comments;
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty || _sending) return;
    setState(() {
      _sending = true;
      _error = null;
    });
    try {
      await widget.service.addComment(
        entrySourceId: widget.question.entrySourceId,
        word: widget.question.word,
        body: text,
      );
      _controller.clear();
      setState(() {
        _commentsFuture = _load();
      });
    } catch (error) {
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _sending = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.viewInsetsOf(context).bottom;
    return FractionallySizedBox(
      heightFactor: 0.68,
      child: DecoratedBox(
        decoration: const BoxDecoration(
          color: Colors.white,
          borderRadius: BorderRadius.vertical(top: Radius.circular(18)),
        ),
        child: Padding(
          padding: EdgeInsets.only(bottom: bottomInset),
          child: Column(
            children: [
              _WordCommentsHeader(
                word: widget.question.word,
                count: _commentCount,
                onClose: () => Navigator.of(context).pop(),
              ),
              Expanded(
                child: FutureBuilder<List<WordComment>>(
                  future: _commentsFuture,
                  builder: (context, snapshot) {
                    if (snapshot.connectionState == ConnectionState.waiting) {
                      return const Center(child: CircularProgressIndicator());
                    }
                    final comments = snapshot.data ?? const <WordComment>[];
                    if (_error != null) {
                      return _WordCommentsMessage(message: _error!);
                    }
                    if (comments.isEmpty) {
                      return const _WordCommentsMessage(
                        message: '\u671f\u5f85\u4f60\u7684\u8bc4\u8bba',
                      );
                    }
                    return ListView.separated(
                      padding: const EdgeInsets.fromLTRB(20, 14, 20, 18),
                      itemCount: comments.length,
                      separatorBuilder: (context, index) =>
                          const SizedBox(height: 18),
                      itemBuilder: (context, index) =>
                          _WordCommentTile(comment: comments[index]),
                    );
                  },
                ),
              ),
              _WordCommentComposer(
                controller: _controller,
                sending: _sending,
                onSend: _send,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _WordCommentsHeader extends StatelessWidget {
  const _WordCommentsHeader({
    required this.word,
    required this.count,
    required this.onClose,
  });

  final String word;
  final int count;
  final VoidCallback onClose;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        border: Border(bottom: BorderSide(color: Color(0xFFEFEFEF))),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 14, 12, 0),
        child: Column(
          children: [
            Row(
              children: [
                Expanded(
                  child: RichText(
                    text: TextSpan(
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        color: const Color(0xFF555555),
                      ),
                      children: [
                        const TextSpan(
                          text: '\u5927\u5bb6\u90fd\u5728\u641c\uff1a ',
                        ),
                        TextSpan(
                          text: word,
                          style: const TextStyle(color: Color(0xFF1F5E8C)),
                        ),
                      ],
                    ),
                  ),
                ),
                IconButton(
                  tooltip: 'Close',
                  icon: const Icon(Icons.close),
                  onPressed: onClose,
                ),
              ],
            ),
            Row(
              children: [
                _CommentTab(label: '\u8bc4\u8bba $count', active: true),
                const SizedBox(width: 28),
                _CommentTab(label: 'AI \u89e3\u6790', active: false),
                const Spacer(),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _CommentTab extends StatelessWidget {
  const _CommentTab({required this.label, required this.active});

  final String label;
  final bool active;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Text(
          label,
          style: Theme.of(context).textTheme.titleMedium?.copyWith(
            color: active ? Colors.black : Colors.black38,
            fontWeight: active ? FontWeight.w800 : FontWeight.w500,
          ),
        ),
        const SizedBox(height: 10),
        SizedBox(
          width: 56,
          height: 3,
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: active ? Colors.black : Colors.transparent,
              borderRadius: BorderRadius.circular(999),
            ),
          ),
        ),
      ],
    );
  }
}

class _WordCommentTile extends StatelessWidget {
  const _WordCommentTile({required this.comment});

  final WordComment comment;

  @override
  Widget build(BuildContext context) {
    final author = comment.authorName ?? 'Vico \u7528\u6237';
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        CircleAvatar(
          radius: 16,
          backgroundColor: const Color(0xFFE8ECEF),
          child: Text(
            author.characters.firstOrNull ?? 'V',
            style: const TextStyle(color: Color(0xFF666666)),
          ),
        ),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                author,
                style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: Colors.black38,
                  fontWeight: FontWeight.w600,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                comment.body,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: Colors.black,
                  height: 1.35,
                  fontWeight: FontWeight.w500,
                ),
              ),
              const SizedBox(height: 6),
              Text(
                '${_relativeCommentTime(comment.createdAt)} \u00b7 \u56de\u590d',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Colors.black38,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ],
          ),
        ),
        const SizedBox(width: 10),
        IconButton(
          tooltip: 'Like',
          onPressed: () {},
          icon: const Icon(Icons.favorite_border),
          color: Colors.black45,
        ),
      ],
    );
  }
}

class _WordCommentsMessage extends StatelessWidget {
  const _WordCommentsMessage({required this.message});

  final String message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Text(
        message,
        style: Theme.of(context).textTheme.titleMedium?.copyWith(
          color: Colors.black38,
          fontWeight: FontWeight.w600,
        ),
      ),
    );
  }
}

class _WordCommentComposer extends StatelessWidget {
  const _WordCommentComposer({
    required this.controller,
    required this.sending,
    required this.onSend,
  });

  final TextEditingController controller;
  final bool sending;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: Colors.white,
        border: Border(top: BorderSide(color: Color(0xFFEFEFEF))),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
          child: Row(
            children: [
              Expanded(
                child: TextField(
                  controller: controller,
                  enabled: !sending,
                  minLines: 1,
                  maxLines: 4,
                  textInputAction: TextInputAction.send,
                  onSubmitted: (_) => onSend(),
                  decoration: InputDecoration(
                    hintText: '\u671f\u5f85\u4f60\u7684\u8bc4\u8bba',
                    filled: true,
                    fillColor: const Color(0xFFF2F3F5),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(999),
                      borderSide: BorderSide.none,
                    ),
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 18,
                      vertical: 12,
                    ),
                  ),
                ),
              ),
              IconButton(
                tooltip: 'Image',
                onPressed: null,
                icon: const Icon(Icons.image_outlined),
              ),
              IconButton(
                tooltip: 'Mention',
                onPressed: null,
                icon: const Icon(Icons.alternate_email),
              ),
              IconButton(
                tooltip: 'Send',
                onPressed: sending ? null : onSend,
                icon: sending
                    ? const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.emoji_emotions_outlined),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

String _relativeCommentTime(DateTime? value) {
  if (value == null) return '\u521a\u521a';
  final diff = DateTime.now().difference(value.toLocal());
  if (diff.inMinutes < 1) return '\u521a\u521a';
  if (diff.inHours < 1) return '${diff.inMinutes}\u5206\u949f\u524d';
  if (diff.inDays < 1) return '${diff.inHours}\u5c0f\u65f6\u524d';
  if (diff.inDays < 7) return '${diff.inDays}\u5929\u524d';
  return '${value.month}\u6708${value.day}\u65e5';
}

class _StudyFeedItem {
  const _StudyFeedItem._({
    this.question,
    this.result,
    this.responseOverride,
    this.completion,
    this.answeredQuestions = const [],
    this.newlyMasteredQuestions = const [],
    this.nextMode,
  });

  factory _StudyFeedItem.unanswered(StudyQuestion question) =>
      _StudyFeedItem._(question: question);

  factory _StudyFeedItem.answered(
    AnsweredStudyQuestion answered, {
    String? responseOverride,
  }) => _StudyFeedItem._(
    question: answered.question,
    result: answered.result,
    responseOverride: responseOverride,
  );

  factory _StudyFeedItem.completion(
    CompleteSessionResponse completion,
    List<AnsweredStudyQuestion> answeredQuestions, {
    List<StudyQuestion> newlyMasteredQuestions = const [],
    String? nextMode,
  }) => _StudyFeedItem._(
    completion: completion,
    answeredQuestions: answeredQuestions,
    newlyMasteredQuestions: newlyMasteredQuestions,
    nextMode: nextMode,
  );

  final StudyQuestion? question;
  final StudyResult? result;
  final String? responseOverride;
  final CompleteSessionResponse? completion;
  final List<AnsweredStudyQuestion> answeredQuestions;
  final List<StudyQuestion> newlyMasteredQuestions;
  final String? nextMode;

  bool get isAnswered => result != null;
  bool get isCompletion => completion != null;
}

List<AnsweredStudyQuestion> mergeLatestAnsweredQuestionForTest(
  List<AnsweredStudyQuestion> answeredQuestions,
  StudyQuestion question,
  StudyResult result,
) {
  final merged = answeredQuestions.toList(growable: true);
  final existingIndex = merged.indexWhere(
    (answered) => answered.question.questionId == question.questionId,
  );
  final answered = AnsweredStudyQuestion(question: question, result: result);
  if (existingIndex >= 0) {
    merged[existingIndex] = answered;
  } else {
    merged.add(answered);
  }
  return merged;
}

StudyQuestion submittedQuestionWithFeedbackForTest(
  List<AnsweredStudyQuestion> answeredQuestions,
  StudyQuestion fallbackQuestion,
  StudyResult result,
) {
  for (final answered in answeredQuestions.reversed) {
    if (answered.question.questionId == result.questionId) {
      return answered.question;
    }
  }
  return fallbackQuestion;
}

List<StudyQuestion> newlyMasteredQuestionsForTest({
  required List<StudyQuestion> existing,
  required StudyQuestion question,
  required int masteredCountBefore,
  required int masteredCountAfter,
}) {
  if (masteredCountAfter <= masteredCountBefore ||
      existing.any((item) => item.entrySourceId == question.entrySourceId)) {
    return existing;
  }
  return [...existing, question];
}

List<AnsweredStudyQuestion> _mergeStableAnsweredFeed(
  List<AnsweredStudyQuestion> existing,
  List<AnsweredStudyQuestion> incoming,
) {
  final merged = existing.toList(growable: true);
  final indexByQuestionId = <String, int>{
    for (var index = 0; index < merged.length; index++)
      merged[index].question.questionId: index,
  };
  for (final answered in incoming) {
    final questionId = answered.question.questionId;
    final existingIndex = indexByQuestionId[questionId];
    if (existingIndex == null) {
      indexByQuestionId[questionId] = merged.length;
      merged.add(answered);
    } else {
      merged[existingIndex] = answered;
    }
  }
  return merged;
}

List<AnsweredStudyQuestion> mergeStableAnsweredFeedForTest(
  List<AnsweredStudyQuestion> existing,
  List<AnsweredStudyQuestion> incoming,
) => _mergeStableAnsweredFeed(existing, incoming);

class _StudyFeedPage extends StatelessWidget {
  const _StudyFeedPage({
    super.key,
    required this.item,
    required this.examPracticeClient,
    required this.mode,
    required this.progress,
    required this.hasPendingNext,
    required this.answerController,
    required this.answerFocusNode,
    required this.showCompletionHint,
    required this.selectedChoice,
    required this.submitting,
    required this.onChoiceSelected,
    required this.onSubmit,
    required this.onSubmitChoice,
    required this.onHintPressed,
    required this.onComments,
    required this.onRevealAnswer,
    required this.onDispute,
    required this.onMastered,
    required this.onPendingNextSwipe,
    required this.onCompletionSwipe,
    required this.isMastered,
  });

  final _StudyFeedItem item;
  final ExamPracticeClient examPracticeClient;
  final String mode;
  final SessionProgress progress;
  final bool hasPendingNext;
  final TextEditingController answerController;
  final FocusNode answerFocusNode;
  final bool showCompletionHint;
  final String? selectedChoice;
  final bool submitting;
  final ValueChanged<String> onChoiceSelected;
  final VoidCallback onSubmit;
  final ValueChanged<String> onSubmitChoice;
  final VoidCallback? onHintPressed;
  final VoidCallback onComments;
  final VoidCallback onRevealAnswer;
  final VoidCallback? onDispute;
  final VoidCallback? onMastered;
  final VoidCallback onPendingNextSwipe;
  final VoidCallback onCompletionSwipe;
  final bool isMastered;

  @override
  Widget build(BuildContext context) {
    final question = item.question;
    if (question == null) return const SizedBox.shrink();
    final result = item.result;
    final answered = item.isAnswered;
    return _AdvanceGestureRegion(
      enabled: hasPendingNext || showCompletionHint,
      hasPendingNext: hasPendingNext,
      onPendingNextSwipe: onPendingNextSwipe,
      onCompletionSwipe: onCompletionSwipe,
      child: ColoredBox(
        color: const Color(0xFFF9FAF7),
        child: SafeArea(
          top: false,
          child: Stack(
            children: [
              Positioned.fill(
                child: ScrollConfiguration(
                  behavior: ScrollConfiguration.of(
                    context,
                  ).copyWith(overscroll: false),
                  child: SingleChildScrollView(
                    physics: _studyQuestionScrollPhysics(
                      continuationEnabled: hasPendingNext || showCompletionHint,
                    ),
                    padding: EdgeInsets.fromLTRB(
                      16,
                      MediaQuery.paddingOf(context).top + 56,
                      16,
                      24,
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _FeedQuestionHeader(question: question),
                        const SizedBox(height: 12),
                        _QuestionComposer(
                          question: question,
                          examPracticeClient: examPracticeClient,
                          mode: mode,
                          answerController: answerController,
                          answerFocusNode: answerFocusNode,
                          selectedChoice: selectedChoice,
                          onChoiceSelected: onChoiceSelected,
                          onSubmit: () async => onSubmit(),
                          onSubmitChoice: onSubmitChoice,
                          submitting: submitting || answered,
                          answered: answered,
                          result: result,
                          responseOverride: item.responseOverride,
                        ),
                        if (answered && showCompletionHint) ...[
                          const SizedBox(height: 24),
                          const _CompletionSwipeHint(),
                        ],
                        if (answered && hasPendingNext) ...[
                          const SizedBox(height: 24),
                          const _NextQuestionSwipeHint(),
                        ],
                        const _KeyboardInsetSpacer(baseHeight: 24),
                      ],
                    ),
                  ),
                ),
              ),
              Positioned(
                right: 12,
                top: MediaQuery.sizeOf(context).height * 0.48,
                child: Column(
                  children: [
                    _RoundActionButton(
                      tooltip: 'Hint',
                      icon: Icons.lightbulb_outline,
                      onPressed: onHintPressed,
                      active: questionHasHintForAction(question),
                    ),
                    const SizedBox(height: 16),
                    _RoundActionButton(
                      tooltip: 'Comments',
                      icon: Icons.mode_comment_outlined,
                      onPressed: onComments,
                    ),
                    const SizedBox(height: 16),
                    _RoundActionButton(
                      tooltip: 'Show answer',
                      icon: Icons.visibility_outlined,
                      onPressed: answered || submitting ? null : onRevealAnswer,
                    ),
                    const SizedBox(height: 16),
                    _RoundActionButton(
                      tooltip: 'Dispute',
                      icon: Icons.gavel_outlined,
                      onPressed: onDispute,
                    ),
                    const SizedBox(height: 16),
                    _RoundActionButton(
                      tooltip: isMastered ? 'Mastered marked' : 'Mastered',
                      icon: _masteredActionIcon(isMastered),
                      onPressed: submitting || isMastered ? null : onMastered,
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AdvanceGestureRegion extends StatefulWidget {
  const _AdvanceGestureRegion({
    required this.enabled,
    required this.hasPendingNext,
    required this.onPendingNextSwipe,
    required this.onCompletionSwipe,
    required this.child,
  });

  final bool enabled;
  final bool hasPendingNext;
  final VoidCallback onPendingNextSwipe;
  final VoidCallback onCompletionSwipe;
  final Widget child;

  @override
  State<_AdvanceGestureRegion> createState() => _AdvanceGestureRegionState();
}

ScrollPhysics? _studyQuestionScrollPhysics({
  required bool continuationEnabled,
}) => continuationEnabled
    ? const AlwaysScrollableScrollPhysics(parent: ClampingScrollPhysics())
    : null;

class _AdvanceGestureRegionState extends State<_AdvanceGestureRegion> {
  bool _advanced = false;

  void _advance() {
    if (_advanced) return;
    _advanced = true;
    if (widget.hasPendingNext) {
      widget.onPendingNextSwipe();
    } else {
      widget.onCompletionSwipe();
    }
  }

  bool _handleScrollNotification(ScrollNotification notification) {
    if (notification.depth == 0 && notification is ScrollStartNotification) {
      _advanced = false;
    }
    if (notification.depth == 0 &&
        notification is OverscrollNotification &&
        notification.metrics.axis == Axis.vertical &&
        notification.metrics.extentAfter <= 0.5 &&
        notification.overscroll > 0) {
      _advance();
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.enabled) return widget.child;
    return NotificationListener<ScrollNotification>(
      onNotification: _handleScrollNotification,
      child: widget.child,
    );
  }
}

class _CompletionSwipeHint extends StatelessWidget {
  const _CompletionSwipeHint();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Colors.black.withValues(alpha: 0.06),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 9),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.keyboard_arrow_down,
                size: 22,
                color: Colors.black54,
              ),
              const SizedBox(width: 4),
              Text(
                '\u4e0b\u6ed1\u67e5\u770b\u672c\u8f6e\u603b\u7ed3',
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: Colors.black54,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _NextQuestionSwipeHint extends StatelessWidget {
  const _NextQuestionSwipeHint();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Colors.black.withValues(alpha: 0.06),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 9),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.keyboard_arrow_down,
                size: 22,
                color: Colors.black54,
              ),
              const SizedBox(width: 4),
              Text(
                '\u4e0b\u6ed1\u7ee7\u7eed\u4e0b\u4e00\u9898',
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: Colors.black54,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _FeedProgressPill extends StatelessWidget {
  const _FeedProgressPill({required this.progress});

  final SessionProgress progress;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Colors.black.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
        child: Text(
          '${progress.current}/${progress.total}',
          style: Theme.of(context).textTheme.labelLarge?.copyWith(
            fontWeight: FontWeight.w700,
            color: Colors.black87,
          ),
        ),
      ),
    );
  }
}

class _MasteredCountPill extends StatelessWidget {
  const _MasteredCountPill({required this.count});

  final int count;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: '已掌握词数 $count',
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: const Color(0xFFE7F2E8),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.check_circle_outline,
                size: 16,
                color: Color(0xFF2E7D32),
              ),
              const SizedBox(width: 4),
              Text(
                '掌握 $count',
                maxLines: 1,
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: const Color(0xFF2E7D32),
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _FeedQuestionHeader extends StatelessWidget {
  const _FeedQuestionHeader({required this.question});

  final StudyQuestion question;

  @override
  Widget build(BuildContext context) {
    final hero = _studyHeroDisplay(question);
    final titleText = Text(
      hero.title,
      style: Theme.of(context).textTheme.displaySmall?.copyWith(
        fontWeight: FontWeight.w800,
        height: 1.06,
        color: Colors.black,
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _shouldHighlightStudyHero(question, hero.title)
            ? DecoratedBox(
                decoration: BoxDecoration(
                  color: _examMarkHighlightColor(question),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 4),
                  child: titleText,
                ),
              )
            : titleText,
        const SizedBox(height: 8),
        Wrap(
          spacing: 14,
          runSpacing: 4,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            if (hero.subtitle != null)
              Text(
                hero.subtitle!,
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: Colors.black54,
                  fontWeight: FontWeight.w600,
                ),
              ),
            Text(
              '错误 ${question.errorCount} 次',
              key: const ValueKey('study-question-error-count'),
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Colors.black45,
                fontWeight: FontWeight.w500,
              ),
            ),
            if ((question.phoneticUs ?? question.phoneticUk)
                    ?.trim()
                    .isNotEmpty ??
                false)
              Text(
                question.phoneticUs ?? question.phoneticUk ?? '',
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
              ),
          ],
        ),
      ],
    );
  }
}

class _RoundActionButton extends StatelessWidget {
  const _RoundActionButton({
    required this.tooltip,
    required this.icon,
    required this.onPressed,
    this.active = false,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final visualActive = roundActionButtonVisualActiveForTest(
      active: active,
      enabled: onPressed != null,
    );
    final enabledColor = visualActive
        ? theme.colorScheme.primary
        : Colors.black87;
    final backgroundColor = visualActive
        ? theme.colorScheme.primary.withValues(alpha: 0.12)
        : Colors.transparent;
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      color: enabledColor,
      disabledColor: Colors.black.withValues(alpha: 0.24),
      iconSize: 30,
      icon: Icon(icon),
      style: IconButton.styleFrom(
        backgroundColor: backgroundColor,
        disabledBackgroundColor: Colors.transparent,
        shape: const CircleBorder(),
        padding: const EdgeInsets.all(4),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
    );
  }
}

class _StudyCompletionFeedPage extends StatelessWidget {
  const _StudyCompletionFeedPage({
    required this.item,
    required this.onRestart,
    required this.onClose,
    required this.onContinueNextRound,
  });

  final _StudyFeedItem item;
  final Future<void> Function() onRestart;
  final VoidCallback onClose;
  final Future<void> Function() onContinueNextRound;

  @override
  Widget build(BuildContext context) {
    final completion = item.completion;
    if (completion == null) return const SizedBox.shrink();
    final summary = completion.summary;
    final wrongWords = _wrongWordSummaries(item.answeredQuestions);
    final bottomPadding = MediaQuery.paddingOf(context).bottom + 24;

    return ColoredBox(
      color: const Color(0xFFF9FAF7),
      child: SafeArea(
        top: false,
        child: ListView(
          padding: EdgeInsets.fromLTRB(
            24,
            MediaQuery.paddingOf(context).top + 78,
            24,
            bottomPadding,
          ),
          children: [
            Text(
              'Session summary',
              style: Theme.of(context).textTheme.displaySmall?.copyWith(
                fontWeight: FontWeight.w800,
                height: 1.05,
                color: Colors.black,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              _summarySubtitle(summary),
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: Colors.black54,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 28),
            _AccuracyPanel(summary: summary),
            const SizedBox(height: 18),
            _SummaryGrid(summary: summary),
            if (item.newlyMasteredQuestions.isNotEmpty) ...[
              const SizedBox(height: 20),
              _NewlyMasteredWordsPanel(questions: item.newlyMasteredQuestions),
            ],
            const SizedBox(height: 20),
            _WrongWordsPanel(wrongWords: wrongWords),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: onClose,
              child: const Text('\u8fd4\u56de\u4eca\u65e5'),
            ),
            const SizedBox(height: 10),
            OutlinedButton(
              onPressed: onRestart,
              child: const Text('\u91cd\u5b66\u672c\u6a21\u5f0f'),
            ),
            if (item.nextMode != null) ...[
              const SizedBox(height: 10),
              FilledButton.tonal(
                onPressed: onContinueNextRound,
                child: Text('\u7ee7\u7eed${_modeLabel(item.nextMode!)}'),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

String? _nextModeFrom(String mode) {
  return switch (mode) {
    'newWord' => 'review',
    'review' => 'mixedTest',
    'mixedTest' => 'wrongWordReinforcement',
    'wrongWordReinforcement' => 'highFrequency',
    'highFrequency' => 'rootAffix',
    _ => null,
  };
}

({String title, String? subtitle}) _studyHeroDisplay(StudyQuestion question) {
  if (question.questionType == 'cnToEnChoice') {
    return (title: question.prompt, subtitle: null);
  }
  if (question.questionType == 'wordSkeletonInput') {
    return (
      title: wordSkeletonDisplayForTest(question.prompt, ''),
      subtitle: question.partOfSpeech,
    );
  }
  return (title: question.word, subtitle: question.partOfSpeech);
}

({String title, String? subtitle}) studyHeroDisplayForTest(
  StudyQuestion question,
) => _studyHeroDisplay(question);

bool shouldHighlightStudyHeroForTest(
  StudyQuestion question,
  String heroTitle,
) => _shouldHighlightStudyHero(question, heroTitle);

bool _shouldHighlightStudyHero(StudyQuestion question, String heroTitle) {
  if (!question.examMarked) return false;
  final normalizedHero = heroTitle.trim().toLowerCase();
  final normalizedWord = question.word.trim().toLowerCase();
  if (normalizedHero.isEmpty || normalizedWord.isEmpty) return false;
  return normalizedHero == normalizedWord ||
      question.questionType == 'wordSkeletonInput';
}

Color _examMarkHighlightColor(StudyQuestion question) {
  return studyExampleMarkColorForTest(question.examMarkLevel ?? 'fuzzy');
}

// ignore: unused_element
class _SessionHero extends StatelessWidget {
  const _SessionHero({
    required this.question,
    required this.progress,
    required this.onEditHint,
  });

  final StudyQuestion question;
  final SessionProgress progress;
  final VoidCallback onEditHint;

  @override
  Widget build(BuildContext context) {
    final completion = progress.total == 0
        ? 0.0
        : progress.current / progress.total;
    final heroDisplay = _studyHeroDisplay(question);
    final compactCnToEnHero =
        question.questionType == 'cnToEnChoice' &&
        heroDisplay.title.characters.length > 12;
    final titleStyle =
        (compactCnToEnHero
                ? Theme.of(context).textTheme.headlineSmall
                : Theme.of(context).textTheme.headlineLarge)
            ?.copyWith(
              color: Colors.white,
              fontWeight: FontWeight.w700,
              height: compactCnToEnHero ? 1.12 : null,
            );
    final hasHintSurface = _shouldShowHeroHintChip(question);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      color: Theme.of(context).colorScheme.primary,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Expanded(
                  child: Text(
                    '${question.questionIndex}/${question.totalQuestions}',
                    style: Theme.of(
                      context,
                    ).textTheme.labelLarge?.copyWith(color: Colors.white70),
                  ),
                ),
                if (hasHintSurface)
                  _MaskedHintChip(
                    hint: question.userHint,
                    hasAiSuggestion: question.hintSuggestions.isNotEmpty,
                    onEdit: onEditHint,
                  ),
              ],
            ),
            const SizedBox(height: 6),
            Text(
              heroDisplay.title,
              maxLines: compactCnToEnHero ? 3 : null,
              overflow: compactCnToEnHero ? TextOverflow.ellipsis : null,
              style: titleStyle,
            ),
            if (heroDisplay.subtitle != null) ...[
              const SizedBox(height: 4),
              Text(
                heroDisplay.subtitle!,
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: Colors.white70),
              ),
            ],
            const SizedBox(height: 12),
            ClipRRect(
              borderRadius: BorderRadius.circular(999),
              child: LinearProgressIndicator(
                value: completion.clamp(0.0, 1.0),
                minHeight: 8,
                backgroundColor: Colors.white24,
                valueColor: const AlwaysStoppedAnimation<Color>(Colors.white),
              ),
            ),
            const SizedBox(height: 8),
            Text(
              '\u672c\u8f6e\u5b66\u4e60\u5df2\u63a8\u8fdb ${progress.current}/${progress.total}',
              style: Theme.of(
                context,
              ).textTheme.bodySmall?.copyWith(color: Colors.white70),
            ),
          ],
        ),
      ),
    );
  }
}

class _KeyboardInsetSpacer extends StatelessWidget {
  const _KeyboardInsetSpacer({required this.baseHeight});

  final double baseHeight;

  @override
  Widget build(BuildContext context) =>
      SizedBox(height: _keyboardInsetSpacerHeight(baseHeight: baseHeight));
}

class _MaskedHintChip extends StatefulWidget {
  const _MaskedHintChip({
    required this.hint,
    required this.hasAiSuggestion,
    required this.onEdit,
  });

  final String? hint;
  final bool hasAiSuggestion;
  final VoidCallback onEdit;

  @override
  State<_MaskedHintChip> createState() => _MaskedHintChipState();
}

class _MaskedHintChipState extends State<_MaskedHintChip> {
  bool _revealed = false;

  @override
  Widget build(BuildContext context) {
    final text = widget.hint?.trim();
    final hasHint = text != null && text.isNotEmpty;
    final label = hasHint
        ? (_revealed ? text : 'Hint ...')
        : (widget.hasAiSuggestion ? 'AI\u63d0\u793a' : '\u63d0\u793a');
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 190),
      child: Material(
        color: Colors.white.withValues(alpha: 0.16),
        borderRadius: BorderRadius.circular(999),
        child: InkWell(
          borderRadius: BorderRadius.circular(999),
          onTap: hasHint
              ? () {
                  setState(() {
                    _revealed = !_revealed;
                  });
                }
              : widget.onEdit,
          onLongPress: widget.onEdit,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  widget.hasAiSuggestion && !hasHint
                      ? Icons.auto_awesome
                      : Icons.lightbulb_outline,
                  size: 15,
                  color: Colors.white,
                ),
                const SizedBox(width: 5),
                Flexible(
                  child: Text(
                    label,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                      color: Colors.white,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _QuestionComposer extends StatelessWidget {
  const _QuestionComposer({
    required this.question,
    required this.examPracticeClient,
    required this.mode,
    required this.answerController,
    required this.answerFocusNode,
    required this.selectedChoice,
    required this.onChoiceSelected,
    required this.onSubmit,
    required this.onSubmitChoice,
    required this.submitting,
    required this.answered,
    this.result,
    this.responseOverride,
  });

  final StudyQuestion question;
  final ExamPracticeClient examPracticeClient;
  final String mode;
  final TextEditingController answerController;
  final FocusNode answerFocusNode;
  final String? selectedChoice;
  final ValueChanged<String> onChoiceSelected;
  final Future<void> Function() onSubmit;
  final ValueChanged<String> onSubmitChoice;
  final bool submitting;
  final bool answered;
  final StudyResult? result;
  final String? responseOverride;

  @override
  Widget build(BuildContext context) {
    final isChoice = question.isChoiceType;
    final isCnToEnChoice = question.questionType == 'cnToEnChoice';
    final isRootAffix =
        question.questionType == 'glossToRootInput' ||
        question.questionType == 'rootToGlossInput';
    final isWordSkeleton = question.questionType == 'wordSkeletonInput';
    final dedicatedExampleSentence = _dedicatedExampleSentence(question);
    final isExampleChoice = dedicatedExampleSentence != null;
    final showSupplementalExamExample = _shouldShowSupplementalExamExample(
      question: question,
      mode: mode,
      answered: answered,
    );
    final showDerivationalFamily = _shouldShowStudyDerivationalFamily(
      question: question,
      mode: mode,
      answered: answered,
    );
    final showSynonyms = _shouldShowStudySynonyms(
      question: question,
      mode: mode,
      answered: answered,
    );
    final displayPrompt =
        _shouldShowStudyPrompt(
          questionType: question.questionType,
          isRootAffix: isRootAffix,
          isExampleChoice: isExampleChoice,
          isCnToEnChoice: isCnToEnChoice,
          isWordSkeleton: isWordSkeleton,
        )
        ? question.prompt
        : null;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          _questionLabel(question.questionType),
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            fontWeight: FontWeight.w600,
            color: Colors.black54,
          ),
        ),
        if (displayPrompt != null) ...[
          const SizedBox(height: 4),
          Text(displayPrompt),
        ],
        const SizedBox(height: 8),
        if (isExampleChoice) ...[
          _HighlightedExampleSentence(
            sentence: dedicatedExampleSentence,
            word: question.word,
          ),
          if (question.exampleTranslation != null) ...[
            const SizedBox(height: 6),
            Text(
              question.exampleTranslation!,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
            ),
          ],
          const SizedBox(height: 10),
        ],
        if (isRootAffix &&
            (question.exampleSentence != null ||
                question.exampleTranslation != null)) ...[
          _RootAffixRelatedWords(question: question),
          const SizedBox(height: 10),
        ],
        if (isWordSkeleton) ...[
          ValueListenableBuilder<TextEditingValue>(
            valueListenable: answerController,
            builder: (context, value, _) {
              return _WordSkeletonPreview(
                prompt: question.prompt,
                answer: value.text,
              );
            },
          ),
          if ((question.exampleTranslation ?? '').trim().isNotEmpty) ...[
            const SizedBox(height: 6),
            Text(
              question.exampleTranslation!,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Colors.black54,
                fontWeight: FontWeight.w600,
              ),
            ),
          ],
          const SizedBox(height: 10),
        ],
        if (isChoice)
          ..._buildChoiceOptions(
            context,
            question,
            selectedChoice,
            result,
            responseOverride,
            onChoiceSelected,
            onSubmitChoice,
            submitting,
          ),
        if (!isChoice) ...[
          if (!answered)
            IgnorePointer(
              ignoring: submitting,
              child: TextField(
                controller: answerController,
                focusNode: answerFocusNode,
                readOnly: _inputAnswerFieldReadOnlyWhileSubmitting(),
                keyboardType: TextInputType.text,
                obscureText: false,
                enableIMEPersonalizedLearning: true,
                autofillHints: null,
                inputFormatters: isWordSkeleton
                    ? [FilteringTextInputFormatter.allow(RegExp(r'[A-Za-z]'))]
                    : null,
                enableSuggestions: true,
                autocorrect: false,
                onTapOutside: (_) {
                  answerFocusNode.unfocus(
                    disposition: UnfocusDisposition.scope,
                  );
                  SystemChannels.textInput
                      .invokeMethod<void>('TextInput.hide')
                      .ignore();
                },
                onSubmitted: (_) {
                  if (!submitting && answerController.text.trim().isNotEmpty) {
                    onSubmit();
                  }
                },
                decoration: const InputDecoration(
                  labelText: 'Answer',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                textInputAction: TextInputAction.done,
                maxLines: 1,
              ),
            ),
          if (answered && result != null)
            _buildInputFeedback(context, result!, question),
        ],
        if (showSupplementalExamExample) ...[
          const SizedBox(height: 16),
          StudyInteractiveExampleSentence(
            client: examPracticeClient,
            sentence: question.exampleSentence!,
            word: question.word,
            entrySourceId: question.entrySourceId,
            mode: mode,
          ),
          const SizedBox(height: 6),
          Text(
            question.exampleTranslation!,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: Colors.black54),
          ),
        ],
        if (showDerivationalFamily) ...[
          const SizedBox(height: 16),
          _StudyMeaningDrawer(
            title: '同源词组',
            drawerKey: const ValueKey('study-derivational-family-drawer'),
            child: _StudyDerivationalFamily(question: question),
          ),
        ],
        if (showSynonyms) ...[
          const SizedBox(height: 12),
          _StudyMeaningDrawer(
            title: '近义词',
            drawerKey: const ValueKey('study-synonym-drawer'),
            child: _StudySynonyms(question: question),
          ),
        ],
      ],
    );
  }
}

String? studyDedicatedExampleSentenceForTest(StudyQuestion question) =>
    _dedicatedExampleSentence(question);

bool shouldShowStudyPromptForTest(String questionType) =>
    _shouldShowStudyPrompt(
      questionType: questionType,
      isRootAffix:
          questionType == 'glossToRootInput' ||
          questionType == 'rootToGlossInput',
      isExampleChoice:
          questionType == 'exampleToCnChoice' ||
          questionType == 'exampleToCnChoiceNoTranslation',
      isCnToEnChoice: questionType == 'cnToEnChoice',
      isWordSkeleton: questionType == 'wordSkeletonInput',
    );

bool _shouldShowStudyPrompt({
  required String questionType,
  required bool isRootAffix,
  required bool isExampleChoice,
  required bool isCnToEnChoice,
  required bool isWordSkeleton,
}) =>
    !isRootAffix &&
    !isExampleChoice &&
    !isCnToEnChoice &&
    !isWordSkeleton &&
    questionType != 'enToCnInput';

String? _dedicatedExampleSentence(StudyQuestion question) {
  if (question.questionType != 'exampleToCnChoice' &&
      question.questionType != 'exampleToCnChoiceNoTranslation') {
    return null;
  }
  final sentence = (question.exampleSentence ?? '').trim();
  return sentence.isEmpty ? null : sentence;
}

bool shouldShowStudyDerivationalFamilyForTest({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) => _shouldShowStudyDerivationalFamily(
  question: question,
  mode: mode,
  answered: answered,
);

bool _shouldShowStudyDerivationalFamily({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) =>
    answered &&
    mode == 'highFrequency' &&
    question.derivationalFamily.isNotEmpty;

bool shouldShowStudySynonymsForTest({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) => _shouldShowStudySynonyms(
  question: question,
  mode: mode,
  answered: answered,
);

bool _shouldShowStudySynonyms({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) => answered && mode == 'highFrequency' && question.synonymGroups.isNotEmpty;

class _StudyMeaningDrawer extends StatelessWidget {
  const _StudyMeaningDrawer({
    required this.title,
    required this.drawerKey,
    required this.child,
  });

  final String title;
  final Key drawerKey;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      decoration: BoxDecoration(
        color: Colors.white,
        border: Border.all(color: const Color(0xFFE1E4DE)),
        borderRadius: BorderRadius.circular(6),
      ),
      child: InkWell(
        key: drawerKey,
        borderRadius: BorderRadius.circular(6),
        onTap: () {
          showModalBottomSheet<void>(
            context: context,
            useSafeArea: true,
            isScrollControlled: true,
            backgroundColor: Colors.white,
            shape: const RoundedRectangleBorder(
              borderRadius: BorderRadius.vertical(top: Radius.circular(8)),
            ),
            builder: (sheetContext) => SafeArea(
              top: false,
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 10, 20, 24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Center(
                      child: Container(
                        width: 34,
                        height: 4,
                        decoration: BoxDecoration(
                          color: Colors.black.withValues(alpha: 0.16),
                          borderRadius: BorderRadius.circular(2),
                        ),
                      ),
                    ),
                    const SizedBox(height: 14),
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            title,
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.w700,
                              color: Colors.black87,
                            ),
                          ),
                        ),
                        IconButton(
                          tooltip: '关闭',
                          onPressed: () => Navigator.of(sheetContext).pop(),
                          icon: const Icon(Icons.close),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    child,
                  ],
                ),
              ),
            ),
          );
        },
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 18),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  title,
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: FontWeight.w700,
                    color: Colors.black87,
                  ),
                ),
              ),
              const Icon(
                Icons.keyboard_arrow_up_rounded,
                color: Colors.black54,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _StudyDerivationalFamily extends StatelessWidget {
  const _StudyDerivationalFamily({required this.question});

  final StudyQuestion question;

  @override
  Widget build(BuildContext context) => _StudyMeaningCard(
    cardKey: const ValueKey('study-derivational-family-card'),
    meanings: [
      StudyDerivationalMeaning(
        word: question.word,
        meaning: question.acceptedMeanings.join('；'),
      ),
      ...question.derivationalFamily,
    ],
  );
}

class _StudySynonyms extends StatelessWidget {
  const _StudySynonyms({required this.question});

  final StudyQuestion question;

  @override
  Widget build(BuildContext context) => _StudySynonymGroupCard(
    cardKey: const ValueKey('study-synonym-card'),
    groups: question.synonymGroups,
  );
}

class _StudySynonymGroupCard extends StatelessWidget {
  const _StudySynonymGroupCard({required this.cardKey, required this.groups});

  final Key cardKey;
  final List<StudySynonymGroup> groups;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final height = math.min(
      studyDerivationalFamilyCardMaxHeightForTest,
      54.0 + groups.length * 58.0,
    );
    return SizedBox(
      height: height,
      child: Scrollbar(
        key: cardKey,
        child: ListView.separated(
          primary: false,
          padding: const EdgeInsets.fromLTRB(14, 14, 14, 14),
          itemCount: groups.length,
          separatorBuilder: (_, _) => const SizedBox(height: 14),
          itemBuilder: (context, index) {
            final group = groups[index];
            return Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  group.meaning,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: Colors.black87,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 5),
                Wrap(
                  spacing: 12,
                  runSpacing: 5,
                  children: group.words
                      .map(
                        (word) => Text(
                          word,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: Colors.black87,
                          ),
                        ),
                      )
                      .toList(growable: false),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _StudyMeaningCard extends StatelessWidget {
  const _StudyMeaningCard({required this.cardKey, required this.meanings});

  final Key cardKey;
  final List<StudyDerivationalMeaning> meanings;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final meaningsByWord = <String, Set<String>>{};
    final displayWords = <String, String>{};
    for (final item in meanings) {
      final key = item.word.trim().toLowerCase();
      if (key.isEmpty) continue;
      displayWords.putIfAbsent(key, () => item.word.trim());
      final meanings = meaningsByWord.putIfAbsent(key, () => <String>{});
      for (final meaning in item.meaning.split(RegExp(r'[；;]'))) {
        final normalizedMeaning = meaning.trim();
        if (normalizedMeaning.isNotEmpty) meanings.add(normalizedMeaning);
      }
    }
    final displayMeanings = meaningsByWord.entries
        .map(
          (entry) => StudyDerivationalMeaning(
            word: displayWords[entry.key]!,
            meaning: entry.value.where((value) => value.isNotEmpty).join('；'),
          ),
        )
        .toList(growable: false);
    final height = math.min(
      studyDerivationalFamilyCardMaxHeightForTest,
      54.0 + displayMeanings.length * 64.0,
    );
    return SizedBox(
      height: height,
      child: Scrollbar(
        key: cardKey,
        child: ListView.separated(
          primary: false,
          shrinkWrap: true,
          physics: const ClampingScrollPhysics(),
          padding: const EdgeInsets.fromLTRB(2, 0, 2, 4),
          itemCount: displayMeanings.length,
          separatorBuilder: (_, _) => const SizedBox(height: 6),
          itemBuilder: (context, index) {
            final item = displayMeanings[index];
            return Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  width: 112,
                  child: Text(
                    item.word,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      fontWeight: FontWeight.w700,
                      color: Colors.black87,
                    ),
                  ),
                ),
                Expanded(
                  child: Text(
                    item.meaning,
                    style: theme.textTheme.bodyMedium?.copyWith(
                      color: Colors.black87,
                    ),
                  ),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}

const double studyDerivationalFamilyCardMaxHeightForTest = 260;

Widget studyDerivationalFamilyCardForTest(StudyQuestion question) =>
    _StudyDerivationalFamily(question: question);

Widget studyHighFrequencyMeaningDrawersForTest(StudyQuestion question) =>
    Column(
      children: [
        if (question.derivationalFamily.isNotEmpty)
          _StudyMeaningDrawer(
            title: '同源词组',
            drawerKey: const ValueKey('study-derivational-family-drawer'),
            child: _StudyDerivationalFamily(question: question),
          ),
        if (question.derivationalFamily.isNotEmpty &&
            question.synonymGroups.isNotEmpty)
          const SizedBox(height: 12),
        if (question.synonymGroups.isNotEmpty)
          _StudyMeaningDrawer(
            title: '近义词',
            drawerKey: const ValueKey('study-synonym-drawer'),
            child: _StudySynonyms(question: question),
          ),
      ],
    );

Widget studyAnsweredHighFrequencyContentForTest({
  required StudyQuestion question,
  required StudyResult result,
  required ExamPracticeClient examPracticeClient,
  required ScrollController scrollController,
  required TextEditingController answerController,
  required FocusNode answerFocusNode,
}) => SingleChildScrollView(
  key: const ValueKey('study-answered-high-frequency-scroll'),
  controller: scrollController,
  child: _QuestionComposer(
    question: question,
    examPracticeClient: examPracticeClient,
    mode: 'highFrequency',
    answerController: answerController,
    answerFocusNode: answerFocusNode,
    selectedChoice: null,
    onChoiceSelected: (_) {},
    onSubmit: () async {},
    onSubmitChoice: (_) {},
    submitting: true,
    answered: true,
    result: result,
  ),
);

String wordSkeletonDisplayForTest(String prompt, String answer) {
  final answerChars = answer
      .trim()
      .characters
      .where((char) => RegExp(r'[A-Za-z]').hasMatch(char))
      .toList(growable: false);
  var answerIndex = 0;
  final buffer = StringBuffer();
  final promptChars = prompt.characters.toList(growable: false);
  for (var index = 0; index < promptChars.length; index += 1) {
    final char = promptChars[index];
    if (char == '_' && answerIndex < answerChars.length) {
      buffer.write(answerChars[answerIndex]);
      answerIndex += 1;
    } else {
      buffer.write(char);
    }
    if (char == '_' &&
        answerIndex >= answerChars.length &&
        index + 1 < promptChars.length &&
        promptChars[index + 1] == '_') {
      buffer.write(' ');
    }
  }
  return buffer.toString();
}

class _WordSkeletonPreview extends StatelessWidget {
  const _WordSkeletonPreview({required this.prompt, required this.answer});

  final String prompt;
  final String answer;

  @override
  Widget build(BuildContext context) {
    final baseStyle = Theme.of(context).textTheme.headlineSmall?.copyWith(
      fontWeight: FontWeight.w700,
      color: Colors.black,
    );
    final answerStyle = baseStyle?.copyWith(
      fontSize: (baseStyle.fontSize ?? 24) + 8,
      fontWeight: FontWeight.w900,
    );
    final answerChars = answer
        .trim()
        .characters
        .where((char) => RegExp(r'[A-Za-z]').hasMatch(char))
        .toList(growable: false);
    var answerIndex = 0;
    final spans = <TextSpan>[];
    final promptChars = prompt.characters.toList(growable: false);
    for (var index = 0; index < promptChars.length; index += 1) {
      final char = promptChars[index];
      if (char == '_' && answerIndex < answerChars.length) {
        spans.add(TextSpan(text: answerChars[answerIndex], style: answerStyle));
        answerIndex += 1;
      } else {
        spans.add(TextSpan(text: char, style: baseStyle));
      }
      if (char == '_' &&
          answerIndex >= answerChars.length &&
          index + 1 < promptChars.length &&
          promptChars[index + 1] == '_') {
        spans.add(TextSpan(text: ' ', style: baseStyle));
      }
    }
    return Text.rich(TextSpan(children: spans));
  }
}

List<Widget> _buildChoiceOptions(
  BuildContext context,
  StudyQuestion question,
  String? selectedChoice,
  StudyResult? result,
  String? responseOverride,
  ValueChanged<String>? onChoiceSelected,
  ValueChanged<String>? onSubmitChoice,
  bool submitting,
) {
  final choices = question.choices ?? const [];
  final answered = result != null;
  final correctTextToken = _resolvedCorrectChoiceTextToken(question, result);

  return choices.indexed
      .map((indexedChoice) {
        final index = indexedChoice.$1;
        final choice = indexedChoice.$2;
        final display = _choiceDisplay(choice, index);
        final isSelected = selectedChoice == display.value;
        final state = _choiceState(
          question,
          result,
          display,
          correctTextToken,
          responseOverride: responseOverride,
        );
        final isCorrect = answered && state.isCorrect;
        final isUserWrong = answered && state.isUserWrong;

        Color bgColor;
        Color textColor;
        Color labelColor;

        if (answered) {
          if (isCorrect) {
            bgColor = const Color(0xFFE8F5E9);
            textColor = const Color(0xFF2E7D32);
            labelColor = const Color(0xFF2E7D32);
          } else if (isUserWrong) {
            bgColor = const Color(0xFFFFEBEE);
            textColor = const Color(0xFFC62828);
            labelColor = const Color(0xFFC62828);
          } else {
            bgColor = const Color(0xFFF2F3F5);
            textColor = const Color(0xFF999999);
            labelColor = const Color(0xFF999999);
          }
        } else {
          bgColor = isSelected
              ? const Color(0xFFE8F5E9)
              : const Color(0xFFF2F3F5);
          textColor = isSelected
              ? const Color(0xFF1B5E20)
              : const Color(0xFF333333);
          labelColor = isSelected
              ? const Color(0xFF1B5E20)
              : const Color(0xFF888888);
        }

        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: InkWell(
            borderRadius: BorderRadius.circular(8),
            onTap: answered || submitting || isSelected
                ? null
                : () => onChoiceSelected?.call(display.value),
            onDoubleTap: answered || submitting
                ? null
                : () {
                    onChoiceSelected?.call(display.value);
                    onSubmitChoice?.call(display.value);
                  },
            child: Ink(
              decoration: BoxDecoration(
                borderRadius: BorderRadius.circular(8),
                color: bgColor,
              ),
              child: Padding(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 14,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 54,
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            display.label,
                            style: Theme.of(context).textTheme.titleMedium
                                ?.copyWith(
                                  color: labelColor,
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                          if (answered && isCorrect) ...[
                            const SizedBox(width: 5),
                            const Icon(
                              Icons.check_circle,
                              size: 20,
                              color: Color(0xFF2E7D32),
                            ),
                          ] else if (answered && isUserWrong) ...[
                            const SizedBox(width: 5),
                            const Icon(
                              Icons.cancel,
                              size: 20,
                              color: Color(0xFFC62828),
                            ),
                          ],
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        display.text,
                        style: Theme.of(context).textTheme.titleMedium
                            ?.copyWith(
                              color: textColor,
                              fontWeight: FontWeight.w600,
                            ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      })
      .toList(growable: false);
}

Widget _buildInputFeedback(
  BuildContext context,
  StudyResult result,
  StudyQuestion question,
) {
  final isCorrect =
      result.outcome == AnswerOutcome.correct ||
      result.outcome == AnswerOutcome.fuzzyCorrect;
  final answerColor = isCorrect
      ? const Color(0xFF2E7D32)
      : const Color(0xFFC62828);
  final displayResponse = result.userResponse.trim().isEmpty
      ? '\u672a\u4f5c\u7b54'
      : result.userResponse;

  return Padding(
    padding: const EdgeInsets.only(top: 8),
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (result.userResponse.isNotEmpty || !isCorrect) ...[
          DecoratedBox(
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: answerColor.withValues(alpha: 0.35)),
              color: answerColor.withValues(alpha: 0.08),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
              child: Text(
                displayResponse,
                style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: answerColor,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
        ],
        if (!isCorrect &&
            inputCorrectAnswerTextForTest(result, question).isNotEmpty)
          Text(
            'Correct answer: ${inputCorrectAnswerTextForTest(result, question)}',
            style: Theme.of(context).textTheme.bodyMedium?.copyWith(
              color: const Color(0xFF2E7D32),
              fontWeight: FontWeight.w700,
            ),
          ),
      ],
    ),
  );
}

String inputCorrectAnswerTextForTest(
  StudyResult result,
  StudyQuestion question,
) {
  final resultAnswer = result.correctAnswer.trim();
  if (resultAnswer.isNotEmpty) return resultAnswer;
  return question.acceptedMeanings
      .map((meaning) => meaning.trim())
      .where((meaning) => meaning.isNotEmpty)
      .join(' / ');
}

const _fallbackChoiceLabels = ['A', 'B', 'C', 'D'];

({String value, String label, String text}) _choiceDisplay(
  Map<String, dynamic> choice,
  int index,
) {
  final fallbackLabel = index < _fallbackChoiceLabels.length
      ? _fallbackChoiceLabels[index]
      : '${index + 1}';
  final label =
      _firstNonEmptyChoiceField(choice, const [
        'label',
        'Label',
        'value',
        'Value',
      ]) ??
      fallbackLabel;
  final text =
      _firstNonEmptyChoiceField(choice, const [
        'text',
        'Text',
        'meaning',
        'Meaning',
        'word',
        'Word',
      ]) ??
      label;
  return (value: label, label: label, text: text);
}

String? _firstNonEmptyChoiceField(
  Map<String, dynamic> choice,
  List<String> keys,
) {
  for (final key in keys) {
    final value = choice[key]?.toString().trim();
    if (value != null && value.isNotEmpty) {
      return value;
    }
  }
  return null;
}

String _normalizeChoiceToken(String? value) =>
    (value ?? '').trim().toLowerCase().replaceAll(RegExp(r'\s+'), ' ');

Set<String> _choiceUserAnswerTokens(
  StudyResult? result, {
  String? responseOverride,
}) {
  final override = responseOverride?.trim();
  final raw = override != null && override.isNotEmpty
      ? override
      : result?.userResponse ?? '';
  if (raw.trim().isEmpty) return const <String>{};
  return {
    _normalizeChoiceToken(raw),
    ...raw
        .split(RegExp('[,;/\uFF0C\uFF1B\u3001]'))
        .map(_normalizeChoiceToken)
        .where((token) => token.isNotEmpty),
  };
}

bool _shouldShowHeroHintChip(StudyQuestion question) =>
    questionHasHintForAction(question);

bool _shouldAutoJumpToCurrentQuestion({
  required StudyQuestion? pendingNextQuestion,
  required bool pendingNextVisible,
}) => pendingNextQuestion == null && !pendingNextVisible;

bool _shouldInsertCompletionPage({
  required bool hasCompletion,
  required bool completionVisible,
}) => hasCompletion && completionVisible;

bool _shouldInsertPendingNextPage({
  required bool hasPendingNext,
  required bool pendingNextVisible,
}) => hasPendingNext && pendingNextVisible;

bool _shouldAcceptStudySubmit({
  required bool submitting,
  required String? activeQuestionId,
  required String questionId,
}) {
  if (submitting) return false;
  if (activeQuestionId == null) return true;
  return activeQuestionId != questionId;
}

bool _shouldSettleAnswerInputBeforeSubmit({
  required bool hasFocus,
  required bool isChoiceType,
}) => !isChoiceType;

bool _inputAnswerFieldReadOnlyWhileSubmitting() => false;

bool _shouldAutoOpenKeyboardForStudyInput() => true;

bool _canMarkStudyItemMastered({
  required bool submitting,
  required bool isAnswered,
  required bool isActiveQuestion,
  required bool isMastered,
}) => !submitting && !isMastered && (isAnswered || isActiveQuestion);

IconData _masteredActionIcon(bool isMastered) =>
    isMastered ? Icons.check_circle_outline : Icons.delete_outline;

bool _shouldRevealCompletionAfterMastered({
  required bool wasAnswered,
  required bool isComplete,
}) => isComplete && !wasAnswered;

bool _shouldShowSupplementalExamExample({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) {
  if (!answered ||
      mode != 'highFrequency' ||
      question.questionType == 'exampleToCnChoice' ||
      question.questionType == 'exampleToCnChoiceNoTranslation' ||
      question.questionType == 'wordSkeletonInput' ||
      question.questionType == 'glossToRootInput' ||
      question.questionType == 'rootToGlossInput') {
    return false;
  }
  final sentence = (question.exampleSentence ?? '').trim();
  final source = (question.exampleTranslation ?? '').trim();
  return sentence.isNotEmpty &&
      source.startsWith('\u771f\u9898\u6765\u6e90\uff1a');
}

double _keyboardInsetSpacerHeight({required double baseHeight}) => baseHeight;

SessionProgress _studyFeedProgress({
  required StudyQuestion? visibleQuestion,
  StudyQuestion? fallbackQuestion,
  required int visibleIndex,
  required int itemCount,
  required int? sessionTotal,
}) {
  final question = visibleQuestion ?? fallbackQuestion;
  final total = sessionTotal ?? question?.totalQuestions ?? itemCount;
  final current = question != null ? question.questionIndex + 1 : visibleIndex;
  return SessionProgress(
    current: total > 0 ? current.clamp(1, total) : current,
    total: total,
  );
}

({String value, String label, String text}) choiceDisplayForTest(
  Map<String, dynamic> choice,
  int index,
) => _choiceDisplay(choice, index);

bool shouldShowHeroHintChipForTest(StudyQuestion question) =>
    _shouldShowHeroHintChip(question);

bool _hintUsedForStudySubmit({
  required String mode,
  required String questionId,
  required Set<String> clickedQuestionIds,
}) => mode == 'highFrequency' && clickedQuestionIds.contains(questionId);

void _recordStudyHintUse({
  required String mode,
  required String questionId,
  required Set<String> clickedQuestionIds,
}) {
  if (mode == 'highFrequency') {
    clickedQuestionIds.add(questionId);
  }
}

bool hintUsedForStudySubmitForTest({
  required String mode,
  required String questionId,
  required Set<String> clickedQuestionIds,
}) => _hintUsedForStudySubmit(
  mode: mode,
  questionId: questionId,
  clickedQuestionIds: clickedQuestionIds,
);

Widget studyQuestionHeaderForTest(StudyQuestion question) =>
    _FeedQuestionHeader(question: question);

bool shouldAutoJumpToCurrentQuestionForTest(
  StudyQuestion? pendingNextQuestion,
) => _shouldAutoJumpToCurrentQuestion(
  pendingNextQuestion: pendingNextQuestion,
  pendingNextVisible: false,
);

bool shouldAutoJumpToCurrentQuestionWithVisiblePendingForTest({
  required StudyQuestion? pendingNextQuestion,
  required bool pendingNextVisible,
}) => _shouldAutoJumpToCurrentQuestion(
  pendingNextQuestion: pendingNextQuestion,
  pendingNextVisible: pendingNextVisible,
);

Widget studyAdvanceGestureRegionForTest({
  required Widget child,
  VoidCallback? onAdvance,
  bool enabled = true,
}) => _AdvanceGestureRegion(
  enabled: enabled,
  hasPendingNext: true,
  onPendingNextSwipe: onAdvance ?? () {},
  onCompletionSwipe: () {},
  child: child,
);

ScrollPhysics? studyQuestionScrollPhysicsForTest({
  required bool continuationEnabled,
}) => _studyQuestionScrollPhysics(continuationEnabled: continuationEnabled);

bool shouldInsertCompletionPageForTest({
  required bool hasCompletion,
  required bool completionVisible,
}) => _shouldInsertCompletionPage(
  hasCompletion: hasCompletion,
  completionVisible: completionVisible,
);

bool shouldInsertPendingNextPageForTest({
  required bool hasPendingNext,
  required bool pendingNextVisible,
}) => _shouldInsertPendingNextPage(
  hasPendingNext: hasPendingNext,
  pendingNextVisible: pendingNextVisible,
);

int questionFeedIndexForTest({
  required List<AnsweredStudyQuestion> answeredQuestions,
  required StudyQuestion currentQuestion,
  required StudyQuestion? pendingNextQuestion,
  bool pendingNextVisible = false,
  required String questionId,
}) {
  final feedItems = [
    ...answeredQuestions.map(_StudyFeedItem.answered),
    if (!answeredQuestions.any(
      (answered) => answered.question.questionId == currentQuestion.questionId,
    ))
      _StudyFeedItem.unanswered(currentQuestion),
    if (_shouldInsertPendingNextPage(
      hasPendingNext: pendingNextQuestion != null,
      pendingNextVisible: pendingNextVisible,
    ))
      _StudyFeedItem.unanswered(pendingNextQuestion!),
  ];
  return feedItems.indexWhere(
    (item) => item.question?.questionId == questionId,
  );
}

bool questionHasHintForAction(StudyQuestion? question) {
  final hint = question?.userHint?.trim();
  return question?.hasHint == true && hint != null && hint.isNotEmpty;
}

String _studyActionFailureMessage(String action) {
  if (action == 'dispute') {
    return '\u5f02\u8bae\u63d0\u4ea4\u5931\u8d25\uff0c\u672c\u8f6e\u5b66\u4e60\u5df2\u4fdd\u7559';
  }
  return '\u64cd\u4f5c\u5931\u8d25\uff0c\u8bf7\u91cd\u8bd5';
}

String _studyMasteredSuccessMessage({required bool wasAnswered}) => wasAnswered
    ? '\u5df2\u6807\u8bb0\u4e3a\u638c\u63e1\uff0c\u5df2\u4fdd\u7559\u672c\u9898\u7b54\u9898\u8bb0\u5f55'
    : '\u5df2\u6807\u8bb0\u4e3a\u638c\u63e1';

String studyMasteredSuccessMessageForTest({required bool wasAnswered}) =>
    _studyMasteredSuccessMessage(wasAnswered: wasAnswered);

IconData masteredActionIconForTest({required bool isMastered}) =>
    _masteredActionIcon(isMastered);

String studyActionFailureMessageForTest(String action) =>
    _studyActionFailureMessage(action);

String _studyDisputeAcceptedMessage() =>
    '\u5df2\u63a5\u53d7\u5f02\u8bae\uff0c\u5df2\u52a0\u5165\u540c\u6b65\u961f\u5217';

String studyDisputeAcceptedMessageForTest() => _studyDisputeAcceptedMessage();

bool roundActionButtonVisualActiveForTest({
  required bool active,
  required bool enabled,
}) => active && enabled;

bool shouldAcceptStudySubmitForTest({
  required bool submitting,
  required String? activeQuestionId,
  required String questionId,
}) => _shouldAcceptStudySubmit(
  submitting: submitting,
  activeQuestionId: activeQuestionId,
  questionId: questionId,
);

bool shouldSettleAnswerInputBeforeSubmitForTest({
  required bool hasFocus,
  required bool isChoiceType,
}) => _shouldSettleAnswerInputBeforeSubmit(
  hasFocus: hasFocus,
  isChoiceType: isChoiceType,
);

bool inputAnswerFieldReadOnlyWhileSubmittingForTest() =>
    _inputAnswerFieldReadOnlyWhileSubmitting();

bool shouldAutoOpenKeyboardForStudyInputForTest() =>
    _shouldAutoOpenKeyboardForStudyInput();

bool canMarkStudyItemMasteredForTest({
  required bool submitting,
  required bool isAnswered,
  required bool isActiveQuestion,
  bool isMastered = false,
}) => _canMarkStudyItemMastered(
  submitting: submitting,
  isAnswered: isAnswered,
  isActiveQuestion: isActiveQuestion,
  isMastered: isMastered,
);

bool shouldRevealCompletionAfterMasteredForTest({
  required bool wasAnswered,
  required bool isComplete,
}) => _shouldRevealCompletionAfterMastered(
  wasAnswered: wasAnswered,
  isComplete: isComplete,
);

bool shouldShowSupplementalExamExampleForTest({
  required StudyQuestion question,
  required String mode,
  required bool answered,
}) => _shouldShowSupplementalExamExample(
  question: question,
  mode: mode,
  answered: answered,
);

double keyboardInsetSpacerHeightForTest({
  required double baseHeight,
  required double bottomInset,
}) => _keyboardInsetSpacerHeight(baseHeight: baseHeight);

String questionLabelForTest(String type) => _questionLabel(type);

SessionProgress studyFeedProgressForTest({
  required StudyQuestion? visibleQuestion,
  StudyQuestion? fallbackQuestion,
  required int visibleIndex,
  required int itemCount,
  required int? sessionTotal,
}) => _studyFeedProgress(
  visibleQuestion: visibleQuestion,
  fallbackQuestion: fallbackQuestion,
  visibleIndex: visibleIndex,
  itemCount: itemCount,
  sessionTotal: sessionTotal,
);

bool modeUsesQuestionTypeWeightsForTest(String mode) =>
    _modeUsesQuestionTypeWeights(mode);

({bool isCorrect, bool isUserWrong}) choiceStateForTest(
  StudyQuestion question,
  StudyResult? result,
  Map<String, dynamic> choice,
  int index, {
  String? responseOverride,
}) {
  final display = _choiceDisplay(choice, index);
  return _choiceState(
    question,
    result,
    display,
    _resolvedCorrectChoiceTextToken(question, result),
    responseOverride: responseOverride,
  );
}

({bool isCorrect, bool isUserWrong}) _choiceState(
  StudyQuestion question,
  StudyResult? result,
  ({String value, String label, String text}) display,
  String correctTextToken, {
  String? responseOverride,
}) {
  final displayLabelToken = _normalizeChoiceToken(display.label);
  final displayValueToken = _normalizeChoiceToken(display.value);
  final displayTextToken = _normalizeChoiceToken(display.text);
  final userTokens = _choiceUserAnswerTokens(
    result,
    responseOverride: responseOverride,
  );
  final correctLabelToken = _normalizeChoiceToken(question.correctChoiceLabel);
  final isCorrect =
      result != null &&
      _isCorrectChoice(
        question,
        result,
        displayLabelToken,
        displayTextToken,
        correctTextToken,
      );
  return (
    isCorrect: isCorrect,
    isUserWrong:
        result != null &&
        (userTokens.contains(displayLabelToken) ||
            userTokens.contains(displayValueToken) ||
            userTokens.contains(displayTextToken)) &&
        !isCorrect &&
        (correctLabelToken.isEmpty ||
            !userTokens.contains(correctLabelToken) ||
            displayLabelToken != correctLabelToken) &&
        result.outcome != AnswerOutcome.skipped,
  );
}

bool _isCorrectChoice(
  StudyQuestion question,
  StudyResult result,
  String displayLabelToken,
  String displayTextToken,
  String correctTextToken,
) {
  final correctLabelToken = _normalizeChoiceToken(question.correctChoiceLabel);
  if (correctTextToken.isNotEmpty && displayTextToken == correctTextToken) {
    return true;
  }

  if (correctLabelToken.isNotEmpty &&
      (correctTextToken.isEmpty || displayTextToken == correctTextToken)) {
    return displayLabelToken == correctLabelToken;
  }

  if (correctTextToken.isNotEmpty) {
    return displayTextToken == correctTextToken;
  }

  if (question.questionType == 'cnToEnChoice') {
    final wordToken = _normalizeChoiceToken(question.word);
    return wordToken.isNotEmpty && displayTextToken == wordToken;
  }

  return false;
}

String _resolvedCorrectChoiceTextToken(
  StudyQuestion question,
  StudyResult? result,
) {
  final choices = question.choices ?? const [];
  final correctAnswerToken = _normalizeChoiceToken(result?.correctAnswer);
  if (correctAnswerToken.isNotEmpty) {
    for (final indexedChoice in choices.indexed) {
      final display = _choiceDisplay(indexedChoice.$2, indexedChoice.$1);
      final textToken = _normalizeChoiceToken(display.text);
      if (textToken == correctAnswerToken) {
        return textToken;
      }
    }
  }

  final correctLabelToken = _normalizeChoiceToken(question.correctChoiceLabel);
  if (correctLabelToken.isNotEmpty) {
    for (final indexedChoice in choices.indexed) {
      final display = _choiceDisplay(indexedChoice.$2, indexedChoice.$1);
      if (_normalizeChoiceToken(display.label) == correctLabelToken ||
          _normalizeChoiceToken(display.value) == correctLabelToken) {
        return _normalizeChoiceToken(display.text);
      }
    }
    return '';
  }

  if (question.questionType == 'cnToEnChoice') {
    final wordToken = _normalizeChoiceToken(question.word);
    for (final indexedChoice in choices.indexed) {
      final display = _choiceDisplay(indexedChoice.$2, indexedChoice.$1);
      final textToken = _normalizeChoiceToken(display.text);
      if (textToken == wordToken) {
        return textToken;
      }
    }
  }

  return '';
}

class _HighlightedExampleSentence extends StatelessWidget {
  const _HighlightedExampleSentence({
    required this.sentence,
    required this.word,
  });

  final String sentence;
  final String word;

  @override
  Widget build(BuildContext context) {
    final baseStyle = Theme.of(
      context,
    ).textTheme.bodyMedium?.copyWith(color: Colors.black87, height: 1.35);
    final highlightStyle = baseStyle?.copyWith(
      color: Theme.of(context).colorScheme.primary,
      fontWeight: FontWeight.w800,
      backgroundColor: Theme.of(
        context,
      ).colorScheme.primary.withValues(alpha: 0.12),
    );
    final spans = _highlightWordSpans(
      sentence: sentence,
      word: word,
      baseStyle: baseStyle,
      highlightStyle: highlightStyle,
    );

    return RichText(
      text: TextSpan(style: baseStyle, children: spans),
    );
  }

  List<TextSpan> _highlightWordSpans({
    required String sentence,
    required String word,
    required TextStyle? baseStyle,
    required TextStyle? highlightStyle,
  }) {
    final target = word.trim().toLowerCase();
    if (target.isEmpty) {
      return [TextSpan(text: sentence, style: baseStyle)];
    }

    final candidates = _wordForms(target);
    final spans = <TextSpan>[];
    var scanCursor = 0;
    var emittedCursor = 0;
    var matched = false;

    while (scanCursor < sentence.length) {
      final start = _nextWordStart(sentence, scanCursor);
      if (start < 0) break;
      final end = _wordEnd(sentence, start);
      final token = sentence.substring(start, end).toLowerCase();
      if (!_matchesWord(token, target, candidates)) {
        scanCursor = end;
        continue;
      }
      if (start > emittedCursor) {
        spans.add(TextSpan(text: sentence.substring(emittedCursor, start)));
      }
      spans.add(
        TextSpan(text: sentence.substring(start, end), style: highlightStyle),
      );
      scanCursor = end;
      emittedCursor = end;
      matched = true;
    }

    if (!matched) {
      return [TextSpan(text: sentence, style: baseStyle)];
    }
    if (emittedCursor < sentence.length) {
      spans.add(TextSpan(text: sentence.substring(emittedCursor)));
    }
    return spans;
  }

  Set<String> _wordForms(String target) {
    final forms = <String>{target};
    if (target.endsWith('y') && target.length > 1) {
      forms.add('${target.substring(0, target.length - 1)}ies');
      forms.add('${target.substring(0, target.length - 1)}ied');
    }
    if (target.endsWith('e') && target.length > 1) {
      forms.add('${target}s');
      forms.add('${target}d');
      forms.add('${target.substring(0, target.length - 1)}ing');
    } else {
      forms.add('${target}s');
      forms.add('${target}es');
      forms.add('${target}ed');
      forms.add('${target}ing');
    }
    return forms;
  }

  bool _matchesWord(String token, String target, Set<String> forms) {
    final normalized = token.endsWith("'s")
        ? token.substring(0, token.length - 2)
        : token;
    if (forms.contains(normalized)) return true;
    return _roughStem(normalized) == target;
  }

  String _roughStem(String token) {
    for (final suffix in const ['ing', 'ed', 'es', 's', 'd']) {
      if (token.length <= suffix.length + 2 || !token.endsWith(suffix)) {
        continue;
      }
      var stem = token.substring(0, token.length - suffix.length);
      if (stem.length >= 2 &&
          stem.codeUnitAt(stem.length - 1) ==
              stem.codeUnitAt(stem.length - 2)) {
        stem = stem.substring(0, stem.length - 1);
      }
      if (suffix == 'ing' || suffix == 'd') {
        return '${stem}e';
      }
      return stem;
    }
    return token;
  }

  int _nextWordStart(String value, int from) {
    for (var i = from; i < value.length; i++) {
      if (_isWordChar(value.codeUnitAt(i))) return i;
    }
    return -1;
  }

  int _wordEnd(String value, int from) {
    var index = from;
    while (index < value.length) {
      final code = value.codeUnitAt(index);
      if (!_isWordChar(code) && code != 39) break;
      index++;
    }
    return index;
  }

  bool _isWordChar(int code) {
    return (code >= 65 && code <= 90) ||
        (code >= 97 && code <= 122) ||
        (code >= 48 && code <= 57);
  }
}

Color studyExampleMarkColorForTest(String level) => switch (level) {
  'fuzzy' => const Color(0xFFDCEEFF),
  'familiar' => const Color(0xFFA9D4FF),
  'unknown' || 'wrong' => const Color(0xFF72B7F2),
  _ => Colors.transparent,
};

Map<String, String> _mergeStudyExampleMarks(
  Map<String, String> prior,
  Map<String, String> current,
) {
  final merged = Map<String, String>.from(prior);
  merged.addAll(current);
  return merged;
}

class StudyInteractiveExampleSentence extends StatefulWidget {
  const StudyInteractiveExampleSentence({
    super.key,
    required this.client,
    required this.sentence,
    required this.word,
    required this.entrySourceId,
    required this.mode,
  });

  final ExamPracticeClient client;
  final String sentence;
  final String word;
  final String entrySourceId;
  final String mode;

  @override
  State<StudyInteractiveExampleSentence> createState() =>
      _StudyInteractiveExampleSentenceState();
}

class _StudyInteractiveExampleSentenceState
    extends State<StudyInteractiveExampleSentence> {
  List<ExamWordToken> _tokens = const [];
  Map<String, String> _priorMarks = const {};
  Map<String, String> _currentMarks = const {};

  String get _articleId => 'study-example:${widget.entrySourceId}';

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void didUpdateWidget(covariant StudyInteractiveExampleSentence oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.sentence != widget.sentence ||
        oldWidget.entrySourceId != widget.entrySourceId) {
      _tokens = const [];
      _priorMarks = const {};
      _currentMarks = const {};
      _load();
    }
  }

  Future<void> _load() async {
    try {
      final tokens = await widget.client.tokenizeText(widget.sentence);
      final state = await widget.client.getAnnotationState(
        articleId: _articleId,
        words: tokens.map((token) => token.normalized).toSet(),
      );
      if (!mounted) return;
      setState(() {
        _tokens = tokens;
        _priorMarks = state.priorMarks;
        _currentMarks = state.currentMarks;
      });
    } catch (_) {
      // Keep the example readable if tokenization or mark hydration is unavailable.
    }
  }

  String? _markFor(ExamWordToken token) {
    final marks = _mergeStudyExampleMarks(_priorMarks, _currentMarks);
    final surface = _normalizeStudyExampleWordFamily(token.normalized);
    final direct = marks[surface] ?? marks[token.normalized.toLowerCase()];
    if (direct != null) return direct;
    for (final entry in marks.entries) {
      final family = _normalizeStudyExampleWordFamily(entry.key);
      if (surface == family ||
          surface == '${family}e' ||
          '${surface}e' == family) {
        return entry.value;
      }
    }
    return null;
  }

  Future<ExamWordInspection> _inspect(
    ExamWordToken token, {
    String? userMark,
  }) => widget.client.inspectWord(
    articleId: _articleId,
    title: '${widget.word} 真题例句',
    body: widget.sentence,
    token: token,
    sentenceText: widget.sentence,
    metadata: {
      'scope': 'study-example',
      'mode': widget.mode,
      'entrySourceId': widget.entrySourceId,
    },
    userMark: userMark,
  );

  Future<void> _openWord(ExamWordToken token) async {
    try {
      final inspection = await _inspect(token);
      if (!mounted) return;
      final selected = await showModalBottomSheet<String>(
        context: context,
        showDragHandle: true,
        builder: (context) => _StudyExampleWordSheet(
          word: token.text,
          meanings: inspection.meanings,
          currentLevel: _markFor(token),
          canClear: _currentMarks.containsKey(inspection.normalized),
        ),
      );
      if (selected == null || !mounted) return;
      final updated = await _inspect(token, userMark: selected);
      if (!mounted) return;
      setState(() {
        final next = Map<String, String>.from(_currentMarks);
        if (updated.userMark == 'none') {
          next.remove(updated.normalized);
        } else {
          next[updated.normalized] = updated.userMark;
        }
        _currentMarks = next;
      });
    } catch (_) {
      if (!mounted) return;
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('暂时无法读取该词释义')));
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_tokens.isEmpty) {
      return Text(
        widget.sentence,
        style: Theme.of(
          context,
        ).textTheme.bodyMedium?.copyWith(color: Colors.black87, height: 1.45),
      );
    }
    final chunks = <Widget>[];
    var cursor = 0;
    for (final token in _tokens) {
      if (token.startOffset > cursor) {
        chunks.add(Text(widget.sentence.substring(cursor, token.startOffset)));
      }
      final level = _markFor(token);
      chunks.add(
        Semantics(
          button: true,
          label: '${token.text}，点击查看中文并标记',
          child: InkWell(
            onTap: () => _openWord(token),
            child: ColoredBox(
              color: level == null
                  ? Colors.transparent
                  : studyExampleMarkColorForTest(level),
              child: Text(
                token.text,
                style: const TextStyle(
                  color: Colors.black87,
                  height: 1.45,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
        ),
      );
      cursor = token.endOffset;
    }
    if (cursor < widget.sentence.length) {
      chunks.add(Text(widget.sentence.substring(cursor)));
    }
    return DefaultTextStyle.merge(
      style: Theme.of(
        context,
      ).textTheme.bodyMedium?.copyWith(color: Colors.black87, height: 1.45),
      child: Wrap(children: chunks),
    );
  }
}

String _normalizeStudyExampleWordFamily(String value) {
  final word = value.trim().toLowerCase();
  const irregular = {
    'children': 'child',
    'people': 'person',
    'men': 'man',
    'women': 'woman',
    'went': 'go',
    'gone': 'go',
    'seen': 'see',
    'made': 'make',
    'thought': 'think',
    'written': 'write',
  };
  if (irregular[word] case final family?) return family;
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
    final stem = word.substring(0, word.length - 2);
    if (RegExp(r'(ss|x|z|ch|sh)$').hasMatch(stem)) return stem;
  }
  if (word.endsWith('s') &&
      word.length > 2 &&
      !word.endsWith('ss') &&
      !const {
        'news',
        'series',
        'species',
        'means',
        'analysis',
      }.contains(word)) {
    return word.substring(0, word.length - 1);
  }
  return word;
}

class _StudyExampleWordSheet extends StatelessWidget {
  const _StudyExampleWordSheet({
    required this.word,
    required this.meanings,
    required this.currentLevel,
    required this.canClear,
  });

  final String word;
  final List<String> meanings;
  final String? currentLevel;
  final bool canClear;

  @override
  Widget build(BuildContext context) {
    const labels = {'fuzzy': '释义模糊', 'familiar': '眼熟', 'unknown': '完全不会'};
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 4, 20, 20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              word,
              style: Theme.of(
                context,
              ).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800),
            ),
            const SizedBox(height: 8),
            Text(
              meanings.isEmpty ? '词书中暂无中文释义' : meanings.join('；'),
              style: Theme.of(context).textTheme.bodyLarge,
            ),
            const SizedBox(height: 18),
            Row(
              children: [
                for (final entry in labels.entries)
                  Expanded(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 3),
                      child: InkWell(
                        onTap: () => Navigator.pop(context, entry.key),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Container(
                              height: 36,
                              decoration: BoxDecoration(
                                color: studyExampleMarkColorForTest(entry.key),
                                border: currentLevel == entry.key
                                    ? Border.all(
                                        color: Colors.black87,
                                        width: 2,
                                      )
                                    : null,
                              ),
                            ),
                            const SizedBox(height: 5),
                            Text(entry.value, textAlign: TextAlign.center),
                          ],
                        ),
                      ),
                    ),
                  ),
              ],
            ),
            if (canClear) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  onPressed: () => Navigator.pop(context, 'none'),
                  child: const Text('取消本例标记'),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _RootAffixRelatedWords extends StatelessWidget {
  const _RootAffixRelatedWords({required this.question});

  final StudyQuestion question;

  @override
  Widget build(BuildContext context) {
    final words = _splitList(question.exampleSentence);
    final glosses = _splitList(question.exampleTranslation);
    final rows = <({String word, String gloss})>[];
    final maxRows = words.length > glosses.length
        ? words.length
        : glosses.length;
    for (var i = 0; i < maxRows; i++) {
      rows.add((
        word: i < words.length ? words[i] : '',
        gloss: i < glosses.length ? glosses[i] : '',
      ));
    }

    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xFFF4F7F6),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: const Color(0xFFD7E8E2)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Related words',
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                color: Theme.of(context).colorScheme.primary,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 8),
            for (var i = 0; i < rows.length; i++) ...[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Expanded(
                    child: _HighlightedAffixWord(
                      word: rows[i].word,
                      affix: question.prompt,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Text(
                      rows[i].gloss,
                      style: Theme.of(
                        context,
                      ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                    ),
                  ),
                ],
              ),
              if (i != rows.length - 1) const SizedBox(height: 8),
            ],
          ],
        ),
      ),
    );
  }

  List<String> _splitList(String? value) {
    if (value == null || value.trim().isEmpty) {
      return const [];
    }
    return value
        .split(',')
        .map((item) => item.trim())
        .where((item) => item.isNotEmpty)
        .toList(growable: false);
  }
}

class _HighlightedAffixWord extends StatelessWidget {
  const _HighlightedAffixWord({required this.word, required this.affix});

  final String word;
  final String affix;

  @override
  Widget build(BuildContext context) {
    final baseStyle = Theme.of(context).textTheme.bodyMedium?.copyWith(
      fontWeight: FontWeight.w600,
      color: Colors.black87,
    );
    final highlightStyle = baseStyle?.copyWith(
      color: Theme.of(context).colorScheme.primary,
      fontWeight: FontWeight.w800,
      backgroundColor: Theme.of(
        context,
      ).colorScheme.primary.withValues(alpha: 0.12),
    );
    final normalizedAffix = affix.replaceAll('-', '').toLowerCase();
    final normalizedWord = word.toLowerCase();
    final start = normalizedAffix.isEmpty
        ? -1
        : normalizedWord.indexOf(normalizedAffix);
    if (start < 0) {
      return Text(word, style: baseStyle);
    }
    final end = start + normalizedAffix.length;
    return RichText(
      text: TextSpan(
        style: baseStyle,
        children: [
          if (start > 0) TextSpan(text: word.substring(0, start)),
          TextSpan(text: word.substring(start, end), style: highlightStyle),
          if (end < word.length) TextSpan(text: word.substring(end)),
        ],
      ),
    );
  }
}

List<Map<String, dynamic>>? _questionTypeWeightsForMode(
  Map<String, dynamic>? weightsByMode,
  String mode,
) {
  final modeWeights = weightsByMode?[mode];
  if (modeWeights is! Map) return null;
  final out = <Map<String, dynamic>>[];
  for (final entry in modeWeights.entries) {
    final weight = entry.value is num ? (entry.value as num).toInt() : 0;
    if (weight <= 0) continue;
    out.add({'questionType': entry.key.toString(), 'weight': weight});
  }
  return out.isEmpty ? null : out;
}

bool _modeUsesQuestionTypeWeights(String mode) =>
    mode != 'newWord' && mode != 'highFrequency' && mode != 'rootAffix';

String _questionLabel(String type) {
  return switch (type) {
    'exampleToCnChoiceNoTranslation' =>
      '\u6839\u636e\u82f1\u6587\u4f8b\u53e5\u9009\u62e9\u4e2d\u6587\u91ca\u4e49',
    'wordSkeletonInput' =>
      '\u6839\u636e\u82f1\u6587\u8865\u5168\u7f3a\u5931\u5b57\u6bcd',
    'enToCnChoice' =>
      '\u6839\u636e\u82f1\u6587\u9009\u62e9\u4e2d\u6587\u91ca\u4e49',
    'exampleToCnChoice' =>
      '\u6839\u636e\u4f8b\u53e5\u9009\u62e9\u4e2d\u6587\u91ca\u4e49',
    'cnToEnChoice' =>
      '\u6839\u636e\u4e2d\u6587\u9009\u62e9\u82f1\u6587\u5355\u8bcd',
    'enToCnInput' =>
      '\u6839\u636e\u82f1\u6587\u586b\u5199\u4e2d\u6587\u91ca\u4e49',
    'glossToRootInput' =>
      '\u6839\u636e\u542b\u4e49\u586b\u5199\u8bcd\u6839/\u8bcd\u7f00',
    'rootToGlossInput' =>
      '\u6839\u636e\u8bcd\u6839/\u8bcd\u7f00\u586b\u5199\u542b\u4e49',
    _ => '\u56de\u7b54\u95ee\u9898',
  };
}

class _StudyCompletion extends StatelessWidget {
  const _StudyCompletion({
    required this.completion,
    required this.newlyMasteredQuestions,
    required this.onRestart,
    required this.onClose,
    required this.nextMode,
    required this.onContinueNextRound,
  });

  final CompleteSessionResponse completion;
  final List<StudyQuestion> newlyMasteredQuestions;
  final Future<void> Function() onRestart;
  final VoidCallback onClose;
  final String? nextMode;
  final Future<void> Function() onContinueNextRound;

  @override
  Widget build(BuildContext context) {
    final summary = completion.summary;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          color: Theme.of(context).colorScheme.primary,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '\u672c\u8f6e\u5b66\u4e60\u5b8c\u6210',
                  style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                    color: Colors.white,
                    fontWeight: FontWeight.w700,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  completion.nextAction,
                  style: Theme.of(
                    context,
                  ).textTheme.bodyMedium?.copyWith(color: Colors.white70),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        _SummaryGrid(summary: summary),
        if (newlyMasteredQuestions.isNotEmpty) ...[
          const SizedBox(height: 16),
          _NewlyMasteredWordsPanel(questions: newlyMasteredQuestions),
        ],
        const SizedBox(height: 16),
        Row(
          children: [
            Expanded(
              child: FilledButton(
                onPressed: onClose,
                child: const Text('\u8fd4\u56de\u4eca\u65e5'),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: OutlinedButton(
                onPressed: onRestart,
                child: const Text('Session is ready for summary.'),
              ),
            ),
          ],
        ),
        if (nextMode != null) ...[
          const SizedBox(height: 12),
          FilledButton.tonal(
            onPressed: onContinueNextRound,
            child: Text('\u7ee7\u7eed\u4e0b\u4e00\u8f6e\uff1a${nextMode!}'),
          ),
        ],
      ],
    );
  }
}

class _SummaryGrid extends StatelessWidget {
  const _SummaryGrid({required this.summary});

  final SessionSummary summary;

  @override
  Widget build(BuildContext context) {
    final items = [
      ('\u6b63\u786e', '${summary.correctCount}'),
      ('\u6a21\u7cca\u6b63\u786e', '${summary.fuzzyCorrectCount}'),
      ('\u9519\u8bef', '${summary.incorrectCount}'),
      ('\u8df3\u8fc7', '${summary.skippedCount}'),
      ('Accuracy', '${summary.accuracyPercent.round()}%'),
      ('\u9519\u8bcd', '${summary.wrongWordCount}'),
    ];

    return GridView.builder(
      shrinkWrap: true,
      physics: const NeverScrollableScrollPhysics(),
      itemCount: items.length,
      gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
        crossAxisCount: 2,
        crossAxisSpacing: 12,
        mainAxisSpacing: 12,
        childAspectRatio: 1.7,
      ),
      itemBuilder: (context, index) {
        final item = items[index];
        return Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Text(item.$2, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 6),
                Text(
                  item.$1,
                  style: Theme.of(
                    context,
                  ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _AccuracyPanel extends StatelessWidget {
  const _AccuracyPanel({required this.summary});

  final SessionSummary summary;

  @override
  Widget build(BuildContext context) {
    final accuracy = (summary.accuracyPercent / 100).clamp(0.0, 1.0).toDouble();
    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xFFE8F5E9),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '${summary.accuracyPercent.round()}%',
              style: Theme.of(context).textTheme.displayMedium?.copyWith(
                color: const Color(0xFF2E7D32),
                fontWeight: FontWeight.w800,
                height: 1,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'Accuracy',
              style: Theme.of(context).textTheme.titleMedium?.copyWith(
                color: const Color(0xFF2E7D32),
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 14),
            ClipRRect(
              borderRadius: BorderRadius.circular(999),
              child: LinearProgressIndicator(
                value: accuracy,
                minHeight: 10,
                backgroundColor: Colors.white,
                valueColor: const AlwaysStoppedAnimation<Color>(
                  Color(0xFF2E7D32),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _NewlyMasteredWordsPanel extends StatelessWidget {
  const _NewlyMasteredWordsPanel({required this.questions});

  final List<StudyQuestion> questions;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '本轮新增掌握 (${questions.length})',
          style: Theme.of(context).textTheme.titleLarge?.copyWith(
            color: Colors.black,
            fontWeight: FontWeight.w800,
          ),
        ),
        const SizedBox(height: 10),
        DecoratedBox(
          decoration: BoxDecoration(
            color: const Color(0xFFE8F5E9),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
            child: Column(
              children: [
                for (var index = 0; index < questions.length; index++) ...[
                  if (index > 0)
                    const Divider(height: 1, color: Color(0x332E7D32)),
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 10),
                    child: Row(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        const Icon(
                          Icons.check_circle_outline,
                          color: Color(0xFF2E7D32),
                          size: 20,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                questions[index].word,
                                style: Theme.of(context).textTheme.titleMedium
                                    ?.copyWith(
                                      color: const Color(0xFF1B5E20),
                                      fontWeight: FontWeight.w800,
                                    ),
                              ),
                              if (questions[index].acceptedMeanings.isNotEmpty)
                                Padding(
                                  padding: const EdgeInsets.only(top: 2),
                                  child: Text(
                                    questions[index].acceptedMeanings.join('；'),
                                    style: Theme.of(context)
                                        .textTheme
                                        .bodyMedium
                                        ?.copyWith(color: Colors.black54),
                                  ),
                                ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
        ),
      ],
    );
  }
}

class _WrongWordsPanel extends StatelessWidget {
  const _WrongWordsPanel({required this.wrongWords});

  final List<_WrongWordSummary> wrongWords;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Words to review',
          style: Theme.of(context).textTheme.titleLarge?.copyWith(
            color: Colors.black,
            fontWeight: FontWeight.w800,
          ),
        ),
        const SizedBox(height: 10),
        if (wrongWords.isEmpty)
          DecoratedBox(
            decoration: BoxDecoration(
              color: const Color(0xFFF2F3F5),
              borderRadius: BorderRadius.circular(8),
            ),
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Text(
                'No missed words this round.',
                style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: Colors.black54,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          )
        else
          ...wrongWords.map(
            (word) => Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: const Color(0xFFFFEBEE),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(14),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              word.word,
                              style: Theme.of(context).textTheme.titleMedium
                                  ?.copyWith(
                                    color: const Color(0xFFC62828),
                                    fontWeight: FontWeight.w800,
                                  ),
                            ),
                            if (word.meaning.isNotEmpty) ...[
                              const SizedBox(height: 4),
                              Text(
                                word.meaning,
                                maxLines: 2,
                                overflow: TextOverflow.ellipsis,
                                style: Theme.of(context).textTheme.bodyMedium
                                    ?.copyWith(color: Colors.black54),
                              ),
                            ],
                          ],
                        ),
                      ),
                      const SizedBox(width: 10),
                      Text(
                        '${word.missCount}x',
                        style: Theme.of(context).textTheme.labelLarge?.copyWith(
                          color: const Color(0xFFC62828),
                          fontWeight: FontWeight.w800,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _WrongWordSummary {
  const _WrongWordSummary({
    required this.word,
    required this.meaning,
    required this.missCount,
  });

  final String word;
  final String meaning;
  final int missCount;
}

List<_WrongWordSummary> _wrongWordSummaries(
  List<AnsweredStudyQuestion> answeredQuestions,
) {
  final byEntry = <String, _WrongWordAccumulator>{};
  for (final answered in answeredQuestions) {
    if (!_isMissedOutcome(answered.result.outcome)) continue;
    final question = answered.question;
    final key = question.entrySourceId;
    final accumulator = byEntry.putIfAbsent(
      key,
      () => _WrongWordAccumulator(
        word: question.word,
        meaning: answered.result.correctAnswer.isNotEmpty
            ? answered.result.correctAnswer
            : question.acceptedMeanings.join(', '),
      ),
    );
    accumulator.missCount += 1;
  }
  final out = byEntry.values
      .map(
        (value) => _WrongWordSummary(
          word: value.word,
          meaning: value.meaning,
          missCount: value.missCount,
        ),
      )
      .toList(growable: false);
  out.sort((left, right) => right.missCount.compareTo(left.missCount));
  return out;
}

class _WrongWordAccumulator {
  _WrongWordAccumulator({required this.word, required this.meaning});

  final String word;
  final String meaning;
  int missCount = 0;
}

bool _isMissedOutcome(AnswerOutcome outcome) =>
    outcome == AnswerOutcome.incorrect || outcome == AnswerOutcome.skipped;

bool _canDisputeAnsweredQuestion(
  StudyQuestion question,
  StudyResult result,
  String submitted,
) =>
    !question.isChoiceType &&
    result.outcome == AnswerOutcome.incorrect &&
    submitted.trim().isNotEmpty;

String _summarySubtitle(SessionSummary summary) =>
    '${summary.correctCount + summary.fuzzyCorrectCount}/${summary.totalQuestions} correct, ${summary.wrongWordCount} words need review';

String _modeLabel(String mode) => switch (mode) {
  'newWord' => 'new words',
  'review' => 'review',
  'mixedTest' => 'mixed test',
  'wrongWordReinforcement' => 'wrong words',
  'highFrequency' => 'high-frequency words',
  'rootAffix' => 'roots',
  _ => mode,
};

class _StudyMessage extends StatelessWidget {
  const _StudyMessage({
    required this.title,
    required this.message,
    required this.actionLabel,
    required this.onAction,
  });

  final String title;
  final String message;
  final String actionLabel;
  final Future<void> Function() onAction;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(title, style: Theme.of(context).textTheme.headlineSmall),
            const SizedBox(height: 12),
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton(onPressed: onAction, child: Text(actionLabel)),
          ],
        ),
      ),
    );
  }
}

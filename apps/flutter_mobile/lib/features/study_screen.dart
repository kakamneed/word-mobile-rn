import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../sdk/sdk.dart';
import '../supabase/word_comment_service.dart';
import '../supabase/word_disputed_meaning_service.dart';
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
  final WordDisputedMeaningService _disputeService =
      WordDisputedMeaningService();
  final ValueNotifier<bool> _hasTypedAnswer = ValueNotifier(false);
  final Stopwatch _responseStopwatch = Stopwatch();
  String? _selectedChoice;

  StartSessionResponse? _session;
  SubmitAnswerResponse? _latestResponse;
  StudyQuestion? _lastSubmittedQuestion;
  StudyResult? _lastSubmittedResult;
  String? _lastSubmittedResponse;
  CompleteSessionResponse? _completion;
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
      _completion = null;
      _selectedChoice = null;
      _visiblePageIndex = 0;
    });
    _answerController.clear();
    try {
      final response = await _startWithBestAvailableSeed();
      setState(() {
        _session = response;
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
    final plan = await widget.sdk.plan.getActivePlan();
    return widget.sdk.study.startSession(
      mode: widget.mode,
      entrySourceIds: const [],
      questionTypeWeights: widget.resumeHint?.hasResume == true
          ? null
          : _questionTypeWeightsForMode(
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

    final responseText =
        (explicitResponse ?? _selectedChoice ?? _answerController.text).trim();
    if (responseText.isEmpty && !allowEmpty) return;
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final response = await widget.sdk.study.submitAnswer(
        questionId: question.questionId,
        response: responseText,
        responseTimeMs: _elapsedResponseMs,
      );
      _settleAnswerInputBeforeSubmit();
      _answerController.clear();
      setState(() {
        _latestResponse = response;
        _lastSubmittedQuestion = question;
        _lastSubmittedResult = response.result;
        _lastSubmittedResponse = responseText;
        _selectedChoice = null;
        if (_session != null) {
          final answeredQuestions = mergeLatestAnsweredQuestionForTest(
            response.answeredQuestions,
            question,
            response.result,
          );
          _session = StartSessionResponse(
            session: _session!.session,
            currentQuestion: response.currentQuestion ?? question,
            progress: response.progress,
            answeredQuestions: answeredQuestions,
          );
        }
      });
      if (response.isComplete && response.summary != null) {
        setState(() {
          _completion = CompleteSessionResponse(
            summary: response.summary!,
            nextAction: response.nextAction ?? 'Return to today',
          );
        });
      } else if (response.currentQuestion != null) {
        _restartResponseTimer();
        _focusAnswerInputForCurrentQuestion();
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
        });
      }
    }
  }

  Future<void> _markCurrentEntryMastered() async {
    final question = _currentQuestion;
    if (question == null || _submitting) return;
    _settleAnswerInputBeforeSubmit();
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      final response = await widget.sdk.study.markEntryMastered(
        entrySourceId: question.entrySourceId,
      );
      if (!mounted) return;
      setState(() {
        _selectedChoice = null;
        _lastSubmittedQuestion = null;
        _lastSubmittedResult = null;
        _lastSubmittedResponse = null;
        _answerController.clear();
        if (response.currentQuestion != null && _session != null) {
          _session = StartSessionResponse(
            session: _session!.session,
            currentQuestion: response.currentQuestion!,
            progress: response.progress,
            answeredQuestions: response.answeredQuestions,
          );
        }
        if (response.isComplete && response.summary != null) {
          _completion = CompleteSessionResponse(
            summary: response.summary!,
            nextAction: response.nextAction ?? 'Return to today',
          );
        }
      });
      if (!response.isComplete) {
        _restartResponseTimer();
        _focusAnswerInputForCurrentQuestion();
      }
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
        });
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
      final cloudRecorded = await _disputeService.recordDispute(
        entrySourceId: response.entrySourceId,
        word: response.word,
        submittedMeaning: response.acceptedMeaning,
        questionId: response.result.questionId,
        questionType: response.result.questionType,
      );
      if (!mounted) return;
      setState(() {
        if (_session != null) {
          _session = StartSessionResponse(
            session: _session!.session,
            currentQuestion: _session!.currentQuestion,
            progress: response.progress,
            answeredQuestions: response.answeredQuestions,
          );
        }
        _lastSubmittedQuestion = question;
        _lastSubmittedResult = response.result;
        _lastSubmittedResponse = submitted;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            cloudRecorded
                ? '\u5df2\u63a5\u53d7\u5f02\u8bae'
                : '\u5df2\u63a5\u53d7\u5f02\u8bae\uff0c\u672c\u673a\u5df2\u751f\u6548\uff0c\u4e91\u7aef\u8bb0\u5f55\u7a0d\u540e\u540c\u6b65',
          ),
        ),
      );
    } catch (error) {
      if (mounted) {
        setState(() {
          _error = error.toString();
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _submitting = false;
        });
      }
    }
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
      });
      return;
    }

    if (response.currentQuestion != null && _session != null) {
      setState(() {
        _session = StartSessionResponse(
          session: _session!.session,
          currentQuestion: response.currentQuestion!,
          progress: response.progress,
          answeredQuestions: response.answeredQuestions,
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
    final result = await showDialog<WordHintState?>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: Text(title),
          content: SingleChildScrollView(
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
                            avatar: const Icon(Icons.auto_awesome, size: 16),
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
                  autofocus: true,
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
              onPressed: () async {
                final text = controller.text.trim();
                final saved = await widget.sdk.wrongWords.saveWordHint(
                  entryId: entryId,
                  hintText: text,
                  source: selectedSource,
                );
                if (context.mounted) Navigator.of(context).pop(saved);
              },
              child: const Text('\u4fdd\u5b58'),
            ),
          ],
        ),
      ),
    );
    controller.dispose();
    return result;
  }

  void _settleAnswerInputBeforeSubmit() {
    if (!_answerFocusNode.hasFocus) return;
    _answerFocusNode.unfocus();
  }

  void _focusAnswerInputForCurrentQuestion() {
    final question = _currentQuestion;
    if (question == null || question.isChoiceType) {
      _settleAnswerInputBeforeSubmit();
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _submitting) return;
      final current = _currentQuestion;
      if (current == null || current.isChoiceType) return;
      if (!_answerFocusNode.hasFocus) {
        _answerFocusNode.requestFocus();
      }
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
    final completion = _completion;
    if (completion != null) {
      answered.add(
        _StudyFeedItem.completion(
          completion,
          answeredQuestions,
          nextMode: _nextModeFrom(widget.mode),
        ),
      );
    }
    return answered;
  }

  void _jumpToCurrentFeedPage() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_pageController.hasClients) return;
      final target = ((_feedItems.length - 1).clamp(
        0,
        _feedItems.length,
      )).toInt();
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
                    return _StudyFeedPage(
                      item: item,
                      showCompletionHint:
                          index + 1 < feedItems.length &&
                          feedItems[index + 1].isCompletion,
                      progress: _studyFeedProgress(
                        visibleQuestion: item.question,
                        visibleIndex: index + 1,
                        itemCount: feedItems.length,
                        sessionTotal: _session?.progress.total,
                      ),
                      answerController: _answerController,
                      answerFocusNode: _answerFocusNode,
                      selectedChoice: item.isAnswered ? null : _selectedChoice,
                      submitting: _submitting,
                      onChoiceSelected: (choice) {
                        if (item.isAnswered) return;
                        setState(() {
                          _selectedChoice = choice;
                        });
                      },
                      onSubmit: () {
                        if (item.isAnswered) return;
                        final question = item.question;
                        if (question == null) return;
                        _submit(questionOverride: question);
                      },
                      onSubmitChoice: (choice) {
                        if (item.isAnswered) return;
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
                              _editHintForQuestion(question);
                            }
                          : null,
                      onComments: () {
                        final question = item.question;
                        if (question == null) return;
                        _showWordComments(question);
                      },
                      onRevealAnswer: () {
                        if (item.isAnswered) return;
                        final question = item.question;
                        if (question == null) return;
                        _revealCurrentAnswer(question);
                      },
                      onDispute: _canDisputeFeedItem(item)
                          ? () => _acceptDispute(item)
                          : null,
                      onMastered: () {
                        if (item.isAnswered) return;
                        _markCurrentEntryMastered();
                      },
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
    return _studyFeedProgress(
      visibleQuestion: visibleQuestion,
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
    final entryId = int.tryParse(question.entrySourceId);
    if (entryId == null) return;
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
    String? nextMode,
  }) => _StudyFeedItem._(
    completion: completion,
    answeredQuestions: answeredQuestions,
    nextMode: nextMode,
  );

  final StudyQuestion? question;
  final StudyResult? result;
  final String? responseOverride;
  final CompleteSessionResponse? completion;
  final List<AnsweredStudyQuestion> answeredQuestions;
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

class _StudyFeedPage extends StatelessWidget {
  const _StudyFeedPage({
    required this.item,
    required this.progress,
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
  });

  final _StudyFeedItem item;
  final SessionProgress progress;
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
  final VoidCallback onMastered;

  @override
  Widget build(BuildContext context) {
    final question = item.question;
    if (question == null) return const SizedBox.shrink();
    final result = item.result;
    final answered = item.isAnswered;
    return ColoredBox(
      color: const Color(0xFFF9FAF7),
      child: SafeArea(
        top: false,
        child: Stack(
          children: [
            Positioned.fill(
              child: SingleChildScrollView(
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
                    const _KeyboardInsetSpacer(baseHeight: 24),
                  ],
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
                    tooltip: 'Mastered',
                    icon: Icons.delete_outline,
                    onPressed: answered || submitting ? null : onMastered,
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

class _FeedQuestionHeader extends StatelessWidget {
  const _FeedQuestionHeader({required this.question});

  final StudyQuestion question;

  @override
  Widget build(BuildContext context) {
    final hero = _studyHeroDisplay(question);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          hero.title,
          style: Theme.of(context).textTheme.displaySmall?.copyWith(
            fontWeight: FontWeight.w800,
            height: 1.06,
            color: Colors.black,
          ),
        ),
        if (hero.subtitle != null) ...[
          const SizedBox(height: 8),
          Text(
            hero.subtitle!,
            style: Theme.of(context).textTheme.titleMedium?.copyWith(
              color: Colors.black54,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
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
    final enabledColor = active ? theme.colorScheme.primary : Colors.black87;
    final backgroundColor = active
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
    'wrongWordReinforcement' => 'rootAffix',
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
  Widget build(BuildContext context) {
    final bottomInset = MediaQuery.viewInsetsOf(context).bottom;
    return SizedBox(height: baseHeight + bottomInset);
  }
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
    final isExampleChoice =
        (question.questionType == 'exampleToCnChoice' ||
            question.questionType == 'exampleToCnChoiceNoTranslation') &&
        question.exampleSentence != null;
    final displayPrompt =
        isRootAffix || isExampleChoice || isCnToEnChoice || isWordSkeleton
        ? null
        : question.prompt;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if ((question.phoneticUs ?? question.phoneticUk) != null)
          Text(
            question.phoneticUs ?? question.phoneticUk ?? '',
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
          ),
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
            sentence: question.exampleSentence!,
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
          ),
        if (!isChoice) ...[
          if (!answered)
            TextField(
              controller: answerController,
              focusNode: answerFocusNode,
              readOnly: submitting,
              keyboardType: TextInputType.text,
              obscureText: false,
              enableIMEPersonalizedLearning: true,
              autofillHints: null,
              inputFormatters: isWordSkeleton
                  ? [FilteringTextInputFormatter.allow(RegExp(r'[A-Za-z]'))]
                  : null,
              enableSuggestions: true,
              autocorrect: false,
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
          if (answered && result != null)
            _buildInputFeedback(context, result!, question),
        ],
      ],
    );
  }
}

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
            onTap: answered || isSelected
                ? null
                : () => onChoiceSelected?.call(display.value),
            onDoubleTap: answered
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

SessionProgress _studyFeedProgress({
  required StudyQuestion? visibleQuestion,
  required int visibleIndex,
  required int itemCount,
  required int? sessionTotal,
}) => SessionProgress(
  current: visibleQuestion != null
      ? visibleQuestion.questionIndex + 1
      : visibleIndex,
  total: sessionTotal ?? visibleQuestion?.totalQuestions ?? itemCount,
);

({String value, String label, String text}) choiceDisplayForTest(
  Map<String, dynamic> choice,
  int index,
) => _choiceDisplay(choice, index);

bool shouldShowHeroHintChipForTest(StudyQuestion question) =>
    _shouldShowHeroHintChip(question);

bool questionHasHintForAction(StudyQuestion? question) =>
    question?.hasHint == true || question?.hintSuggestions.isNotEmpty == true;

String questionLabelForTest(String type) => _questionLabel(type);

SessionProgress studyFeedProgressForTest({
  required StudyQuestion? visibleQuestion,
  required int visibleIndex,
  required int itemCount,
  required int? sessionTotal,
}) => _studyFeedProgress(
  visibleQuestion: visibleQuestion,
  visibleIndex: visibleIndex,
  itemCount: itemCount,
  sessionTotal: sessionTotal,
);

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
    required this.onRestart,
    required this.onClose,
    required this.nextMode,
    required this.onContinueNextRound,
  });

  final CompleteSessionResponse completion;
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

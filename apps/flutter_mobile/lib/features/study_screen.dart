import 'package:flutter/material.dart';

import '../sdk/sdk.dart';

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
  final Stopwatch _responseStopwatch = Stopwatch();
  String? _selectedChoice;

  StartSessionResponse? _session;
  SubmitAnswerResponse? _latestResponse;
  CompleteSessionResponse? _completion;
  String? _error;
  bool _loading = true;
  bool _submitting = false;

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
    _answerController.dispose();
    super.dispose();
  }

  void _handleAnswerChanged() {
    if (mounted) {
      setState(() {});
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
      _completion = null;
      _selectedChoice = null;
    });
    _answerController.clear();
    try {
      final response = await _startWithBestAvailableSeed();
      setState(() {
        _session = response;
      });
      _restartResponseTimer();
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
    return widget.sdk.study.startSession(
      mode: widget.mode,
      entrySourceIds: const [],
    );
  }

  Future<void> _submit({bool allowEmpty = false}) async {
    final question = _currentQuestion;
    if (question == null) return;

    final responseText = (_selectedChoice ?? _answerController.text).trim();
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
      _answerController.clear();
      setState(() {
        _latestResponse = response;
        _selectedChoice = null;
      });
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
        );
        _latestResponse = null;
        _selectedChoice = null;
      });
      _restartResponseTimer();
    }
  }

  Future<void> _skip() async {
    _answerController.text = '';
    setState(() {
      _selectedChoice = null;
    });
    await _submit(allowEmpty: true);
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
        title: const Text('退出本轮学习？'),
        content: const Text('当前进度会被保留，并同步到首页。返回首页后可以继续学习，也可以先去看其他页面。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop('continue'),
            child: const Text('继续学习'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop('today'),
            child: const Text('返回 Today'),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop('end'),
            child: const Text('结束本轮'),
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
    await _complete();
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

  @override
  Widget build(BuildContext context) {
    final currentQuestion = _currentQuestion;
    final showQuestion =
        !_loading &&
        _error == null &&
        _completion == null &&
        currentQuestion != null;
    final showFeedback = _latestResponse != null && _completion == null;
    final isChoiceQuestion = currentQuestion?.isChoiceType ?? false;
    final canSubmit = isChoiceQuestion
        ? _selectedChoice != null
        : _answerController.text.trim().isNotEmpty;

    return Scaffold(
      appBar: AppBar(
        title: const Text('Study'),
        leading: IconButton(
          onPressed: _returnToTodayKeepingProgress,
          tooltip: 'Return to today',
          icon: const Icon(Icons.home_outlined),
        ),
        actions: [
          IconButton(
            onPressed: _showExitOptions,
            tooltip: 'Exit options',
            icon: const Icon(Icons.close),
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? _StudyMessage(
              title: 'Study error',
              message: _error!,
              actionLabel: 'Restart session',
              onAction: _start,
            )
          : _completion != null
          ? _StudyCompletion(
              completion: _completion!,
              onRestart: _start,
              onClose: () => _complete(closeAfter: true),
              nextMode: _nextModeFrom(widget.mode),
              onContinueNextRound: () => _startNextRound(),
            )
          : _currentQuestion == null
          ? _StudyMessage(
              title: 'No question',
              message: 'Session did not return a current question.',
              actionLabel: 'Restart session',
              onAction: _start,
            )
          : Column(
              children: [
                Expanded(
                  child: ListView(
                    padding: const EdgeInsets.fromLTRB(16, 10, 16, 10),
                    children: [
                      _SessionHero(
                        question: currentQuestion!,
                        progress: _session!.progress,
                      ),
                      if (_latestResponse == null)
                        _QuestionComposer(
                          question: currentQuestion,
                          answerController: _answerController,
                          selectedChoice: _selectedChoice,
                          onChoiceSelected: (choice) {
                            setState(() {
                              _selectedChoice = choice;
                            });
                          },
                          onSubmit: _submit,
                          submitting: _submitting,
                        ),
                      if (_latestResponse != null)
                        _FeedbackPanel(
                          response: _latestResponse!,
                          onAdvance: _advance,
                        ),
                    ],
                  ),
                ),
              ],
            ),
      bottomNavigationBar: showFeedback
          ? SafeArea(
              top: false,
              child: _FeedbackBottomBar(
                onNext: _advance,
                summaryReady: _latestResponse?.summary != null,
              ),
            )
          : showQuestion
          ? SafeArea(
              top: false,
              child: _StudyBottomBar(
                showSkip: !isChoiceQuestion,
                canSubmit: canSubmit && !_submitting,
                submitting: _submitting,
                onSubmit: _submit,
                onSkip: _skip,
                onReturnHome: _returnToTodayKeepingProgress,
              ),
            )
          : null,
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
  return (title: question.word, subtitle: question.partOfSpeech);
}

({String title, String? subtitle}) studyHeroDisplayForTest(
  StudyQuestion question,
) => _studyHeroDisplay(question);

class _SessionHero extends StatelessWidget {
  const _SessionHero({required this.question, required this.progress});

  final StudyQuestion question;
  final SessionProgress progress;

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

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      color: const Color(0xFF1F6F5E),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '${question.questionIndex}/${question.totalQuestions}',
              style: Theme.of(
                context,
              ).textTheme.labelLarge?.copyWith(color: Colors.white70),
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
              '本轮学习已推进 ${progress.current}/${progress.total}',
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

class _QuestionComposer extends StatelessWidget {
  const _QuestionComposer({
    required this.question,
    required this.answerController,
    required this.selectedChoice,
    required this.onChoiceSelected,
    required this.onSubmit,
    required this.submitting,
  });

  final StudyQuestion question;
  final TextEditingController answerController;
  final String? selectedChoice;
  final ValueChanged<String> onChoiceSelected;
  final Future<void> Function() onSubmit;
  final bool submitting;

  @override
  Widget build(BuildContext context) {
    final isChoice = question.isChoiceType;
    final isCnToEnChoice = question.questionType == 'cnToEnChoice';
    final isRootAffix =
        question.questionType == 'glossToRootInput' ||
        question.questionType == 'rootToGlossInput';
    final isExampleChoice =
        question.questionType == 'exampleToCnChoice' &&
        question.exampleSentence != null;
    final displayWord = isCnToEnChoice ? question.prompt : question.word;
    final displayPrompt = isRootAffix || isExampleChoice || isCnToEnChoice
        ? null
        : question.prompt;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              displayWord,
              style: Theme.of(
                context,
              ).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
            ),
            if ((question.phoneticUs ?? question.phoneticUk) != null ||
                question.partOfSpeech != null) ...[
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                children: [
                  if ((question.phoneticUs ?? question.phoneticUk) != null)
                    Text(
                      question.phoneticUs ?? question.phoneticUk ?? '',
                      style: Theme.of(
                        context,
                      ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                    ),
                  if (question.partOfSpeech != null)
                    Text(
                      question.partOfSpeech!,
                      style: Theme.of(
                        context,
                      ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                    ),
                ],
              ),
            ],
            const SizedBox(height: 10),
            Text(
              _questionLabel(question.questionType),
              style: Theme.of(context).textTheme.titleMedium,
            ),
            if (displayPrompt != null) ...[
              const SizedBox(height: 6),
              Text(displayPrompt),
            ],
            const SizedBox(height: 10),
            if (isExampleChoice) ...[
              Text(question.exampleSentence!),
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
            if (isChoice)
              ...(question.choices ?? const <Map<String, dynamic>>[]).map((
                choice,
              ) {
                final choiceValue =
                    '${choice['label'] ?? choice['value'] ?? choice['text'] ?? ''}';
                final choiceText =
                    '${choice['text'] ?? choice['meaning'] ?? choice['label'] ?? choiceValue}';
                final selected = selectedChoice == choiceValue;
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: () => onChoiceSelected(choiceValue),
                    child: Ink(
                      decoration: BoxDecoration(
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(
                          color: selected
                              ? const Color(0xFF1F6F5E)
                              : const Color(0xFFD8DDE3),
                        ),
                        color: selected
                            ? const Color(0xFF1F6F5E).withValues(alpha: 0.08)
                            : Colors.white,
                      ),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 10,
                        ),
                        child: Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(
                              choiceValue,
                              style: Theme.of(context).textTheme.titleMedium
                                  ?.copyWith(
                                    color: selected
                                        ? const Color(0xFF1F6F5E)
                                        : Colors.black87,
                                    fontWeight: FontWeight.w700,
                                  ),
                            ),
                            const SizedBox(width: 12),
                            Expanded(child: Text(choiceText)),
                          ],
                        ),
                      ),
                    ),
                  ),
                );
              }),
            if (!isChoice)
              TextField(
                controller: answerController,
                onSubmitted: (_) {
                  if (!submitting && answerController.text.trim().isNotEmpty) {
                    onSubmit();
                  }
                },
                decoration: const InputDecoration(
                  labelText: '输入答案',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
                textInputAction: TextInputAction.done,
                maxLines: 1,
              ),
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
              '相关词',
              style: Theme.of(context).textTheme.labelLarge?.copyWith(
                color: const Color(0xFF1F6F5E),
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
      color: const Color(0xFF1F6F5E),
      fontWeight: FontWeight.w800,
      backgroundColor: const Color(0xFFE1F1EA),
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

class _StudyBottomBar extends StatelessWidget {
  const _StudyBottomBar({
    required this.showSkip,
    required this.canSubmit,
    required this.submitting,
    required this.onSubmit,
    required this.onSkip,
    required this.onReturnHome,
  });

  final bool showSkip;
  final bool canSubmit;
  final bool submitting;
  final Future<void> Function() onSubmit;
  final Future<void> Function() onSkip;
  final Future<void> Function() onReturnHome;

  @override
  Widget build(BuildContext context) {
    return Material(
      elevation: 10,
      color: Colors.white,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 10, 16, 12),
        child: Row(
          children: [
            Expanded(
              child: OutlinedButton(
                onPressed: submitting ? null : onReturnHome,
                child: const Text('返回首页'),
              ),
            ),
            if (showSkip) ...[
              const SizedBox(width: 10),
              Expanded(
                child: OutlinedButton(
                  onPressed: submitting ? null : onSkip,
                  child: const Text('Skip'),
                ),
              ),
            ],
            const SizedBox(width: 10),
            Expanded(
              flex: 2,
              child: FilledButton(
                onPressed: canSubmit ? onSubmit : null,
                child: Text(submitting ? 'Submitting...' : '提交答案'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _FeedbackBottomBar extends StatelessWidget {
  const _FeedbackBottomBar({required this.onNext, required this.summaryReady});

  final Future<void> Function() onNext;
  final bool summaryReady;

  @override
  Widget build(BuildContext context) {
    return Material(
      elevation: 10,
      color: Colors.white,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 10, 16, 12),
        child: FilledButton(
          onPressed: onNext,
          child: Text(summaryReady ? '查看总结' : '继续下一题'),
        ),
      ),
    );
  }
}

String _questionLabel(String type) {
  return switch (type) {
    'enToCnChoice' => '根据英文选择中文释义',
    'exampleToCnChoice' => '根据例句选择中文释义',
    'cnToEnChoice' => '根据中文选择英文单词',
    'enToCnInput' => '根据英文填写中文释义',
    'glossToRootInput' => '根据含义填写词根/词缀',
    'rootToGlossInput' => '根据词根/词缀填写含义',
    _ => '回答问题',
  };
}

class _FeedbackPanel extends StatelessWidget {
  const _FeedbackPanel({required this.response, required this.onAdvance});

  final SubmitAnswerResponse response;
  final Future<void> Function() onAdvance;

  @override
  Widget build(BuildContext context) {
    final result = response.result;
    final isCorrect =
        result.outcome == AnswerOutcome.correct ||
        result.outcome == AnswerOutcome.fuzzyCorrect;
    final color = isCorrect ? const Color(0xFF2F8F6A) : const Color(0xFFB64A4A);

    return Card(
      color: color.withValues(alpha: 0.08),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              isCorrect ? '回答正确' : '这题需要再巩固',
              style: Theme.of(
                context,
              ).textTheme.titleLarge?.copyWith(color: color),
            ),
            const SizedBox(height: 8),
            Text('你的答案：${result.userResponse}'),
            Text('正确答案：${result.correctAnswer}'),
            Text('结果：${result.outcome.name}'),
            if (response.summary != null) ...[
              const SizedBox(height: 12),
              Text('这一轮已经可以进入总结。'),
            ] else if (response.currentQuestion != null) ...[
              const SizedBox(height: 12),
              Text('下一题已准备好，继续推进学习流。'),
            ],
          ],
        ),
      ),
    );
  }
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
          color: const Color(0xFF1F6F5E),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '本轮学习完成',
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
                child: const Text('返回 Today'),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: OutlinedButton(
                onPressed: onRestart,
                child: const Text('再来一轮'),
              ),
            ),
          ],
        ),
        if (nextMode != null) ...[
          const SizedBox(height: 12),
          FilledButton.tonal(
            onPressed: onContinueNextRound,
            child: Text('继续下一轮：${nextMode!}'),
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
      ('正确', '${summary.correctCount}'),
      ('模糊正确', '${summary.fuzzyCorrectCount}'),
      ('错误', '${summary.incorrectCount}'),
      ('跳过', '${summary.skippedCount}'),
      ('正确率', '${summary.accuracyPercent.round()}%'),
      ('错词', '${summary.wrongWordCount}'),
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

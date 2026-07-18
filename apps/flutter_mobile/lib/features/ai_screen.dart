import 'dart:async';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'exam_paper_import_dialog.dart';
import 'wrong_word_graph_screen.dart';

enum _AiToolMode { passage, import, paperImport, paperAnalysis, graph }

enum _AiChatMessageKind { text, passage, history, importReview, loading }

class _AiChatMessage {
  const _AiChatMessage.text(this.text)
    : kind = _AiChatMessageKind.text,
      passage = null,
      history = null,
      analysis = null;

  const _AiChatMessage.passage(this.passage)
    : kind = _AiChatMessageKind.passage,
      text = null,
      history = null,
      analysis = null;

  const _AiChatMessage.history(this.history)
    : kind = _AiChatMessageKind.history,
      text = null,
      passage = null,
      analysis = null;

  const _AiChatMessage.importReview(this.analysis)
    : kind = _AiChatMessageKind.importReview,
      text = null,
      passage = null,
      history = null;

  const _AiChatMessage.loading()
    : kind = _AiChatMessageKind.loading,
      text = null,
      passage = null,
      history = null,
      analysis = null;

  final _AiChatMessageKind kind;
  final String? text;
  final AiPassage? passage;
  final List<AiPassageHistoryItem>? history;
  final AiWrongWordImportAnalysis? analysis;
}

class AiScreen extends StatefulWidget {
  const AiScreen({
    super.key,
    required this.sdk,
    required this.isSignedIn,
    this.generateOnOpen = false,
    this.showPassageFirst = false,
    this.onWrongWordsImported,
    this.onAiPassageGenerated,
  });

  final WordSdk sdk;
  final bool isSignedIn;
  final bool generateOnOpen;
  final bool showPassageFirst;
  final VoidCallback? onWrongWordsImported;
  final VoidCallback? onAiPassageGenerated;

  @override
  State<AiScreen> createState() => _AiScreenState();
}

class _AiScreenState extends State<AiScreen> {
  final _input = TextEditingController();
  final _scrollController = ScrollController();

  TodayAiPassageContext? _context;
  List<AiPassageHistoryItem> _history = const [];
  AiPassage? _passage;
  String _stylePreference = '';
  final List<_AiChatMessage> _messages = [];
  final Set<String> _selectedImportCandidateIds = {};

  bool _loading = true;
  bool _busy = false;
  bool _handledOpenIntent = false;
  _AiToolMode _mode = _AiToolMode.passage;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _input.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  Future<void> _load({bool showFullLoading = true}) async {
    if (mounted && showFullLoading) setState(() => _loading = true);
    try {
      var history = await widget.sdk.ai.getAiPassageHistory();
      if (history.isEmpty && widget.isSignedIn) {
        await _restoreCloudAiPassageHistory();
        history = await widget.sdk.ai.getAiPassageHistory();
      }
      final context = await widget.sdk.ai.getTodayAiPassageContext();
      final stylePreference = await widget.sdk.ai.getAiPassageStylePreference();
      final todayItem = _todayHistoryItem(context, history);
      final latest = todayItem == null
          ? null
          : await widget.sdk.ai.getAiPassage(todayItem.passageId);
      if (!mounted) return;
      setState(() {
        _context = context;
        _history = history;
        _passage = latest;
        _stylePreference = stylePreference.style;
      });
      if (widget.showPassageFirst && latest != null && _messages.isEmpty) {
        _appendMessage(_AiChatMessage.passage(latest));
      }
      if (widget.generateOnOpen && !_handledOpenIntent && latest == null) {
        _handledOpenIntent = true;
        await _generatePassage();
      }
    } catch (error) {
      _appendText(error.toString());
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<int> _restoreCloudAiPassageHistory({bool announce = false}) async {
    if (!widget.isSignedIn) {
      if (announce) {
        _appendText(
          '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u4ece\u4e91\u7aef\u6062\u590d AI \u77ed\u6587\u5386\u53f2\u3002',
        );
      }
      return 0;
    }
    try {
      final rows = await widget.sdk.sync.fetchCloudAiPassages();
      if (rows.isEmpty) {
        if (announce) {
          _appendText(
            '\u4e91\u7aef\u6ca1\u6709\u53ef\u6062\u590d\u7684 AI \u77ed\u6587\u5386\u53f2\u3002',
          );
        }
        return 0;
      }
      final restored = await widget.sdk.sync.restoreCloudAiPassagesToLocal(
        rows,
      );
      final history = await widget.sdk.ai.getAiPassageHistory();
      if (mounted) setState(() => _history = history);
      if (announce) {
        _appendText(
          '\u5df2\u4ece\u4e91\u7aef\u6062\u590d $restored \u7bc7 AI \u77ed\u6587\u3002',
        );
      }
      return restored;
    } catch (error) {
      if (announce) {
        _appendText(
          '\u4e91\u7aef AI \u77ed\u6587\u6062\u590d\u5931\u8d25\uff1a$error',
        );
      }
      return 0;
    }
  }

  void _appendText(String text) => _appendMessage(_AiChatMessage.text(text));

  void _appendMessage(_AiChatMessage message) {
    if (!mounted) return;
    setState(() => _messages.add(message));
    _scrollToBottom();
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scrollController.hasClients) return;
      _scrollController.animateTo(
        _scrollController.position.maxScrollExtent,
        duration: const Duration(milliseconds: 180),
        curve: Curves.easeOut,
      );
    });
  }

  bool _looksLikeGenerateIntent(String text) {
    final lower = text.toLowerCase();
    if (text.isEmpty) return true;
    return text.contains('\u751f\u6210') ||
        text.contains('\u5199\u4e00\u7bc7') ||
        text.contains('\u6765\u4e00\u7bc7') ||
        text.contains('\u51fa\u4e00\u7bc7') ||
        lower.contains('generate') ||
        lower.contains('write');
  }

  String _extractStyleInstruction(String text) {
    var style = text.trim();
    final replacements = <String>[
      '\u4ee5\u540e',
      '\u4eca\u540e',
      '\u4e0b\u6b21',
      '\u4ee5\u540e\u7684',
      '\u4eca\u540e\u7684',
      '\u4e0b\u6b21\u7684',
      '\u77ed\u6587',
      'AI \u77ed\u6587',
      'ai \u77ed\u6587',
      '\u98ce\u683c',
      '\u6539\u6210',
      '\u8bbe\u4e3a',
      '\u6309',
      '\u7528',
      '\u5199',
      '\u4e00\u70b9',
    ];
    for (final item in replacements) {
      style = style.replaceAll(item, ' ');
    }
    style = style
        .replaceAll(RegExp(r'\s+'), ' ')
        .replaceAll(RegExp(r'[闁挎稑琚埀?.!闁?闁挎稓鍣︾槐?闁?]+'), ' ')
        .trim();
    return style.isEmpty ? text.trim() : style;
  }

  Future<void> _saveStylePreferenceFromChat(String text) async {
    if (!widget.isSignedIn) {
      _appendText(
        '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u4fdd\u5b58 AI \u77ed\u6587\u98ce\u683c\u504f\u597d\u3002',
      );
      return;
    }
    final style = _extractStyleInstruction(text);
    if (style.isEmpty) {
      _appendText(
        '\u53ef\u4ee5\u544a\u8bc9\u6211\u60f3\u8981\u7684\u77ed\u6587\u98ce\u683c\uff0c\u6bd4\u5982\u201c\u79d1\u5e7b\u63a2\u9669\u201d\u6216\u201c\u6e29\u67d4\u65e5\u8bb0\u201d\u3002',
      );
      return;
    }
    try {
      final saved = await widget.sdk.ai.saveAiPassageStylePreference(style);
      if (!mounted) return;
      setState(() => _stylePreference = saved.style);
      _appendText(
        '\u5df2\u8bb0\u4f4f\u77ed\u6587\u98ce\u683c\uff1a${saved.style}\n'
        '\u4eca\u5929\u5982\u679c\u5df2\u7ecf\u751f\u6210\u8fc7\uff0c\u4f1a\u4ece\u4e0b\u4e00\u6b21\u751f\u6210\u5f00\u59cb\u751f\u6548\u3002',
      );
    } catch (error) {
      _appendText(
        '\u4fdd\u5b58\u77ed\u6587\u98ce\u683c\u5931\u8d25\uff1a$error',
      );
    }
  }

  Future<void> _handlePassageChat(String text) async {
    final lower = text.toLowerCase();
    if (text.contains('\u5386\u53f2') || lower.contains('history')) {
      await _showHistoryInChat();
      return;
    }
    final explicitGenerate = _looksLikeGenerateIntent(text);
    final hasTodayPassage =
        _passage != null ||
        (_context != null && _todayHistoryItem(_context!, _history) != null);
    if (text.isNotEmpty && (!explicitGenerate || hasTodayPassage)) {
      await _saveStylePreferenceFromChat(text);
      if (explicitGenerate && hasTodayPassage) {
        _appendText(
          '\u4eca\u5929\u7684 AI \u77ed\u6587\u5df2\u7ecf\u751f\u6210\u8fc7\uff0c\u4e3a\u4e86\u4fdd\u6301\u6bcf\u5929\u4e00\u7bc7\uff0c\u65b0\u98ce\u683c\u4f1a\u7528\u5728\u4e0b\u6b21\u751f\u6210\u3002',
        );
      }
      return;
    }
    await _generatePassage(styleInstruction: text.isEmpty ? null : text);
  }

  Future<void> _showHistoryInChat() async {
    if (_history.isEmpty) {
      await _restoreCloudAiPassageHistory();
      _history = await widget.sdk.ai.getAiPassageHistory();
    }
    if (_history.isEmpty) {
      _appendText('\u8fd8\u6ca1\u6709 AI \u77ed\u6587\u5386\u53f2\u3002');
      return;
    }
    _appendMessage(_AiChatMessage.history(_history));
  }

  Future<void> _openHistoryItem(AiPassageHistoryItem item) async {
    _appendText('\u6b63\u5728\u8bfb\u53d6\u5386\u53f2\u77ed\u6587...');
    final passage = await widget.sdk.ai.getAiPassage(item.passageId);
    if (!mounted) return;
    setState(() => _messages.removeLast());
    if (passage == null) {
      _appendText(
        '\u672a\u80fd\u8bfb\u53d6\u8fd9\u7bc7\u5386\u53f2\u77ed\u6587\u6b63\u6587\uff0c\u8bf7\u91cd\u65b0\u540c\u6b65\u540e\u518d\u8bd5\u3002',
      );
      return;
    }
    setState(() {
      _mode = _AiToolMode.passage;
      _passage = passage;
      _messages.add(_AiChatMessage.passage(passage));
    });
    _scrollToBottom();
  }

  Future<void> _generatePassage({String? styleInstruction}) async {
    if (!widget.isSignedIn) {
      _appendText(
        '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002',
      );
      return;
    }
    final context = _context;
    if (context == null) return;
    if (!context.tasksComplete) {
      _appendText(
        '\u8bf7\u5148\u5b8c\u6210\u4eca\u65e5\u5b66\u4e60\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002',
      );
      return;
    }
    final wrongWords = context.generationWrongWords.toList(growable: false);
    final targetWords = context.generationTargetWords.toList(growable: false);
    if (wrongWords.isEmpty && targetWords.isEmpty) {
      _appendText(
        '\u4eca\u5929\u8fd8\u6ca1\u6709\u53ef\u7528\u4e8e\u751f\u6210\u77ed\u6587\u7684\u9519\u8bcd\u3002',
      );
      return;
    }

    setState(() {
      _mode = _AiToolMode.passage;
      _busy = true;
      _messages.add(const _AiChatMessage.loading());
    });
    _scrollToBottom();
    try {
      final generated = await widget.sdk.ai.generateAiPassage(
        wrongWords: wrongWords,
        targetWords: targetWords,
        level: 'intermediate',
        date: context.date,
        style: styleInstruction ?? _stylePreference,
      );
      if (!mounted) return;
      setState(() {
        _messages.removeLast();
        _passage = generated;
        _messages.add(_AiChatMessage.passage(generated));
      });
      _scrollToBottom();
      widget.onAiPassageGenerated?.call();
      await widget.sdk.sync.flushPendingToCloud();
      await _load(showFullLoading: false);
    } catch (error) {
      if (!mounted) return;
      setState(() => _messages.removeLast());
      _appendText(error.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _openWrongWordGraph() async {
    setState(() => _mode = _AiToolMode.graph);
    await Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => WrongWordGraphScreen(sdk: widget.sdk)),
    );
  }

  Future<void> _openExamAnalysis() async {
    setState(() {
      _mode = _AiToolMode.paperAnalysis;
      _busy = true;
    });
    try {
      final items = await widget.sdk.examPractice.getVocabularyPriority();
      if (!mounted) return;
      await showModalBottomSheet<void>(
        context: context,
        isScrollControlled: true,
        showDragHandle: true,
        builder: (context) => _ExamPrioritySheet(items: items),
      );
    } catch (error) {
      if (mounted) {
        _appendText(
          '\u8bd5\u5377\u5206\u6790\u52a0\u8f7d\u5931\u8d25\uff1a$error',
        );
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _submitComposer() async {
    if (_mode == _AiToolMode.graph) {
      await _openWrongWordGraph();
      return;
    }
    final text = _input.text.trim();
    _input.clear();
    if (_mode == _AiToolMode.import) {
      if (text.isEmpty) {
        _appendText(
          '\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55\uff0c\u6216\u7528\u76f8\u673a\u8bc6\u522b\u9519\u8bcd\u622a\u56fe\u3002',
        );
        return;
      }
      await _runWrongWordImport(
        AiWrongWordImportSource(
          sourceType: 'text',
          sourceName: 'pasted-wrong-word-record',
          textContent: text,
        ),
      );
      return;
    }
    if (_mode == _AiToolMode.paperImport) {
      if (text.isEmpty) {
        _appendText(
          '\u7c98\u8d34\u8bd5\u5377\u6587\u672c\uff0c\u6216\u9009\u62e9 TXT / \u56fe\u7247\u6587\u4ef6\u3002',
        );
        return;
      }
      await _runExamPaperImport(
        AiWrongWordImportSource(
          sourceType: 'text',
          sourceName: 'pasted-exam-paper.txt',
          textContent: text,
        ),
      );
      return;
    }
    if (_mode == _AiToolMode.paperAnalysis) {
      _appendText(
        '\u8bd5\u5377\u5206\u6790\u4f1a\u4f7f\u7528\u5df2\u4fdd\u5b58\u8bd5\u5377\u4e0e\u4f5c\u7b54\u8bc1\u636e\u3002',
      );
      return;
    }
    await _handlePassageChat(text);
  }

  Future<void> _startImportPicker() async {
    if (_mode == _AiToolMode.paperImport) {
      final source = await widget.sdk.ai.pickWrongWordImportSource(
        sourceType: 'image',
      );
      if (source != null) {
        await _runExamPaperImport(source);
      }
      return;
    }
    await _startImageWrongWordImport();
  }

  Future<void> _startTextExamPaperImport() async {
    final source = await widget.sdk.ai.pickWrongWordImportSource(
      sourceType: 'text',
    );
    if (source != null) await _runExamPaperImport(source);
  }

  Future<void> _runExamPaperImport(AiWrongWordImportSource source) async {
    if (!widget.isSignedIn) {
      _appendText(
        '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u4f7f\u7528 AI \u89e3\u6790\u8bd5\u5377\u3002',
      );
      return;
    }
    setState(() {
      _mode = _AiToolMode.paperImport;
      _busy = true;
      _messages.add(const _AiChatMessage.loading());
    });
    _scrollToBottom();
    try {
      final draft = await widget.sdk.examPractice.analyzeImport(
        sourceType: source.sourceType,
        sourceName: source.sourceName,
        textContent: source.textContent,
        bytesBase64: source.bytesBase64,
        mimeType: source.mimeType,
      );
      if (!mounted) return;
      setState(() => _messages.removeLast());
      final confirmed = await showExamPaperImportReviewDialog(context, draft);
      if (!confirmed || !mounted) return;
      final paper = await widget.sdk.examPractice.saveImportedPaper(draft);
      _appendText(
        '\u5df2\u4fdd\u5b58\u300c${paper.title}\u300d\uff0c\u53ef\u5728\u4eca\u65e5 > \u6a21\u62df\u7ec3\u4e60\u4e2d\u9009\u62e9\u3002',
      );
    } catch (error) {
      if (!mounted) return;
      if (_messages.isNotEmpty &&
          _messages.last.kind == _AiChatMessageKind.loading) {
        setState(() => _messages.removeLast());
      }
      _appendText('\u8bd5\u5377\u89e3\u6790\u5931\u8d25\uff1a$error');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _startImageWrongWordImport() async {
    if (!widget.isSignedIn) {
      _appendText(
        '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u5bfc\u5165\u9519\u8bcd\u3002',
      );
      return;
    }
    final importSource = await widget.sdk.ai.pickWrongWordImportSource(
      sourceType: 'image',
    );
    if (importSource == null) return;
    await _runWrongWordImport(importSource);
  }

  Future<void> _runWrongWordImport(AiWrongWordImportSource source) async {
    if (!widget.isSignedIn) {
      _appendText(
        '\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u5bfc\u5165\u9519\u8bcd\u3002',
      );
      return;
    }
    setState(() {
      _mode = _AiToolMode.import;
      _busy = true;
      _messages.add(const _AiChatMessage.loading());
    });
    _scrollToBottom();
    try {
      final results = await Future.wait<dynamic>([
        widget.sdk.ai.analyzeWrongWordImport(
          sourceType: source.sourceType,
          sourceName: source.sourceName,
          textContent: source.textContent,
          bytesBase64: source.bytesBase64,
          mimeType: source.mimeType,
        ),
        Future<void>.delayed(
          source.sourceType == 'image'
              ? const Duration(milliseconds: 1200)
              : const Duration(milliseconds: 500),
        ),
      ]);
      final analysis = results.first as AiWrongWordImportAnalysis;
      final defaultSelection = analysis.candidates
          .where((candidate) => candidate.confidence >= 0.55)
          .map((candidate) => candidate.candidateId);
      if (!mounted) return;
      setState(() {
        _messages.removeLast();
        _selectedImportCandidateIds
          ..clear()
          ..addAll(defaultSelection);
        _messages.add(_AiChatMessage.importReview(analysis));
      });
      _scrollToBottom();
      if (analysis.candidates.isEmpty) {
        _appendText(
          '\u6ca1\u6709\u8bc6\u522b\u5230\u53ef\u5bfc\u5165\u7684\u9519\u8bcd\u3002',
        );
      }
    } catch (error) {
      if (!mounted) return;
      setState(() => _messages.removeLast());
      _appendText(error.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _commitSelectedImport(AiWrongWordImportAnalysis analysis) async {
    if (_selectedImportCandidateIds.isEmpty) {
      _appendText(
        '\u8bf7\u5148\u52fe\u9009\u8981\u52a0\u5165\u9519\u8bcd\u672c\u7684\u8bcd\u3002',
      );
      return;
    }
    final result = await widget.sdk.ai.commitWrongWordImport(
      analysis: analysis,
      acceptedCandidateIds: _selectedImportCandidateIds,
    );
    final added = _wordList(result.added);
    final skipped = _wordList(result.skipped);
    final highFrequency = _wordList(result.highFrequency);
    _appendText(
      '\u5df2\u786e\u8ba4\u5bfc\u5165\u3002\n'
      '\u65b0\u589e\uff1a$added\n'
      '\u8df3\u8fc7\uff1a$skipped\n'
      '\u9ad8\u9891\uff1a$highFrequency',
    );
    if (result.persisted && result.added.isNotEmpty) {
      widget.onWrongWordsImported?.call();
      await _load(showFullLoading: false);
    }
  }

  String _candidateSummary(AiWrongWordImportCandidate candidate) {
    final meaning = candidate.meaning == null || candidate.meaning!.isEmpty
        ? ''
        : ' | ${candidate.meaning}';
    final frequency = candidate.isHighFrequency ? ' | \u9ad8\u9891' : '';
    return '${candidate.occurrenceCount} \u6b21 | \u7f6e\u4fe1\u5ea6 ${(candidate.confidence * 100).round()}%$frequency$meaning\n${candidate.evidence}';
  }

  String _wordList(List<AiWrongWordImportCandidate> candidates) {
    if (candidates.isEmpty) return '\u65e0';
    return candidates.map((candidate) => candidate.word).join(', ');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('AI'), centerTitle: true),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '\u52a0\u8f7d\u4e2d...')
          : SafeArea(
              child: Column(
                children: [
                  Expanded(
                    child: _messages.isEmpty
                        ? _buildEmptyState(context)
                        : ListView.builder(
                            controller: _scrollController,
                            padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
                            itemCount: _messages.length,
                            itemBuilder: (context, index) =>
                                _buildMessage(context, _messages[index]),
                          ),
                  ),
                  _ToolSelector(
                    selected: _mode,
                    onSelected: (mode) {
                      if (mode == _AiToolMode.graph) {
                        _openWrongWordGraph();
                        return;
                      }
                      if (mode == _AiToolMode.paperAnalysis) {
                        _openExamAnalysis();
                        return;
                      }
                      setState(() => _mode = mode);
                    },
                  ),
                  _ComposerBar(
                    mode: _mode,
                    controller: _input,
                    busy: _busy,
                    signedIn: widget.isSignedIn,
                    onCamera: _startImportPicker,
                    onFile: _startTextExamPaperImport,
                    onSubmit: _submitComposer,
                  ),
                ],
              ),
            ),
    );
  }

  Widget _buildEmptyState(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.chat_bubble_outline_rounded,
              size: 56,
              color: colorScheme.onSurface.withValues(alpha: 0.18),
            ),
            const SizedBox(height: 12),
            Text(
              _emptyStateMessage(),
              textAlign: TextAlign.center,
              style: TextStyle(
                color: colorScheme.onSurface.withValues(alpha: 0.42),
                fontSize: 14,
                height: 1.45,
              ),
            ),
            if (widget.isSignedIn && _history.isEmpty) ...[
              const SizedBox(height: 16),
              TextButton.icon(
                onPressed: () => _restoreCloudAiPassageHistory(announce: true),
                icon: const Icon(Icons.cloud_download_outlined, size: 18),
                label: const Text(
                  '\u6062\u590d\u4e91\u7aef AI \u77ed\u6587\u5386\u53f2',
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _emptyStateMessage() {
    return switch (_mode) {
      _AiToolMode.passage =>
        _stylePreference.isEmpty
            ? '\u8f93\u5165\u60f3\u8981\u7684\u77ed\u6587\u98ce\u683c\uff0c\u6216\u8f93\u5165\u5386\u53f2\u67e5\u770b AI \u77ed\u6587\u5386\u53f2\u3002'
            : '\u5f53\u524d\u77ed\u6587\u98ce\u683c\uff1a$_stylePreference\u3002\u53ef\u4ee5\u7ee7\u7eed\u544a\u8bc9\u6211\u4e0b\u6b21\u60f3\u600e\u4e48\u5199\u3002',
      _AiToolMode.import =>
        '\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55\uff0c\u6216\u70b9\u76f8\u673a\u8bc6\u522b\u9519\u8bcd\u622a\u56fe\u3002',
      _AiToolMode.paperImport =>
        '\u7c98\u8d34\u8bd5\u5377\u6587\u672c\uff0c\u6216\u5bfc\u5165\u56fe\u7247 / TXT\uff0c\u786e\u8ba4\u8349\u7a3f\u540e\u4fdd\u5b58\u3002',
      _AiToolMode.paperAnalysis =>
        '\u5206\u6790\u5df2\u4fdd\u5b58\u8bd5\u5377\u7684\u8bcd\u9891\u3001\u9519\u9898\u8bc1\u636e\u4e0e\u5355\u8bcd\u4f18\u5148\u5ea6\u3002',
      _AiToolMode.graph =>
        '\u70b9\u4e0b\u65b9\u77e5\u8bc6\u56fe\u8c31\u5165\u53e3\uff0c\u67e5\u770b\u9519\u8bcd\u5173\u8054\u7f51\u7edc\u3002',
    };
  }

  Widget _buildMessage(BuildContext context, _AiChatMessage message) {
    return switch (message.kind) {
      _AiChatMessageKind.text => _TextBubble(text: message.text ?? ''),
      _AiChatMessageKind.passage => _PassageBubble(passage: message.passage!),
      _AiChatMessageKind.history => _HistoryBubble(
        history: message.history ?? const [],
        selectedPassageId: _passage?.passageId,
        onOpen: _openHistoryItem,
      ),
      _AiChatMessageKind.importReview => _ImportReviewBubble(
        analysis: message.analysis!,
        selectedIds: _selectedImportCandidateIds,
        onToggle: (candidateId, selected) {
          setState(() {
            if (selected) {
              _selectedImportCandidateIds.add(candidateId);
            } else {
              _selectedImportCandidateIds.remove(candidateId);
            }
          });
        },
        onCommit: () => _commitSelectedImport(message.analysis!),
        candidateSummary: _candidateSummary,
      ),
      _AiChatMessageKind.loading => const _StreamLoadingBubble(),
    };
  }
}

class _TextBubble extends StatelessWidget {
  const _TextBubble({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return _BubbleShell(
      child: Text(text, style: const TextStyle(height: 1.55)),
    );
  }
}

class _StreamLoadingBubble extends StatefulWidget {
  const _StreamLoadingBubble();

  @override
  State<_StreamLoadingBubble> createState() => _StreamLoadingBubbleState();
}

class _StreamLoadingBubbleState extends State<_StreamLoadingBubble>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      duration: const Duration(milliseconds: 1200),
      vsync: this,
    )..repeat();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return _BubbleShell(
      child: AnimatedBuilder(
        animation: _controller,
        builder: (context, child) {
          final dots = '.' * (1 + (_controller.value * 3).floor().clamp(0, 2));
          return Text(
            '\u6b63\u5728\u601d\u8003$dots',
            style: TextStyle(
              color: Theme.of(
                context,
              ).colorScheme.onSurface.withValues(alpha: 0.6),
              fontStyle: FontStyle.italic,
            ),
          );
        },
      ),
    );
  }
}

class _PassageBubble extends StatelessWidget {
  const _PassageBubble({required this.passage});

  final AiPassage passage;

  @override
  Widget build(BuildContext context) {
    return _BubbleShell(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            passage.title,
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 10),
          for (final block in passage.blocks)
            Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _AiBlockView(block: block),
            ),
        ],
      ),
    );
  }
}

class _HistoryBubble extends StatelessWidget {
  const _HistoryBubble({
    required this.history,
    required this.selectedPassageId,
    required this.onOpen,
  });

  final List<AiPassageHistoryItem> history;
  final String? selectedPassageId;
  final ValueChanged<AiPassageHistoryItem> onOpen;

  @override
  Widget build(BuildContext context) {
    return _BubbleShell(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '\u5386\u53f2 AI \u77ed\u6587',
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 8),
          for (final item in history.take(8))
            ListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              onTap: () => onOpen(item),
              leading: Icon(
                selectedPassageId == item.passageId
                    ? Icons.radio_button_checked
                    : Icons.radio_button_unchecked,
                size: 20,
              ),
              title: Text(item.title, style: const TextStyle(fontSize: 14)),
              subtitle: Text(
                item.preview,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(fontSize: 12),
              ),
              trailing: Text(
                item.generatedAt.split('T').first,
                style: const TextStyle(fontSize: 12),
              ),
            ),
        ],
      ),
    );
  }
}

class _ImportReviewBubble extends StatelessWidget {
  const _ImportReviewBubble({
    required this.analysis,
    required this.selectedIds,
    required this.onToggle,
    required this.onCommit,
    required this.candidateSummary,
  });

  final AiWrongWordImportAnalysis analysis;
  final Set<String> selectedIds;
  final void Function(String candidateId, bool selected) onToggle;
  final VoidCallback onCommit;
  final String Function(AiWrongWordImportCandidate candidate) candidateSummary;

  @override
  Widget build(BuildContext context) {
    final highFrequency = analysis.candidates
        .where((candidate) => candidate.isHighFrequency)
        .length;
    return _BubbleShell(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '\u8bc6\u522b\u5230 ${analysis.candidates.length} \u4e2a\u5019\u9009\u9519\u8bcd',
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 4),
          Text(
            '\u5176\u4e2d $highFrequency \u4e2a\u88ab\u5224\u5b9a\u4e3a\u9ad8\u9891\u3002\u52fe\u9009\u8981\u52a0\u5165\u9519\u8bcd\u672c\u7684\u8bcd\u3002',
            style: const TextStyle(fontSize: 13),
          ),
          const SizedBox(height: 8),
          for (final candidate in analysis.candidates)
            CheckboxListTile(
              dense: true,
              contentPadding: EdgeInsets.zero,
              value: selectedIds.contains(candidate.candidateId),
              onChanged: (checked) =>
                  onToggle(candidate.candidateId, checked ?? false),
              title: Text(candidate.word, style: const TextStyle(fontSize: 14)),
              subtitle: Text(
                candidateSummary(candidate),
                style: const TextStyle(fontSize: 12),
              ),
              secondary: candidate.isHighFrequency
                  ? const Icon(
                      Icons.priority_high_rounded,
                      color: Color(0xFFD64545),
                      size: 20,
                    )
                  : null,
            ),
          Align(
            alignment: Alignment.centerRight,
            child: FilledButton.icon(
              onPressed: selectedIds.isEmpty ? null : onCommit,
              icon: const Icon(Icons.check_rounded, size: 18),
              label: const Text('\u786e\u8ba4\u52a0\u5165\u9519\u8bcd\u672c'),
            ),
          ),
        ],
      ),
    );
  }
}

class _AiBlockView extends StatelessWidget {
  const _AiBlockView({required this.block});

  final dynamic block;

  @override
  Widget build(BuildContext context) {
    if (block is String) return Text(block);
    if (block is Map) {
      final map = Map<String, dynamic>.from(block);
      final segments = map['segments'];
      if (segments is List) {
        return RichText(
          text: TextSpan(
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
              color: Theme.of(context).colorScheme.onSurface,
              height: 1.65,
            ),
            children: segments
                .map<InlineSpan>((segment) {
                  if (segment is! Map) return TextSpan(text: '$segment');
                  final segmentMap = Map<String, dynamic>.from(segment);
                  final text = '${segmentMap['text'] ?? ''}';
                  final gloss = segmentMap['glossZh'];
                  if (segmentMap['type'] != 'word') return TextSpan(text: text);
                  final colorScheme = Theme.of(context).colorScheme;
                  return TextSpan(
                    children: [
                      TextSpan(
                        text: text,
                        style: TextStyle(
                          color: colorScheme.primary,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      if (gloss != null && '$gloss'.isNotEmpty)
                        TextSpan(
                          text: '($gloss)',
                          style: const TextStyle(
                            color: Color(0xFFB64A4A),
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                    ],
                  );
                })
                .toList(growable: false),
          ),
        );
      }
      return Text('${map['text'] ?? map['content'] ?? map}');
    }
    return Text('$block');
  }
}

class _BubbleShell extends StatelessWidget {
  const _BubbleShell({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Container(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.sizeOf(context).width - 48,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
            borderRadius: const BorderRadius.only(
              topLeft: Radius.circular(4),
              topRight: Radius.circular(18),
              bottomLeft: Radius.circular(18),
              bottomRight: Radius.circular(18),
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}

class _ExamPrioritySheet extends StatelessWidget {
  const _ExamPrioritySheet({required this.items});

  final List<ExamVocabularyPriority> items;

  @override
  Widget build(BuildContext context) {
    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.72,
      minChildSize: 0.4,
      maxChildSize: 0.92,
      builder: (context, controller) => Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(20, 4, 20, 12),
            child: Text(
              '\u5355\u8bcd\u4f18\u5148\u5ea6',
              style: Theme.of(context).textTheme.titleLarge,
            ),
          ),
          Expanded(
            child: items.isEmpty
                ? const Center(
                    child: Text(
                      '\u6682\u65e0\u53ef\u5206\u6790\u7684\u8bd5\u5377\u8bcd\u6c47',
                    ),
                  )
                : ListView.separated(
                    controller: controller,
                    padding: const EdgeInsets.fromLTRB(20, 0, 20, 24),
                    itemCount: items.length,
                    separatorBuilder: (_, _) => const Divider(height: 1),
                    itemBuilder: (context, index) {
                      final item = items[index];
                      return ListTile(
                        contentPadding: EdgeInsets.zero,
                        leading: CircleAvatar(child: Text('${index + 1}')),
                        title: Text(
                          item.word,
                          style: const TextStyle(fontWeight: FontWeight.w700),
                        ),
                        subtitle: Text(
                          '\u8de8 ${item.paperCount} \u4efd\u8bd5\u5377 \u00b7 ${item.articleCount} \u7bc7\u6587\u7ae0 \u00b7 ${item.occurrenceCount} \u6b21\n'
                          '\u4e0d\u4f1a ${item.unknownMarkCount} \u00b7 \u81f4\u9519 ${item.wrongAssociationCount} \u00b7 \u5df2\u638c\u63e1 ${item.masteredMarkCount}',
                        ),
                        trailing: Text(item.priorityScore.toStringAsFixed(1)),
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}

class _ToolSelector extends StatelessWidget {
  const _ToolSelector({required this.selected, required this.onSelected});

  final _AiToolMode selected;
  final ValueChanged<_AiToolMode> onSelected;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surface,
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 6, 16, 6),
        child: Row(
          children: [
            _ToolChip(
              icon: Icons.auto_stories_rounded,
              label: 'AI \u77ed\u6587\u67e5\u770b',
              selected: selected == _AiToolMode.passage,
              onTap: () => onSelected(_AiToolMode.passage),
            ),
            const SizedBox(width: 8),
            _ToolChip(
              icon: Icons.library_add_check_rounded,
              label: '\u9519\u8bcd\u5bfc\u5165',
              selected: selected == _AiToolMode.import,
              onTap: () => onSelected(_AiToolMode.import),
            ),
            const SizedBox(width: 8),
            _ToolChip(
              icon: Icons.note_add_outlined,
              label: '\u8bd5\u5377\u5bfc\u5165',
              selected: selected == _AiToolMode.paperImport,
              onTap: () => onSelected(_AiToolMode.paperImport),
            ),
            const SizedBox(width: 8),
            _ToolChip(
              icon: Icons.analytics_outlined,
              label: '\u8bd5\u5377\u5206\u6790',
              selected: selected == _AiToolMode.paperAnalysis,
              onTap: () => onSelected(_AiToolMode.paperAnalysis),
            ),
            const SizedBox(width: 8),
            _ToolChip(
              icon: Icons.hub_outlined,
              label: '\u77e5\u8bc6\u56fe\u8c31',
              selected: selected == _AiToolMode.graph,
              onTap: () => onSelected(_AiToolMode.graph),
            ),
          ],
        ),
      ),
    );
  }
}

class _ToolChip extends StatelessWidget {
  const _ToolChip({
    required this.icon,
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ChoiceChip(
      avatar: Icon(
        icon,
        size: 16,
        color: selected ? colorScheme.onPrimaryContainer : colorScheme.primary,
      ),
      label: Text(label, style: const TextStyle(fontSize: 12)),
      selected: selected,
      onSelected: (_) => onTap(),
      selectedColor: colorScheme.primaryContainer,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    );
  }
}

class _ComposerBar extends StatelessWidget {
  const _ComposerBar({
    required this.mode,
    required this.controller,
    required this.busy,
    required this.signedIn,
    required this.onCamera,
    required this.onFile,
    required this.onSubmit,
  });

  final _AiToolMode mode;
  final TextEditingController controller;
  final bool busy;
  final bool signedIn;
  final VoidCallback onCamera;
  final VoidCallback onFile;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final importMode =
        mode == _AiToolMode.import || mode == _AiToolMode.paperImport;
    final paperImportMode = mode == _AiToolMode.paperImport;
    final graphMode = mode == _AiToolMode.graph;
    return Material(
      color: colorScheme.surface,
      elevation: 8,
      shadowColor: Colors.black.withValues(alpha: 0.08),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 6, 12, 10),
        child: Container(
          decoration: BoxDecoration(
            color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.65),
            borderRadius: BorderRadius.circular(28),
          ),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              SizedBox(
                width: 42,
                height: 44,
                child: IconButton(
                  tooltip: importMode
                      ? '\u62cd\u7167\u8bc6\u522b'
                      : '\u5207\u6362\u5230\u9519\u8bcd\u5bfc\u5165\u540e\u4f7f\u7528',
                  onPressed: busy || !signedIn || !importMode ? null : onCamera,
                  icon: Icon(
                    Icons.photo_camera_rounded,
                    size: 22,
                    color: importMode && signedIn && !busy
                        ? colorScheme.primary
                        : colorScheme.onSurface.withValues(alpha: 0.3),
                  ),
                ),
              ),
              if (paperImportMode)
                SizedBox(
                  width: 42,
                  height: 44,
                  child: IconButton(
                    tooltip: '\u9009\u62e9 TXT',
                    onPressed: busy || !signedIn ? null : onFile,
                    icon: const Icon(Icons.description_outlined, size: 22),
                  ),
                ),
              Expanded(
                child: TextField(
                  controller: controller,
                  enabled: !busy && signedIn,
                  minLines: 1,
                  maxLines: 4,
                  textInputAction: TextInputAction.newline,
                  style: const TextStyle(fontSize: 14),
                  decoration: InputDecoration(
                    hintText: signedIn
                        ? importMode
                              ? paperImportMode
                                    ? '\u7c98\u8d34\u8bd5\u5377\u6587\u672c...'
                                    : '\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55...'
                              : graphMode
                              ? '\u6253\u5f00\u9519\u8bcd\u77e5\u8bc6\u56fe\u8c31...'
                              : '\u8bf4\u4e0b\u6b21\u60f3\u8981\u7684\u77ed\u6587\u98ce\u683c...'
                        : '\u8bf7\u5148\u767b\u5f55\u540e\u4f7f\u7528 AI',
                    hintStyle: TextStyle(
                      fontSize: 13,
                      color: colorScheme.onSurface.withValues(alpha: 0.4),
                    ),
                    border: InputBorder.none,
                    contentPadding: const EdgeInsets.symmetric(
                      horizontal: 8,
                      vertical: 12,
                    ),
                  ),
                ),
              ),
              SizedBox(
                width: 42,
                height: 44,
                child: IconButton(
                  tooltip: graphMode
                      ? '\u6253\u5f00\u77e5\u8bc6\u56fe\u8c31'
                      : importMode
                      ? paperImportMode
                            ? '\u89e3\u6790\u8bd5\u5377'
                            : '\u5bfc\u5165'
                      : '\u751f\u6210',
                  onPressed: busy || !signedIn ? null : onSubmit,
                  icon: busy
                      ? const SizedBox(
                          width: 18,
                          height: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Icon(
                          Icons.arrow_upward_rounded,
                          size: 22,
                          color: signedIn && !busy
                              ? colorScheme.primary
                              : colorScheme.onSurface.withValues(alpha: 0.3),
                        ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

AiPassageHistoryItem? _todayHistoryItem(
  TodayAiPassageContext context,
  List<AiPassageHistoryItem> history,
) {
  for (final item in history) {
    if (item.date == context.date ||
        item.generatedAt.startsWith(context.date)) {
      return item;
    }
  }
  return null;
}

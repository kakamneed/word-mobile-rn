import 'dart:async';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';

enum _AiToolMode { passage, import }

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
  });

  final WordSdk sdk;
  final bool isSignedIn;
  final bool generateOnOpen;
  final bool showPassageFirst;
  final VoidCallback? onWrongWordsImported;

  @override
  State<AiScreen> createState() => _AiScreenState();
}

class _AiScreenState extends State<AiScreen> {
  final _input = TextEditingController();
  final _scrollController = ScrollController();

  TodayAiPassageContext? _context;
  List<AiPassageHistoryItem> _history = const [];
  AiPassage? _passage;
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
      final todayItem = _todayHistoryItem(context, history);
      final latest = todayItem == null
          ? null
          : await widget.sdk.ai.getAiPassage(todayItem.passageId);
      if (!mounted) return;
      setState(() {
        _context = context;
        _history = history;
        _passage = latest;
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
        _appendText('\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u4ece\u4e91\u7aef\u6062\u590d AI \u77ed\u6587\u5386\u53f2\u3002');
      }
      return 0;
    }
    try {
      final rows = await widget.sdk.sync.fetchCloudAiPassages();
      if (rows.isEmpty) {
        if (announce) _appendText('\u4e91\u7aef\u6ca1\u6709\u53ef\u6062\u590d\u7684 AI \u77ed\u6587\u5386\u53f2\u3002');
        return 0;
      }
      final restored = await widget.sdk.sync.restoreCloudAiPassagesToLocal(rows);
      final history = await widget.sdk.ai.getAiPassageHistory();
      if (mounted) setState(() => _history = history);
      if (announce) _appendText('\u5df2\u4ece\u4e91\u7aef\u6062\u590d $restored \u7bc7 AI \u77ed\u6587\u3002');
      return restored;
    } catch (error) {
      if (announce) _appendText('\u4e91\u7aef AI \u77ed\u6587\u6062\u590d\u5931\u8d25\uff1a$error');
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
      _appendText('\u672a\u80fd\u8bfb\u53d6\u8fd9\u7bc7\u5386\u53f2\u77ed\u6587\u6b63\u6587\uff0c\u8bf7\u91cd\u65b0\u540c\u6b65\u540e\u518d\u8bd5\u3002');
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
      _appendText('\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002');
      return;
    }
    final context = _context;
    if (context == null) return;
    if (!context.tasksComplete) {
      _appendText('\u8bf7\u5148\u5b8c\u6210\u4eca\u65e5\u5b66\u4e60\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002');
      return;
    }
    final wrongWords = context.generationWrongWords.toList(growable: false);
    final targetWords = context.generationTargetWords.toList(growable: false);
    if (wrongWords.isEmpty && targetWords.isEmpty) {
      _appendText('\u4eca\u5929\u8fd8\u6ca1\u6709\u53ef\u7528\u4e8e\u751f\u6210\u77ed\u6587\u7684\u9519\u8bcd\u3002');
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
        style: styleInstruction,
      );
      if (!mounted) return;
      setState(() {
        _messages.removeLast();
        _passage = generated;
        _messages.add(_AiChatMessage.passage(generated));
      });
      _scrollToBottom();
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

  Future<void> _submitComposer() async {
    final text = _input.text.trim();
    _input.clear();
    if (_mode == _AiToolMode.import) {
      if (text.isEmpty) {
        _appendText('\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55\uff0c\u6216\u7528\u76f8\u673a\u8bc6\u522b\u9519\u8bcd\u622a\u56fe\u3002');
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
    final lower = text.toLowerCase();
    if (text.contains('\u5386\u53f2') || lower.contains('history')) {
      await _showHistoryInChat();
      return;
    }
    await _generatePassage(styleInstruction: text.isEmpty ? null : text);
  }

  Future<void> _startImageWrongWordImport() async {
    if (!widget.isSignedIn) {
      _appendText('\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u5bfc\u5165\u9519\u8bcd\u3002');
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
      _appendText('\u8bf7\u5148\u767b\u5f55\uff0c\u518d\u5bfc\u5165\u9519\u8bcd\u3002');
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
        _appendText('\u6ca1\u6709\u8bc6\u522b\u5230\u53ef\u5bfc\u5165\u7684\u9519\u8bcd\u3002');
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
      _appendText('\u8bf7\u5148\u52fe\u9009\u8981\u52a0\u5165\u9519\u8bcd\u672c\u7684\u8bcd\u3002');
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
                    onSelected: (mode) => setState(() => _mode = mode),
                  ),
                  _ComposerBar(
                    mode: _mode,
                    controller: _input,
                    busy: _busy,
                    signedIn: widget.isSignedIn,
                    onCamera: _startImageWrongWordImport,
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
              _mode == _AiToolMode.passage
                  ? '\u8f93\u5165\u60f3\u8981\u7684\u77ed\u6587\u98ce\u683c\uff0c\u6216\u8f93\u5165\u5386\u53f2\u67e5\u770b AI \u77ed\u6587\u5386\u53f2\u3002'
                  : '\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55\uff0c\u6216\u70b9\u76f8\u673a\u8bc6\u522b\u9519\u8bcd\u622a\u56fe\u3002',
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
                label: const Text('\u6062\u590d\u4e91\u7aef AI \u77ed\u6587\u5386\u53f2'),
              ),
            ],
          ],
        ),
      ),
    );
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
              color: Theme.of(context).colorScheme.onSurface.withValues(alpha: 0.6),
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
            style: Theme.of(context)
                .textTheme
                .titleMedium
                ?.copyWith(fontWeight: FontWeight.w700),
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
            style: Theme.of(context)
                .textTheme
                .titleMedium
                ?.copyWith(fontWeight: FontWeight.w700),
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
    final highFrequency =
        analysis.candidates.where((candidate) => candidate.isHighFrequency).length;
    return _BubbleShell(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            '\u8bc6\u522b\u5230 ${analysis.candidates.length} \u4e2a\u5019\u9009\u9519\u8bcd',
            style: Theme.of(context)
                .textTheme
                .titleMedium
                ?.copyWith(fontWeight: FontWeight.w700),
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
            children: segments.map<InlineSpan>((segment) {
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
            }).toList(growable: false),
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
    required this.onSubmit,
  });

  final _AiToolMode mode;
  final TextEditingController controller;
  final bool busy;
  final bool signedIn;
  final VoidCallback onCamera;
  final VoidCallback onSubmit;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final importMode = mode == _AiToolMode.import;
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
                  tooltip: importMode ? '\u62cd\u7167\u8bc6\u522b' : '\u5207\u6362\u5230\u9519\u8bcd\u5bfc\u5165\u540e\u4f7f\u7528',
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
                            ? '\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55...'
                            : '\u8f93\u5165\u77ed\u6587\u98ce\u683c\uff0c\u6216\u8f93\u5165\u5386\u53f2...'
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
                  tooltip: importMode ? '\u5bfc\u5165' : '\u751f\u6210',
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
    if (item.date == context.date || item.generatedAt.startsWith(context.date)) {
      return item;
    }
  }
  return null;
}

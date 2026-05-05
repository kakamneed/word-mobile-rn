import 'dart:async';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';

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
  TodayAiPassageContext? _context;
  List<AiPassageHistoryItem> _history = const [];
  AiPassage? _passage;
  AiWrongWordImportAnalysis? _importAnalysis;
  String? _message;
  String? _importMessage;
  String? _importProgressMessage;
  bool _loading = true;
  bool _generating = false;
  bool _analyzingImport = false;
  bool _handledOpenIntent = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _message = null;
    });
    try {
      final context = await widget.sdk.ai.getTodayAiPassageContext();
      final history = await widget.sdk.ai.getAiPassageHistory();
      final todayItem = _todayHistoryItem(context, history);
      AiPassage? latest;
      if (todayItem != null) {
        latest = await widget.sdk.ai.getAiPassage(todayItem.passageId);
      }
      setState(() {
        _context = context;
        _history = history;
        _passage = latest;
      });
      if (widget.generateOnOpen && !_handledOpenIntent && latest == null) {
        _handledOpenIntent = true;
        await _generate();
      }
    } catch (error) {
      setState(() {
        _message = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  Future<void> _openHistoryItem(AiPassageHistoryItem item) async {
    final next = await widget.sdk.ai.getAiPassage(item.passageId);
    if (!mounted) return;
    setState(() {
      _passage = next;
    });
  }

  Future<void> _generate() async {
    if (!widget.isSignedIn) {
      setState(() {
        _message = '请先登录账号，再生成 AI 短文。';
      });
      return;
    }

    final context = _context;
    if (context == null) return;

    final wrongWords = context.generationWrongWords.toList(growable: false);
    final targetWords = context.generationTargetWords.toList(growable: false);

    if (!context.tasksComplete) {
      setState(() {
        _message =
            '\u8bf7\u5148\u5b8c\u6210\u4eca\u65e5\u4efb\u52a1\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002';
      });
      return;
    }

    if (wrongWords.isEmpty && targetWords.isEmpty) {
      setState(() {
        _message =
            '\u5f53\u524d\u6ca1\u6709\u53ef\u7528\u4e8e\u751f\u6210\u77ed\u6587\u7684\u9519\u8bcd\u3002';
      });
      return;
    }

    setState(() {
      _generating = true;
      _message = null;
    });
    try {
      final generated = await widget.sdk.ai.generateAiPassage(
        wrongWords: wrongWords,
        targetWords: targetWords,
        level: 'intermediate',
        date: context.date,
      );
      setState(() {
        _passage = generated;
      });
      if (widget.isSignedIn) {
        await widget.sdk.sync.flushPendingToCloud();
      }
      await _load();
    } catch (error) {
      setState(() {
        _message = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _generating = false;
        });
      }
    }
  }

  Future<void> _startWrongWordImport(String sourceType) async {
    if (!widget.isSignedIn) {
      setState(() {
        _importMessage = '请先登录账号，再导入错词。';
      });
      return;
    }

    final importSource = sourceType == 'text'
        ? await _chooseWrongWordRecordSource()
        : await widget.sdk.ai.pickWrongWordImportSource(sourceType: sourceType);
    if (importSource == null) return;

    setState(() {
      _analyzingImport = true;
      _importMessage = null;
      _importProgressMessage = _initialImportProgressMessage(importSource);
    });
    try {
      final analysis = await _analyzeWrongWordImportWithProgress(importSource);
      if (!mounted) return;
      setState(() {
        _importAnalysis = analysis;
        _importProgressMessage = null;
      });
      if (analysis.candidates.isEmpty) {
        await _showEmptyWrongWordImportResult(analysis);
        return;
      }
      await _reviewWrongWordImport(analysis);
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _importMessage = error.toString();
        _importProgressMessage = null;
      });
      await _showWrongWordImportError(error);
    } finally {
      if (mounted) {
        setState(() {
          _analyzingImport = false;
          _importProgressMessage = null;
        });
      }
    }
  }

  Future<AiWrongWordImportAnalysis> _analyzeWrongWordImportWithProgress(
    AiWrongWordImportSource importSource,
  ) async {
    final progressTitle = importSource.sourceType == 'image'
        ? '\u6b63\u5728\u5206\u6790\u9519\u8bcd\u7167\u7247'
        : '\u6b63\u5728\u5206\u6790\u9519\u8bcd\u8bb0\u5f55';
    final progressBody = importSource.sourceType == 'image'
        ? '\u6b63\u5728\u8bfb\u53d6\u56fe\u7247\u5e76\u51c6\u5907 AI \u63d0\u53d6\u3002'
        : '\u6b63\u5728\u8bfb\u53d6\u8bb0\u5f55\u5e76\u63d0\u53d6\u5019\u9009\u8bcd\u3002';
    final minimumProgress = importSource.sourceType == 'image'
        ? const Duration(milliseconds: 1400)
        : const Duration(milliseconds: 700);

    unawaited(
      showDialog<void>(
        context: context,
        barrierDismissible: false,
        builder: (dialogContext) => AlertDialog(
          title: Text(progressTitle),
          content: const Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              LinearProgressIndicator(),
              SizedBox(height: 16),
              Text('\u8bfb\u53d6\u6765\u6e90\u5185\u5bb9'),
              SizedBox(height: 4),
              Text('\u6267\u884c AI \u63d0\u53d6'),
              SizedBox(height: 4),
              Text('\u6574\u7406\u5019\u9009\u8bcd'),
            ],
          ),
        ),
      ),
    );

    setState(() {
      _importProgressMessage = progressBody;
    });
    try {
      final analysisFuture = widget.sdk.ai.analyzeWrongWordImport(
        sourceType: importSource.sourceType,
        sourceName: importSource.sourceName,
        textContent: importSource.textContent,
        bytesBase64: importSource.bytesBase64,
        mimeType: importSource.mimeType,
      );
      final results = await Future.wait<dynamic>([
        analysisFuture,
        Future<void>.delayed(minimumProgress),
      ]);
      return results.first as AiWrongWordImportAnalysis;
    } finally {
      setState(() {
        _importProgressMessage = '\u6b63\u5728\u5b8c\u6210\u5206\u6790';
      });
      if (mounted) {
        Navigator.of(context, rootNavigator: true).pop();
      }
    }
  }

  String _initialImportProgressMessage(AiWrongWordImportSource source) {
    return source.sourceType == 'image'
        ? '\u6b63\u5728\u51c6\u5907\u7167\u7247\u5206\u6790...'
        : '\u6b63\u5728\u51c6\u5907\u8bb0\u5f55\u5206\u6790...';
  }

  Future<AiWrongWordImportSource?> _chooseWrongWordRecordSource() async {
    final choice = await showDialog<String>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('\u5bfc\u5165\u9519\u8bcd\u8bb0\u5f55'),
        content: const Text(
          '\u53ef\u4ee5\u9009\u62e9\u6587\u672c\u5bfc\u51fa\u6587\u4ef6\uff0c\u6216\u7c98\u8d34\u5176\u4ed6\u5e94\u7528\u91cc\u7684\u9519\u8bcd\u8bb0\u5f55\u3002',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('\u53d6\u6d88'),
          ),
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop('paste'),
            child: const Text('\u7c98\u8d34'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop('file'),
            child: const Text('\u9009\u62e9\u6587\u4ef6'),
          ),
        ],
      ),
    );
    if (choice == null) return null;
    if (choice == 'file') {
      return widget.sdk.ai.pickWrongWordImportSource(sourceType: 'text');
    }
    final textContent = await _showWrongWordRecordInput();
    if (textContent == null) return null;
    return AiWrongWordImportSource(
      sourceType: 'text',
      sourceName: 'pasted-wrong-word-record',
      textContent: textContent,
    );
  }

  Future<String?> _showWrongWordRecordInput() async {
    final controller = TextEditingController();
    final result = await showDialog<String>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('\u7c98\u8d34\u9519\u8bcd\u8bb0\u5f55'),
        content: TextField(
          controller: controller,
          autofocus: true,
          minLines: 6,
          maxLines: 10,
          decoration: const InputDecoration(
            hintText:
                '\u7c98\u8d34\u5355\u8bcd\u3001CSV \u6587\u672c\u6216\u5176\u4ed6\u5e94\u7528\u91cc\u7684\u7b14\u8bb0',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('\u53d6\u6d88'),
          ),
          FilledButton(
            onPressed: () {
              final value = controller.text.trim();
              Navigator.of(dialogContext).pop(value.isEmpty ? null : value);
            },
            child: const Text('\u5206\u6790'),
          ),
        ],
      ),
    );
    controller.dispose();
    return result;
  }

  Future<void> _reviewWrongWordImport(
    AiWrongWordImportAnalysis analysis,
  ) async {
    final selectedIds = analysis.candidates
        .where((candidate) => candidate.confidence >= 0.6)
        .map((candidate) => candidate.candidateId)
        .toSet();
    final acceptedIds = await showDialog<Set<String>>(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setDialogState) {
          return AlertDialog(
            title: const Text('\u786e\u8ba4\u5bfc\u5165\u7684\u9519\u8bcd'),
            content: SizedBox(
              width: double.maxFinite,
              child: SingleChildScrollView(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    for (final warning in analysis.warnings)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child: Text(
                          warning,
                          style: const TextStyle(color: Color(0xFF9A6A1D)),
                        ),
                      ),
                    for (final candidate in analysis.candidates)
                      CheckboxListTile(
                        value: selectedIds.contains(candidate.candidateId),
                        onChanged: (checked) {
                          setDialogState(() {
                            if (checked ?? false) {
                              selectedIds.add(candidate.candidateId);
                            } else {
                              selectedIds.remove(candidate.candidateId);
                            }
                          });
                        },
                        title: Text(candidate.word),
                        subtitle: Text(_candidateSummary(candidate)),
                        secondary: candidate.isHighFrequency
                            ? const Icon(
                                Icons.priority_high_rounded,
                                color: Color(0xFFD64545),
                              )
                            : null,
                      ),
                  ],
                ),
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: const Text('\u53d6\u6d88'),
              ),
              FilledButton(
                onPressed: selectedIds.isEmpty
                    ? null
                    : () => Navigator.of(dialogContext).pop(selectedIds),
                child: const Text('\u5bfc\u5165\u9009\u4e2d\u9879'),
              ),
            ],
          );
        },
      ),
    );

    if (acceptedIds == null || acceptedIds.isEmpty) return;
    final result = await widget.sdk.ai.commitWrongWordImport(
      analysis: analysis,
      acceptedCandidateIds: acceptedIds,
    );
    if (!mounted) return;
    await _showWrongWordImportResult(result);
    if (result.persisted && result.added.isNotEmpty) {
      widget.onWrongWordsImported?.call();
      await _load();
    }
  }

  Future<void> _showEmptyWrongWordImportResult(
    AiWrongWordImportAnalysis analysis,
  ) async {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('\u5206\u6790\u5b8c\u6210'),
        content: _EmptyImportAnalysisView(analysis: analysis),
        actions: [
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('\u5b8c\u6210'),
          ),
        ],
      ),
    );
  }

  Future<void> _showWrongWordImportError(Object error) async {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('\u5206\u6790\u5931\u8d25'),
        content: Text(error.toString()),
        actions: [
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('\u5b8c\u6210'),
          ),
        ],
      ),
    );
  }

  Future<void> _showWrongWordImportResult(
    AiWrongWordImportCommitResult result,
  ) async {
    await showDialog<void>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('\u9519\u8bcd\u5206\u6790\u7ed3\u679c'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              result.persisted
                  ? '\u5df2\u5c06\u9009\u4e2d\u7684\u5355\u8bcd\u52a0\u5165\u9519\u8bcd\u672c\u3002'
                  : '\u9884\u89c8\u5df2\u5b8c\u6210\uff0c\u6682\u672a\u5199\u5165\u9519\u8bcd\u672c\u3002',
            ),
            const SizedBox(height: 12),
            Text('\u5df2\u6dfb\u52a0\uff1a${_wordList(result.added)}'),
            Text('\u5df2\u8df3\u8fc7\uff1a${_wordList(result.skipped)}'),
            Text(
              '\u9ad8\u9891\u9519\u8bcd\uff1a${_wordList(result.highFrequency)}',
            ),
          ],
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('\u5b8c\u6210'),
          ),
        ],
      ),
    );
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

  Widget _buildPassageSection(BuildContext context) {
    return _SectionCard(
      title: '\u5f53\u524d\u6b63\u6587',
      subtitle:
          '\u4f18\u5148\u67e5\u770b\u6700\u8fd1\u4e00\u7bc7 AI \u77ed\u6587\u7684\u6807\u9898\u3001\u72b6\u6001\u548c\u6b63\u6587\u5757\u3002',
      child: _passage == null
          ? const Text('\u8fd8\u6ca1\u6709 AI \u77ed\u6587\u3002')
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _passage!.title,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text('\u72b6\u6001\uff1a${_passage!.validationStatus}'),
                if (_passage!.failureReason != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    _passage!.failureReason!,
                    style: const TextStyle(color: Colors.redAccent),
                  ),
                ],
                const SizedBox(height: 12),
                for (final block in _passage!.blocks)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: _AiBlockView(block: block),
                  ),
              ],
            ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final contextPayload = _context;
    final signedIn = widget.isSignedIn;

    return Scaffold(
      appBar: AppBar(title: const Text('AI 短文')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : RefreshIndicator(
              onRefresh: _load,
              child: ListView(
                physics: const AlwaysScrollableScrollPhysics(),
                padding: const EdgeInsets.all(16),
                children: [
                Card(
                  margin: const EdgeInsets.only(bottom: 16),
                  color: Theme.of(context).colorScheme.primary,
                  child: Padding(
                    padding: const EdgeInsets.all(20),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'AI \u9605\u8bfb',
                          style: Theme.of(context).textTheme.headlineSmall
                              ?.copyWith(
                                color: Colors.white,
                                fontWeight: FontWeight.w700,
                              ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          '\u548c RN \u4e00\u6837\uff0cAI \u9875\u9762\u4f18\u5148\u5c55\u793a today context\u3001\u5386\u53f2\u4e0e\u6b63\u6587\u3002',
                          style: Theme.of(context).textTheme.bodyMedium
                              ?.copyWith(
                                color: Colors.white.withValues(alpha: 0.88),
                              ),
                        ),
                        const SizedBox(height: 16),
                        Wrap(
                          spacing: 12,
                          runSpacing: 12,
                          children: [
                            _StatPill(
                              label: '\u4efb\u52a1\u5b8c\u6210',
                              value:
                                  '${contextPayload?.tasksComplete ?? false}',
                            ),
                            _StatPill(
                              label: '\u4eca\u65e5\u9519\u8bcd',
                              value:
                                  '${contextPayload?.wrongWords.length ?? 0}',
                            ),
                            _StatPill(
                              label: '\u5386\u53f2\u7bc7\u6570',
                              value: '${_history.length}',
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                if (widget.showPassageFirst && _passage != null)
                  _buildPassageSection(context),
                _SectionCard(
                  title: '\u9519\u8bcd\u5bfc\u5165',
                  subtitle:
                      '\u5206\u6790\u622a\u56fe\u3001\u624b\u5199\u7b14\u8bb0\u6216\u5bfc\u51fa\u8bb0\u5f55\uff0c\u786e\u8ba4\u540e\u518d\u52a0\u5165\u9519\u8bcd\u672c\u3002',
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Text(
                        'AI \u4f1a\u63d0\u53d6\u5019\u9009\u8bcd\u3001\u6807\u8bb0\u91cd\u590d\u6216\u9ad8\u9891\u9879\uff0c\u4f60\u786e\u8ba4\u540e\u624d\u4f1a\u5199\u5165\u9519\u8bcd\u672c\u3002',
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: [
                          FilledButton.icon(
                            onPressed: _analyzingImport || !signedIn
                                ? null
                                : () => _startWrongWordImport('image'),
                            icon: const Icon(Icons.image_search_rounded),
                            label: const Text('\u7167\u7247\u5bfc\u5165'),
                          ),
                          FilledButton.tonalIcon(
                            onPressed: _analyzingImport || !signedIn
                                ? null
                                : () => _startWrongWordImport('text'),
                            icon: const Icon(Icons.upload_file_rounded),
                            label: const Text('\u8bb0\u5f55\u5bfc\u5165'),
                          ),
                        ],
                      ),
                      if (_analyzingImport) ...[
                        const SizedBox(height: 12),
                        const LinearProgressIndicator(),
                        if (_importProgressMessage != null) ...[
                          const SizedBox(height: 8),
                          Text(_importProgressMessage!),
                        ],
                      ],
                      if (_importAnalysis != null) ...[
                        const SizedBox(height: 12),
                        Text(
                          '\u4e0a\u6b21\u5206\u6790\uff1a${_importAnalysis!.candidates.length} \u4e2a\u5019\u9009\u8bcd\uff0c${_importAnalysis!.candidates.where((candidate) => candidate.isHighFrequency).length} \u4e2a\u9ad8\u9891\u9879\u3002',
                        ),
                      ],
                      if (_importMessage != null) ...[
                        const SizedBox(height: 12),
                        Text(
                          _importMessage!,
                          style: const TextStyle(color: Colors.redAccent),
                        ),
                      ],
                    ],
                  ),
                ),
                _SectionCard(
                  title: '\u751f\u6210\u5165\u53e3',
                  subtitle:
                      '\u53ea\u6709\u5728\u4eca\u65e5\u4efb\u52a1\u5b8c\u6210\u4e14\u5b58\u5728\u9519\u8bcd\u65f6\uff0c\u624d\u5efa\u8bae\u751f\u6210 AI \u77ed\u6587\u3002',
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        contextPayload == null
                            ? '\u6682\u65f6\u6ca1\u6709 today context\u3002'
                            : contextPayload.tasksComplete
                            ? '\u4eca\u65e5\u4efb\u52a1\u5df2\u5b8c\u6210\uff0c\u53ef\u4ee5\u751f\u6210 AI \u77ed\u6587\u3002'
                            : '\u5b8c\u6210\u4eca\u65e5\u4efb\u52a1\u540e\u518d\u751f\u6210 AI \u77ed\u6587\u3002',
                      ),
                      const SizedBox(height: 8),
                      Text(
                        '\u4eca\u65e5\u9519\u8bcd\u6570\uff1a${contextPayload?.wrongWords.length ?? 0}',
                      ),
                      const SizedBox(height: 12),
                      if (!signedIn) ...[
                        const Text('登录后可使用错词导入。'),
                        const SizedBox(height: 8),
                      ],
                      FilledButton(
                        onPressed: _generating || !signedIn ? null : _generate,
                        child: Text(
                          _generating
                              ? '\u751f\u6210\u4e2d...'
                              : '\u751f\u6210 AI \u77ed\u6587',
                        ),
                      ),
                      if (_message != null) ...[
                        const SizedBox(height: 12),
                        Text(
                          _message!,
                          style: const TextStyle(color: Colors.redAccent),
                        ),
                      ],
                    ],
                  ),
                ),
                if (!widget.showPassageFirst || _passage == null)
                  _buildPassageSection(context),
                _SectionCard(
                  title: '\u5386\u53f2\u8bb0\u5f55',
                  subtitle:
                      '\u67e5\u770b\u5df2\u751f\u6210\u7684 AI \u77ed\u6587\u3002',
                  child: _history.isEmpty
                      ? const Text(
                          '\u8fd8\u6ca1\u6709\u5386\u53f2\u8bb0\u5f55\u3002',
                        )
                      : Column(
                          children: _history
                              .map((item) {
                                final selected =
                                    _passage?.passageId == item.passageId;
                                final colorScheme = Theme.of(
                                  context,
                                ).colorScheme;
                                return Card(
                                  margin: const EdgeInsets.only(bottom: 10),
                                  color: selected
                                      ? colorScheme.primary.withValues(
                                          alpha: 0.10,
                                        )
                                      : null,
                                  child: ListTile(
                                    onTap: () => _openHistoryItem(item),
                                    title: Text(item.title),
                                    subtitle: Text(item.preview),
                                    trailing: Column(
                                      mainAxisAlignment:
                                          MainAxisAlignment.center,
                                      crossAxisAlignment:
                                          CrossAxisAlignment.end,
                                      children: [
                                        Text(item.generatedAt.split('T').first),
                                        Text(item.validationStatus),
                                      ],
                                    ),
                                  ),
                                );
                              })
                              .toList(growable: false),
                        ),
                ),
                ],
              ),
            ),
    );
  }
}

class _AiBlockView extends StatelessWidget {
  const _AiBlockView({required this.block});

  final dynamic block;

  @override
  Widget build(BuildContext context) {
    if (block is String) {
      return Text(block as String);
    }

    if (block is Map<String, dynamic>) {
      final map = block as Map<String, dynamic>;
      final segments = map['segments'];
      if (segments is List) {
        return RichText(
          text: TextSpan(
            style: Theme.of(
              context,
            ).textTheme.bodyLarge?.copyWith(color: Colors.black87),
            children: segments
                .map<InlineSpan>((segment) {
                  if (segment is! Map<String, dynamic>) {
                    return TextSpan(text: '$segment');
                  }
                  final text = '${segment['text'] ?? ''}';
                  final gloss = segment['glossZh'];
                  final isWord = segment['type'] == 'word';
                  if (!isWord) {
                    return TextSpan(text: text);
                  }
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
                          text: '（$gloss）',
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

class _StatPill extends StatelessWidget {
  const _StatPill({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: Colors.white,
        borderRadius: BorderRadius.circular(14),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            value,
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
          ),
          Text(label),
        ],
      ),
    );
  }
}

class _EmptyImportAnalysisView extends StatelessWidget {
  const _EmptyImportAnalysisView({required this.analysis});

  final AiWrongWordImportAnalysis analysis;

  @override
  Widget build(BuildContext context) {
    final message = analysis.warnings.isNotEmpty
        ? analysis.warnings.first
        : '\u6ca1\u6709\u627e\u5230\u53ef\u5bfc\u5165\u7684\u5019\u9009\u8bcd\u3002';
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(message),
        if (analysis.sourceType == 'image') ...[
          const SizedBox(height: 12),
          const Text(
            '\u8bf7\u786e\u4fdd\u624b\u5199\u5355\u8bcd\u6e05\u6670\u3001\u65b9\u5411\u6b63\u786e\u3001\u8ddd\u79bb\u8db3\u591f\u8fd1\u3002\u9002\u5f53\u88c1\u6389\u65e0\u5173\u5185\u5bb9\u53ef\u4ee5\u63d0\u9ad8\u8bc6\u522b\u6548\u679c\u3002',
          ),
        ],
      ],
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.subtitle,
    required this.child,
  });

  final String title;
  final String subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 6),
            Text(
              subtitle,
              style: Theme.of(
                context,
              ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
            ),
            const SizedBox(height: 16),
            child,
          ],
        ),
      ),
    );
  }
}

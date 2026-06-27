import 'dart:convert';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'study_screen.dart';
import 'wrong_word_graph_screen.dart';

class WrongWordsScreen extends StatefulWidget {
  const WrongWordsScreen({super.key, required this.sdk, this.onStartStudy});

  final WordSdk sdk;
  final Future<void> Function(String mode)? onStartStudy;

  @override
  State<WrongWordsScreen> createState() => _WrongWordsScreenState();
}

class _WrongWordsScreenState extends State<WrongWordsScreen> {
  final ScrollController _scrollController = ScrollController();
  final GlobalKey _detailAnchorKey = GlobalKey();
  final Map<int, GlobalKey> _wordKeys = <int, GlobalKey>{};

  List<WrongWordEntry> _words = const [];
  WrongWordDetail? _detail;
  int? _selectedId;
  int _detailScrollGeneration = 0;
  String _filter = 'all';
  bool _loading = true;
  bool _loadingDetail = false;
  String? _error;

  static const _filters = <String, String>{
    'all': '全部',
    'highPriority': '高优先级',
    'recent': '最近错误',
    'frequent': '高频出错',
  };

  @override
  void initState() {
    super.initState();
    _loadAll();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  GlobalKey _keyForEntry(int entryId) {
    return _wordKeys.putIfAbsent(entryId, GlobalKey.new);
  }

  Future<void> _scrollToDetail({int attempts = 3, int? generation}) async {
    final activeGeneration = generation ?? _detailScrollGeneration;
    for (var attempt = 0; attempt < attempts; attempt++) {
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted ||
          _selectedId == null ||
          activeGeneration != _detailScrollGeneration ||
          !_scrollController.hasClients) {
        return;
      }
      final context = _detailAnchorKey.currentContext;
      if (context == null) return;
      if (!context.mounted) return;
      await Scrollable.ensureVisible(
        context,
        duration: attempt == 0
            ? Duration.zero
            : const Duration(milliseconds: 180),
        curve: Curves.easeOutCubic,
        alignment: 0,
        alignmentPolicy: ScrollPositionAlignmentPolicy.explicit,
      );
      if (attempt < attempts - 1) {
        await Future<void>.delayed(const Duration(milliseconds: 80));
      }
    }
  }

  void _scrollToSelectedWord() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final selectedId = _selectedId;
      if (selectedId == null) return;
      final context = _wordKeys[selectedId]?.currentContext;
      if (context == null) return;
      Scrollable.ensureVisible(
        context,
        duration: const Duration(milliseconds: 360),
        curve: Curves.easeOutCubic,
        alignment: 0.24,
      );
    });
  }

  Future<void> _loadAll({bool showFullLoading = true}) async {
    setState(() {
      if (showFullLoading) _loading = true;
      _error = null;
    });
    try {
      final words = await widget.sdk.wrongWords.getWrongWords(_filter);
      if (!mounted) return;
      setState(() {
        _words = words;
        _wordKeys.removeWhere(
          (entryId, _) => !words.any((word) => word.entryId == entryId),
        );
        if (_selectedId != null &&
            !words.any((word) => word.entryId == _selectedId)) {
          _selectedId = null;
          _detail = null;
        }
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  Future<void> _selectWord(WrongWordEntry entry) async {
    if (_selectedId == entry.entryId) {
      setState(() {
        _detailScrollGeneration++;
        _selectedId = null;
        _detail = null;
      });
      return;
    }
    setState(() {
      _detailScrollGeneration++;
      _selectedId = entry.entryId;
      _loadingDetail = true;
    });
    final scrollGeneration = _detailScrollGeneration;
    _scrollToDetail(generation: scrollGeneration);
    try {
      final detail = await widget.sdk.wrongWords.getWrongWordDetail(
        entry.entryId,
      );
      if (!mounted) return;
      setState(() {
        _detail = detail;
      });
      _scrollToDetail(generation: scrollGeneration);
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _loadingDetail = false;
        });
        _scrollToDetail(generation: scrollGeneration);
      }
    }
  }

  Future<void> _startReinforcement() async {
    if (widget.onStartStudy != null) {
      await widget.onStartStudy!.call('wrongWordReinforcement');
      await _loadAll();
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) =>
            StudyScreen(sdk: widget.sdk, mode: 'wrongWordReinforcement'),
      ),
    );
    await _loadAll();
  }

  Future<void> _editHint(
    WrongWordDetail detail, {
    int? effectiveErrorCount,
  }) async {
    final unlockedErrorCount = effectiveErrorCount ?? detail.errorCount;
    if (!detail.hasHint && unlockedErrorCount < 5) return;
    final controller = TextEditingController(text: detail.userHint ?? '');
    var selectedSource = detail.hintSource ?? 'user';
    final saved = await showDialog<WordHintState?>(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setDialogState) => AlertDialog(
          title: const Text('\u7f16\u8f91\u63d0\u793a\u8bcd'),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  detail.word,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 12),
                if (detail.hintSuggestions.isNotEmpty) ...[
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: detail.hintSuggestions
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
                    labelText: '提示词',
                    hintText: '输入这个词的私人记忆提示',
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
              child: const Text('清空'),
            ),
            TextButton(
              onPressed: () => Navigator.of(context).pop(null),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () async {
                final saved = await widget.sdk.wrongWords.saveWordHint(
                  entryId: detail.entryId,
                  hintText: controller.text,
                  source: selectedSource,
                );
                if (context.mounted) Navigator.of(context).pop(saved);
              },
              child: const Text('保存'),
            ),
          ],
        ),
      ),
    );
    controller.dispose();
    if (saved == null || !mounted) return;
    await _loadAll();
    if (!mounted) return;
    final refreshed = await widget.sdk.wrongWords.getWrongWordDetail(
      detail.entryId,
    );
    if (!mounted) return;
    setState(() {
      _detail = refreshed;
    });
  }

  String _primaryMeaning(List<dynamic> meanings) {
    if (meanings.isEmpty) return '暂无释义';
    final first = meanings.first;
    if (first is String) return first;
    if (first is Map<String, dynamic>) {
      return '${first['meaningCn'] ?? first['meaning'] ?? first['gloss'] ?? '暂无释义'}';
    }
    return '$first';
  }

  String _questionTypeLabel(String type) {
    return switch (_storedEnumText(type)) {
      'exampleToCnChoice' => '例句选义',
      'enToCnChoice' => '英译中选择',
      'cnToEnChoice' => '中译英选择',
      'enToCnInput' => '英译中输入',
      'cnToEnInput' => '中译英输入',
      'glossToRootInput' => '释义写词根',
      'rootToGlossInput' => '词根释义',
      final value => value,
    };
  }

  String _outcomeLabel(String outcome) {
    return switch (_storedEnumText(outcome)) {
      'incorrect' => '错误',
      'skipped' => '跳过',
      'correct' => '正确',
      'fuzzyCorrect' => '模糊正确',
      final value => value,
    };
  }

  String _historyContextLabel(String context) {
    final parts = context
        .split('/')
        .map((part) => part.trim())
        .toList(growable: false);
    if (parts.length >= 2) {
      return '${_questionTypeLabel(parts.first)} / ${_outcomeLabel(parts.sublist(1).join('/'))}';
    }
    return _storedEnumText(context);
  }

  String _storedEnumText(String value) {
    final trimmed = value.trim();
    if (trimmed.isEmpty) return trimmed;
    try {
      final decoded = jsonDecode(trimmed);
      if (decoded is String) return decoded;
    } catch (_) {
      // Some older rows are plain enum strings; leave those as-is.
    }
    if (trimmed.length >= 2 &&
        trimmed.startsWith('"') &&
        trimmed.endsWith('"')) {
      return trimmed.substring(1, trimmed.length - 1);
    }
    return trimmed;
  }

  String _historyDate(String value) {
    if (value.isEmpty) return '--';
    final cleaned = value.replaceAll('T', ' ').replaceAll('Z', '');
    return cleaned.length > 19 ? cleaned.substring(0, 19) : cleaned;
  }

  Widget _highlightedExampleSentence(String sentence, String word) {
    final baseStyle = Theme.of(context).textTheme.bodyMedium;
    final colorScheme = Theme.of(context).colorScheme;
    final highlightStyle = baseStyle?.copyWith(
      color: colorScheme.primary,
      fontWeight: FontWeight.w700,
      backgroundColor: colorScheme.primary.withValues(alpha: 0.12),
    );
    final spans = _highlightWordSpans(
      sentence,
      word,
      baseStyle,
      highlightStyle,
    );
    return RichText(
      text: TextSpan(style: baseStyle, children: spans),
    );
  }

  List<TextSpan> _highlightWordSpans(
    String sentence,
    String word,
    TextStyle? baseStyle,
    TextStyle? highlightStyle,
  ) {
    final target = word.trim();
    if (sentence.isEmpty || target.isEmpty) {
      return [TextSpan(text: sentence, style: baseStyle)];
    }

    final pattern = RegExp(
      r'(?<![A-Za-z])' + RegExp.escape(target) + r'(?![A-Za-z])',
      caseSensitive: false,
    );
    final matches = pattern.allMatches(sentence).toList(growable: false);
    if (matches.isEmpty) return [TextSpan(text: sentence, style: baseStyle)];

    final spans = <TextSpan>[];
    var cursor = 0;
    for (final match in matches) {
      if (match.start > cursor) {
        spans.add(
          TextSpan(
            text: sentence.substring(cursor, match.start),
            style: baseStyle,
          ),
        );
      }
      spans.add(
        TextSpan(
          text: sentence.substring(match.start, match.end),
          style: highlightStyle,
        ),
      );
      cursor = match.end;
    }
    if (cursor < sentence.length) {
      spans.add(TextSpan(text: sentence.substring(cursor), style: baseStyle));
    }
    return spans;
  }

  WrongWordEntry? _selectedEntry() {
    final selectedId = _selectedId;
    if (selectedId == null) return null;
    for (final entry in _words) {
      if (entry.entryId == selectedId) return entry;
    }
    return null;
  }

  Widget _buildDetailSection() {
    final selectedEntry = _selectedEntry();
    final detailErrorCount = _detail?.errorCount ?? 0;
    final listErrorCount = selectedEntry?.errorCount ?? 0;
    final effectiveErrorCount = detailErrorCount > listErrorCount
        ? detailErrorCount
        : listErrorCount;
    final detailCanEditHint =
        _detail != null && (_detail!.hasHint || effectiveErrorCount >= 5);
    return _SectionCard(
      title: selectedEntry?.isRootAffix == true ? '词根词缀详情' : '词条详情',
      subtitle: '查看释义、题型风险、错误历史和例句。',
      child: _loadingDetail
          ? const Padding(
              padding: EdgeInsets.symmetric(vertical: 24),
              child: CrocodileLoadingAnimation(
                label: '加载中...',
                width: 120,
                height: 92,
              ),
            )
          : _detail == null
          ? const Text('暂无详情')
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _detail!.word,
                  style: Theme.of(context).textTheme.headlineSmall,
                ),
                const SizedBox(height: 8),
                if ((_detail!.phoneticUs ?? _detail!.phoneticUk) != null)
                  Text(
                    _detail!.phoneticUs ?? _detail!.phoneticUk ?? '',
                    style: Theme.of(
                      context,
                    ).textTheme.bodyMedium?.copyWith(color: Colors.black54),
                  ),
                if (_detail!.partOfSpeech.isNotEmpty) ...[
                  const SizedBox(height: 4),
                  Text('词性：${_detail!.partOfSpeech}'),
                ],
                if (_detail!.meanings.isNotEmpty) ...[
                  const SizedBox(height: 12),
                  Text(
                    '释义：${_detail!.meanings.map((item) => item is Map<String, dynamic> ? item['meaningCn'] ?? '' : item.toString()).where((item) => '$item'.trim().isNotEmpty).join(' / ')}',
                  ),
                ],
                const SizedBox(height: 16),
                Text('题型情况', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                if (detailCanEditHint) ...[
                  _HintDetailBox(
                    hint: _detail!.userHint,
                    hasSuggestions: _detail!.hintSuggestions.isNotEmpty,
                    onEdit: () => _editHint(
                      _detail!,
                      effectiveErrorCount: effectiveErrorCount,
                    ),
                  ),
                  const SizedBox(height: 16),
                ],
                if (_detail!.riskBreakdown.isEmpty)
                  const Text('暂无分题型数据')
                else
                  for (final item in _detail!.riskBreakdown)
                    if (item is Map<String, dynamic>)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Text(
                          '${_questionTypeLabel('${item['questionType'] ?? ''}')}：${(item['incorrect'] ?? 0) + (item['skipped'] ?? 0)}/${item['attempts'] ?? 0} 次出错',
                        ),
                      ),
                const SizedBox(height: 16),
                Text('最近错误', style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                if (_detail!.errorHistory.isEmpty)
                  const Text('暂无错误历史')
                else
                  for (final item in _detail!.errorHistory)
                    if (item is Map<String, dynamic>)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 6),
                        child: Text(
                          '${_historyDate('${item['date'] ?? ''}')}  ${_historyContextLabel('${item['context'] ?? ''}')}',
                        ),
                      ),
                if (_detail!.examples.isNotEmpty) ...[
                  const SizedBox(height: 16),
                  Text('例句', style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 8),
                  for (final item in _detail!.examples)
                    if (item is Map<String, dynamic>)
                      Padding(
                        padding: const EdgeInsets.only(bottom: 10),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            _highlightedExampleSentence(
                              '${item['sentenceEn'] ?? ''}',
                              _detail!.word,
                            ),
                            const SizedBox(height: 4),
                            Text(
                              '${item['sentenceCn'] ?? ''}',
                              style: Theme.of(context).textTheme.bodySmall
                                  ?.copyWith(color: Colors.black54),
                            ),
                          ],
                        ),
                      ),
                ],
                const SizedBox(height: 8),
                Align(
                  alignment: Alignment.centerLeft,
                  child: FilledButton.tonalIcon(
                    onPressed: _scrollToSelectedWord,
                    icon: const Icon(Icons.arrow_upward_rounded),
                    label: const Text('返回所选错词'),
                  ),
                ),
              ],
            ),
    );
  }

  Widget _buildWrongEntrySection({
    required String title,
    required String subtitle,
    required List<WrongWordEntry> entries,
  }) {
    return _SectionCard(
      title: title,
      subtitle: subtitle,
      child: Column(
        children: entries
            .expand<Widget>(
              (entry) => [
                KeyedSubtree(
                  key: _keyForEntry(entry.entryId),
                  child: _WrongWordTile(
                    entry: entry,
                    selected: _selectedId == entry.entryId,
                    primaryMeaning: _primaryMeaning(entry.meanings),
                    onTap: () => _selectWord(entry),
                  ),
                ),
                if (_selectedId == entry.entryId) ...[
                  SizedBox(key: _detailAnchorKey, height: 1),
                  _buildDetailSection(),
                ],
              ],
            )
            .toList(growable: false),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final totalErrors = _words.fold<int>(
      0,
      (sum, item) => sum + item.errorCount,
    );
    final wordEntries = _words
        .where((entry) => !entry.isRootAffix)
        .toList(growable: false);
    final rootAffixEntries = _words
        .where((entry) => entry.isRootAffix)
        .toList(growable: false);
    final averagePriority = _words.isEmpty
        ? '0.0'
        : (_words.fold<double>(0, (sum, item) => sum + item.priorityScore) /
                  _words.length)
              .toStringAsFixed(1);

    return Scaffold(
      appBar: AppBar(
        title: const Text('\u9519\u8bcd\u672c'),
        actions: [
          IconButton(
            tooltip: 'Wrong word graph',
            onPressed: () {
              Navigator.of(context).push(
                MaterialPageRoute(
                  builder: (_) => WrongWordGraphScreen(sdk: widget.sdk),
                ),
              );
            },
            icon: const Icon(Icons.hub_outlined),
          ),
        ],
      ),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '加载中...')
          : _error != null
          ? _WrongWordsMessage(message: _error!, onRetry: _loadAll)
          : CrocodileRefreshIndicator(
              onRefresh: () => _loadAll(showFullLoading: false),
              child: ListView(
                controller: _scrollController,
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
                            '错词本',
                            style: Theme.of(context).textTheme.headlineSmall
                                ?.copyWith(
                                  color: Colors.white,
                                  fontWeight: FontWeight.w700,
                                ),
                          ),
                          const SizedBox(height: 8),
                          Text(
                            '这里会聚合历史和今天暴露出来的薄弱点，并给出强化优先级。',
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
                                label: '错词数',
                                value: '${wordEntries.length}',
                              ),
                              _StatPill(
                                label: '词根词缀',
                                value: '${rootAffixEntries.length}',
                              ),
                              _StatPill(label: '累计错误', value: '$totalErrors'),
                              _StatPill(label: '平均优先级', value: averagePriority),
                            ],
                          ),
                        ],
                      ),
                    ),
                  ),
                  _SectionCard(
                    title: '筛选',
                    subtitle: '先缩小范围，再打开词条详情看风险拆解和错误记录。',
                    child: Wrap(
                      spacing: 10,
                      runSpacing: 10,
                      children: _filters.entries
                          .map(
                            (entry) => ChoiceChip(
                              label: Text(entry.value),
                              selected: _filter == entry.key,
                              onSelected: (_) {
                                setState(() {
                                  _filter = entry.key;
                                });
                                _loadAll();
                              },
                            ),
                          )
                          .toList(growable: false),
                    ),
                  ),
                  if (_words.isNotEmpty)
                    _SectionCard(
                      title: '强化入口',
                      subtitle: '按当前筛选结果进入错词强化，优先回收高风险词条。',
                      child: FilledButton.tonal(
                        onPressed: _startReinforcement,
                        child: Text(
                          '开始错词强化（最多 ${_words.length.clamp(0, 20)} 个）',
                        ),
                      ),
                    ),
                  if (_words.isEmpty)
                    const _SectionCard(
                      title: '当前为空',
                      subtitle: '错词会在学习过程中逐步累积到这里。',
                      child: Text('还没有错词，先去今日页开始学习。'),
                    ),
                  if (wordEntries.isNotEmpty)
                    _buildWrongEntrySection(
                      title: '错词列表',
                      subtitle: '普通单词的错误记录；点击某个词条展开详情，再次点击可收起。',
                      entries: wordEntries,
                    ),
                  if (rootAffixEntries.isNotEmpty)
                    _buildWrongEntrySection(
                      title: '词根词缀错题',
                      subtitle: '词根、前缀、后缀和词缀题的错误记录单独归档。',
                      entries: rootAffixEntries,
                    ),
                ],
              ),
            ),
    );
  }
}

class _WrongWordTile extends StatelessWidget {
  const _WrongWordTile({
    required this.entry,
    required this.selected,
    required this.primaryMeaning,
    required this.onTap,
  });

  final WrongWordEntry entry;
  final bool selected;
  final String primaryMeaning;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final priorityColor = entry.priorityScore >= 8
        ? const Color(0xFFD64545)
        : entry.priorityScore >= 5
        ? const Color(0xFFE69229)
        : const Color(0xFF2F8F6A);

    final colorScheme = Theme.of(context).colorScheme;
    final needsHint = entry.errorCount >= 5 && !entry.hasHint;
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      color: selected ? colorScheme.primary.withValues(alpha: 0.10) : null,
      child: Stack(
        children: [
          ListTile(
            contentPadding: EdgeInsetsDirectional.fromSTEB(
              needsHint ? 36 : 16,
              8,
              16,
              8,
            ),
            onTap: onTap,
            title: Text(entry.word),
            subtitle: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if ((entry.phoneticUs ?? entry.phoneticUk) != null)
                  Text(entry.phoneticUs ?? entry.phoneticUk ?? ''),
                Text(primaryMeaning),
                if (entry.hasHint) const Text('提示词  •••'),
                Text(
                  '错误 ${entry.errorCount} 次 · 优先级 ${entry.priorityScore.toStringAsFixed(1)}',
                ),
              ],
            ),
            trailing: Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              decoration: BoxDecoration(
                color: priorityColor.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(999),
              ),
              child: Text(
                entry.priorityScore.toStringAsFixed(1),
                style: TextStyle(
                  color: priorityColor,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ),
          if (needsHint)
            const PositionedDirectional(
              start: 16,
              top: 16,
              child: _HintReminderDot(),
            ),
        ],
      ),
    );
  }
}

class _HintReminderDot extends StatelessWidget {
  const _HintReminderDot();

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: const BoxDecoration(
        color: Color(0xFFD64545),
        shape: BoxShape.circle,
      ),
      child: SizedBox(width: 8, height: 8),
    );
  }
}

class _HintDetailBox extends StatefulWidget {
  const _HintDetailBox({
    required this.hint,
    required this.hasSuggestions,
    required this.onEdit,
  });

  final String? hint;
  final bool hasSuggestions;
  final VoidCallback onEdit;

  @override
  State<_HintDetailBox> createState() => _HintDetailBoxState();
}

class _HintDetailBoxState extends State<_HintDetailBox> {
  bool _revealed = false;

  @override
  Widget build(BuildContext context) {
    final hint = widget.hint?.trim();
    final hasHint = hint != null && hint.isNotEmpty;
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colorScheme.primary.withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: colorScheme.primary.withValues(alpha: 0.18)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.lightbulb_outline, color: colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '提示词',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                TextButton(
                  onPressed: widget.onEdit,
                  child: Text(hasHint ? '修改' : '添加'),
                ),
              ],
            ),
            const SizedBox(height: 6),
            if (hasHint)
              InkWell(
                onTap: () {
                  setState(() {
                    _revealed = !_revealed;
                  });
                },
                child: Text(_revealed ? hint : '•••••• 点击查看'),
              )
            else
              Text(widget.hasSuggestions ? '有 AI 推荐提示可选' : '还没有提示词'),
          ],
        ),
      ),
    );
  }
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

class _WrongWordsMessage extends StatelessWidget {
  const _WrongWordsMessage({required this.message, required this.onRetry});

  final String message;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(message, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton(onPressed: onRetry, child: const Text('重试')),
          ],
        ),
      ),
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

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';

class AiScreen extends StatefulWidget {
  const AiScreen({
    super.key,
    required this.sdk,
    this.generateOnOpen = false,
    this.showPassageFirst = false,
  });

  final WordSdk sdk;
  final bool generateOnOpen;
  final bool showPassageFirst;

  @override
  State<AiScreen> createState() => _AiScreenState();
}

class _AiScreenState extends State<AiScreen> {
  TodayAiPassageContext? _context;
  List<AiPassageHistoryItem> _history = const [];
  AiPassage? _passage;
  String? _message;
  bool _loading = true;
  bool _generating = false;
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
      AiPassage? latest;
      if (history.isNotEmpty) {
        latest = await widget.sdk.ai.getAiPassage(history.first.passageId);
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
    final context = _context;
    if (context == null) return;

    final wrongWords = context.generationWrongWords
        .take(6)
        .toList(growable: false);
    final targetWords = context.generationTargetWords
        .take(6)
        .toList(growable: false);

    if (!context.tasksComplete) {
      setState(() {
        _message = '请先完成今日任务，再生成 AI 短文。';
      });
      return;
    }

    if (wrongWords.isEmpty && targetWords.isEmpty) {
      setState(() {
        _message = '当前没有可用于生成短文的错词。';
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
      );
      setState(() {
        _passage = generated;
      });
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

  Widget _buildPassageSection(BuildContext context) {
    return _SectionCard(
      title: 'å½“å‰æ­£æ–‡',
      subtitle:
          'ä¼˜å…ˆçœ‹æœ€è¿‘ä¸€ç¯‡ AI çŸ­æ–‡çš„æ ‡é¢˜ã€çŠ¶æ€å’Œæ­£æ–‡å—ã€‚',
      child: _passage == null
          ? const Text('è¿˜æ²¡æœ‰å¯å±•ç¤ºçš„ AI çŸ­æ–‡ã€‚')
          : Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  _passage!.title,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                const SizedBox(height: 8),
                Text('çŠ¶æ€ï¼š${_passage!.validationStatus}'),
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

    return Scaffold(
      appBar: AppBar(title: const Text('AI passages')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Card(
                  margin: const EdgeInsets.only(bottom: 16),
                  color: const Color(0xFF1F6F5E),
                  child: Padding(
                    padding: const EdgeInsets.all(20),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'AI 阅读',
                          style: Theme.of(context).textTheme.headlineSmall
                              ?.copyWith(
                                color: Colors.white,
                                fontWeight: FontWeight.w700,
                              ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          '和 RN 一样，AI 页面应该先展示 today context、历史与正文，而不是直接暴露调试生成入口。',
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
                              label: '任务完成',
                              value:
                                  '${contextPayload?.tasksComplete ?? false}',
                            ),
                            _StatPill(
                              label: '今日错词',
                              value:
                                  '${contextPayload?.wrongWords.length ?? 0}',
                            ),
                            _StatPill(
                              label: '历史篇数',
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
                  title: '生成入口',
                  subtitle: '只有在今日任务完成且存在错词时，才应该鼓励生成。',
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        contextPayload == null
                            ? '尚未读取到 today context。'
                            : contextPayload.tasksComplete
                            ? '今日任务已完成，可以尝试生成新的 AI 短文。'
                            : '先完成今日学习任务，再进入 AI 短文。',
                      ),
                      const SizedBox(height: 8),
                      Text('今日错词数：${contextPayload?.wrongWords.length ?? 0}'),
                      const SizedBox(height: 12),
                      FilledButton(
                        onPressed: _generating ? null : _generate,
                        child: Text(_generating ? '生成中...' : '生成新的 AI 短文'),
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
                  _SectionCard(
                    title: '当前正文',
                    subtitle: '优先看最近一篇 AI 短文的标题、状态和正文块。',
                    child: _passage == null
                        ? const Text('还没有可展示的 AI 短文。')
                        : Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(
                                _passage!.title,
                                style: Theme.of(context).textTheme.titleLarge,
                              ),
                              const SizedBox(height: 8),
                              Text('状态：${_passage!.validationStatus}'),
                              if (_passage!.failureReason != null) ...[
                                const SizedBox(height: 8),
                                Text(
                                  _passage!.failureReason!,
                                  style: const TextStyle(
                                    color: Colors.redAccent,
                                  ),
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
                  ),
                _SectionCard(
                  title: '历史记录',
                  subtitle: '点击历史项切换查看，不再把标题数组直接打印给用户。',
                  child: _history.isEmpty
                      ? const Text('暂无历史记录。')
                      : Column(
                          children: _history
                              .map((item) {
                                final selected =
                                    _passage?.passageId == item.passageId;
                                return Card(
                                  margin: const EdgeInsets.only(bottom: 10),
                                  color: selected
                                      ? const Color(0xFFEAF4F2)
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
                  return TextSpan(
                    children: [
                      TextSpan(
                        text: text,
                        style: const TextStyle(
                          color: Color(0xFF1F6F5E),
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

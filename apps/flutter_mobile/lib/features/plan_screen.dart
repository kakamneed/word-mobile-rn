import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'shell_page_data_cache.dart';

const int planWrongWordTargetMax = 200;

int clampPlanTargetValue(int value, int min, int max) =>
    value < min ? min : (value > max ? max : value);

int parseWrongWordPlanTargetForTest(String text, int fallback) =>
    clampPlanTargetValue(
      int.tryParse(text.trim()) ?? fallback,
      0,
      planWrongWordTargetMax,
    );

class PlanScreen extends StatefulWidget {
  const PlanScreen({
    super.key,
    required this.sdk,
    this.dataCache,
    this.onTodayPlanApplied,
    this.onDirtyChanged,
  });

  final WordSdk sdk;
  final ShellPageDataCache? dataCache;
  final VoidCallback? onTodayPlanApplied;
  final ValueChanged<bool>? onDirtyChanged;

  @override
  State<PlanScreen> createState() => _PlanScreenState();
}

class _PlanScreenState extends State<PlanScreen> {
  final ScrollController _scrollController = ScrollController();

  bool _loading = true;
  bool _saving = false;
  bool _applying = false;
  bool _suppressDirtyNotifications = false;
  bool _lastDirtyState = false;
  PlanSummary? _plan;
  List<WordbookSummary> _wordbooks = const [];
  int? _pendingWordbookId;
  String? _error;

  late final TextEditingController _nameController;
  late final TextEditingController _newWordsController;
  late final TextEditingController _reviewWordsController;
  late final TextEditingController _mixedController;
  late final TextEditingController _wrongWordsController;
  late final TextEditingController _highFrequencyController;
  late final TextEditingController _rootAffixController;
  late final TextEditingController _growthIntervalController;
  late final TextEditingController _growthIncrementController;
  final Map<String, TextEditingController> _perModeIntervalControllers = {};
  final Map<String, TextEditingController> _perModeIncrementControllers = {};

  bool _growthRuleEnabled = true;
  String _growthRuleMode = 'shared';

  static const List<String> _growthModes = [
    'newWord',
    'review',
    'mixedTest',
    'wrongWordReinforcement',
    'highFrequency',
    'rootAffix',
  ];

  @override
  void initState() {
    super.initState();
    _nameController = TextEditingController();
    _newWordsController = TextEditingController();
    _reviewWordsController = TextEditingController();
    _mixedController = TextEditingController();
    _wrongWordsController = TextEditingController();
    _highFrequencyController = TextEditingController();
    _rootAffixController = TextEditingController();
    _growthIntervalController = TextEditingController();
    _growthIncrementController = TextEditingController();
    for (final mode in _growthModes) {
      _perModeIntervalControllers[mode] = TextEditingController();
      _perModeIncrementControllers[mode] = TextEditingController();
    }
    for (final controller in _dirtyTrackedControllers) {
      controller.addListener(_notifyDirtyChanged);
    }
    _load();
  }

  @override
  void dispose() {
    _scrollController.dispose();
    _nameController.dispose();
    _newWordsController.dispose();
    _reviewWordsController.dispose();
    _mixedController.dispose();
    _wrongWordsController.dispose();
    _highFrequencyController.dispose();
    _rootAffixController.dispose();
    _growthIntervalController.dispose();
    _growthIncrementController.dispose();
    for (final controller in _perModeIntervalControllers.values) {
      controller.dispose();
    }
    for (final controller in _perModeIncrementControllers.values) {
      controller.dispose();
    }
    super.dispose();
  }

  List<TextEditingController> get _dirtyTrackedControllers => [
    _nameController,
    _newWordsController,
    _reviewWordsController,
    _mixedController,
    _wrongWordsController,
    _highFrequencyController,
    _rootAffixController,
    _growthIntervalController,
    _growthIncrementController,
    ..._perModeIntervalControllers.values,
    ..._perModeIncrementControllers.values,
  ];

  bool get _hasUnsavedChanges {
    final plan = _plan;
    if (plan == null) return false;
    if (_nameController.text.trim() != plan.name) return true;
    if (_newWordsController.text != '${plan.newWordsPerDay}') return true;
    if (_reviewWordsController.text != '${plan.reviewWordsPerDay}') return true;
    if (_mixedController.text != '${plan.mixedTestPerDay}') return true;
    if (_wrongWordsController.text != '${plan.wrongWordTestPerDay}') {
      return true;
    }
    if (_highFrequencyController.text != '${plan.highFrequencyPerDay}') {
      return true;
    }
    if (_rootAffixController.text != '${plan.rootAffixPerDay ?? 0}') {
      return true;
    }
    if (_growthRuleEnabled != plan.growthRuleEnabled) return true;
    if (_growthRuleMode != plan.growthRuleMode) return true;
    if (_hasWordbookChange) return true;
    if (_growthIntervalController.text != '${plan.growthIntervalDays}') {
      return true;
    }
    if (_growthIncrementController.text != '${plan.growthIncrement}') {
      return true;
    }

    for (final mode in _growthModes) {
      final rule = _ruleForMode(mode, plan);
      if (_perModeIntervalControllers[mode]!.text != '${rule.intervalDays}') {
        return true;
      }
      if (_perModeIncrementControllers[mode]!.text != '${rule.increment}') {
        return true;
      }
    }
    return false;
  }

  int? get _savedWordbookId {
    for (final wordbook in _wordbooks) {
      if (wordbook.isActive) return wordbook.id;
    }
    return null;
  }

  bool get _hasWordbookChange => _pendingWordbookId != _savedWordbookId;

  void _notifyDirtyChanged() {
    if (_suppressDirtyNotifications) return;
    final dirty = _hasUnsavedChanges;
    if (dirty == _lastDirtyState) return;
    _lastDirtyState = dirty;
    if (mounted) {
      setState(() {});
    }
    widget.onDirtyChanged?.call(dirty);
  }

  Future<void> _load({bool showFullLoading = true}) async {
    setState(() {
      if (showFullLoading) _loading = true;
      _error = null;
    });
    try {
      final planFuture =
          widget.dataCache?.loadActivePlan(refresh: showFullLoading) ??
          widget.sdk.plan.getActivePlan();
      final wordbooksFuture =
          widget.dataCache?.loadWordbooks(refresh: showFullLoading) ??
          widget.sdk.plan.getWordbooks();
      final plan = await planFuture;
      final wordbooks = await wordbooksFuture;
      if (!mounted) return;
      _suppressDirtyNotifications = true;
      setState(() {
        _plan = plan;
        _wordbooks = wordbooks;
        _pendingWordbookId = _activeWordbookId(wordbooks);
      });
      if (plan != null) {
        _hydrateControllers(plan);
      }
      _suppressDirtyNotifications = false;
      _notifyDirtyChanged();
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

  void _hydrateControllers(PlanSummary plan) {
    _nameController.text = plan.name;
    _newWordsController.text = '${plan.newWordsPerDay}';
    _reviewWordsController.text = '${plan.reviewWordsPerDay}';
    _mixedController.text = '${plan.mixedTestPerDay}';
    _wrongWordsController.text = '${plan.wrongWordTestPerDay}';
    _highFrequencyController.text = '${plan.highFrequencyPerDay}';
    _rootAffixController.text = '${plan.rootAffixPerDay ?? 0}';
    _growthRuleEnabled = plan.growthRuleEnabled;
    _growthRuleMode = plan.growthRuleMode;
    _growthIntervalController.text = '${plan.growthIntervalDays}';
    _growthIncrementController.text = '${plan.growthIncrement}';
    for (final mode in _growthModes) {
      final rule = _ruleForMode(mode, plan);
      _perModeIntervalControllers[mode]!.text = '${rule.intervalDays}';
      _perModeIncrementControllers[mode]!.text = '${rule.increment}';
    }
  }

  _GrowthRule _ruleForMode(String mode, PlanSummary plan) {
    final raw = (plan.growthRulesByMode ?? const <String, dynamic>{})[mode];
    if (raw is Map<String, dynamic>) {
      return _GrowthRule(
        intervalDays: _clamp(
          (raw['intervalDays'] as num?)?.toInt() ?? plan.growthIntervalDays,
          1,
          365,
        ),
        increment: _clamp(
          (raw['increment'] as num?)?.toInt() ?? plan.growthIncrement,
          0,
          100,
        ),
      );
    }
    return _GrowthRule(
      intervalDays: plan.growthIntervalDays,
      increment: plan.growthIncrement,
    );
  }

  int _parseController(
    TextEditingController controller,
    int fallback,
    int min,
    int max,
  ) {
    return _clamp(int.tryParse(controller.text.trim()) ?? fallback, min, max);
  }

  int _parseNewWordQuestions(PlanSummary plan) {
    final parsed = _parseController(
      _newWordsController,
      plan.newWordsPerDay,
      0,
      180,
    );
    return parsed - (parsed % 4);
  }

  int _clamp(int value, int min, int max) =>
      clampPlanTargetValue(value, min, max);

  int? get _selectedWordbookId {
    return _pendingWordbookId ?? _savedWordbookId;
  }

  int? _activeWordbookId(List<WordbookSummary> wordbooks) {
    for (final wordbook in wordbooks) {
      if (wordbook.isActive) return wordbook.id;
    }
    return null;
  }

  Map<String, dynamic> _buildPlanInput(PlanSummary plan) {
    final sharedInterval = _parseController(
      _growthIntervalController,
      plan.growthIntervalDays,
      1,
      365,
    );
    final sharedIncrement = _parseController(
      _growthIncrementController,
      plan.growthIncrement,
      0,
      100,
    );
    final rulesByMode = <String, dynamic>{};
    for (final mode in _growthModes) {
      rulesByMode[mode] = {
        'intervalDays': _parseController(
          _perModeIntervalControllers[mode]!,
          sharedInterval,
          1,
          365,
        ),
        'increment': _parseController(
          _perModeIncrementControllers[mode]!,
          sharedIncrement,
          0,
          100,
        ),
      };
    }
    return <String, dynamic>{
      'name': _nameController.text.trim().isEmpty
          ? plan.name
          : _nameController.text.trim(),
      'newWordsPerDay': _parseNewWordQuestions(plan),
      'reviewWordsPerDay': _parseController(
        _reviewWordsController,
        plan.reviewWordsPerDay,
        0,
        200,
      ),
      'mixedTestPerDay': _parseController(
        _mixedController,
        plan.mixedTestPerDay,
        0,
        50,
      ),
      'wrongWordTestPerDay': _parseController(
        _wrongWordsController,
        plan.wrongWordTestPerDay,
        0,
        planWrongWordTargetMax,
      ),
      'highFrequencyPerDay': _parseController(
        _highFrequencyController,
        plan.highFrequencyPerDay,
        0,
        100,
      ),
      'rootAffixPerDay': _parseController(
        _rootAffixController,
        plan.rootAffixPerDay ?? 0,
        0,
        50,
      ),
      'growthRuleEnabled': _growthRuleEnabled,
      'growthRuleMode': _growthRuleMode,
      'growthIntervalDays': sharedInterval,
      'growthIncrement': sharedIncrement,
      'sharedGrowthRule': {
        'intervalDays': sharedInterval,
        'increment': sharedIncrement,
      },
      'growthRulesByMode': rulesByMode,
      'questionTypeWeightsByMode':
          plan.questionTypeWeightsByMode ?? const <String, dynamic>{},
    };
  }

  Future<PlanSummary> _persistPlanAndWordbook(PlanSummary plan) async {
    final selectedWordbookId = _selectedWordbookId;
    if (selectedWordbookId != null && selectedWordbookId != _savedWordbookId) {
      await widget.sdk.plan.toggleWordbook(
        wordbookId: selectedWordbookId,
        isActive: true,
      );
    }
    return widget.sdk.plan.savePlan(
      planId: plan.id,
      input: _buildPlanInput(plan),
    );
  }

  Future<void> _reloadWordbooksAfterPersist() async {
    final wordbooks = await widget.sdk.plan.getWordbooks();
    if (!mounted) return;
    setState(() {
      _wordbooks = wordbooks;
      _pendingWordbookId = _activeWordbookId(wordbooks);
    });
  }

  Future<void> _save() async {
    final plan = _plan;
    if (plan == null) return;
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final saved = await _persistPlanAndWordbook(plan);
      if (!mounted) return;
      _suppressDirtyNotifications = true;
      _hydrateControllers(saved);
      setState(() {
        _plan = saved;
      });
      await _reloadWordbooksAfterPersist();
      if (!mounted) return;
      _suppressDirtyNotifications = false;
      _notifyDirtyChanged();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('计划已保存，默认从明天开始生效')));
      final applyToday = await _showTodayApplyDialog(
        title: '计划已保存',
        message: '这份计划要立刻应用到今天，还是从明天开始生效？',
        confirmLabel: '应用到今天',
        cancelLabel: '明天生效',
      );
      if (!mounted) return;
      if (applyToday == true) {
        await _applyToToday(showToast: false);
      } else {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('计划已保存，将从后续学习中生效')));
      }
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _saving = false;
        });
      }
    }
  }

  Future<void> _applyToToday({bool showToast = true}) async {
    final plan = _plan;
    if (plan == null) return;
    setState(() {
      _applying = true;
      _error = null;
    });
    try {
      await _persistPlanAndWordbook(plan);
      final applied = await widget.sdk.plan.applySavedPlanToToday();
      if (!mounted) return;
      _suppressDirtyNotifications = true;
      setState(() {
        _plan = applied;
      });
      _hydrateControllers(applied);
      await _reloadWordbooksAfterPersist();
      if (!mounted) return;
      _suppressDirtyNotifications = false;
      _notifyDirtyChanged();
      widget.onTodayPlanApplied?.call();
      if (showToast) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('已同步计划到 Today')));
      }
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _applying = false;
        });
      }
    }
  }

  Future<void> _toggleWordbook(WordbookSummary wordbook, bool nextValue) async {
    FocusManager.instance.primaryFocus?.unfocus();
    final offset = _scrollController.hasClients
        ? _scrollController.offset
        : null;
    setState(() {
      _error = null;
      _pendingWordbookId = nextValue ? wordbook.id : _savedWordbookId;
    });
    _notifyDirtyChanged();
    if (offset != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!_scrollController.hasClients) return;
        final position = _scrollController.position;
        final restored = offset.clamp(
          position.minScrollExtent,
          position.maxScrollExtent,
        );
        _scrollController.jumpTo(restored);
      });
    }
    if (wordbook.id < 0) {
      try {
        await widget.sdk.plan.toggleWordbook(
          wordbookId: wordbook.id,
          isActive: nextValue,
        );
        await _load();
        if (!mounted) return;
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('词书选择已保存，默认从明天开始生效')));
        final applyToday = await _showTodayApplyDialog(
          title: '词书选择已保存',
          message: '要把新的词书选择应用到今天吗？应用到今天会从当前计划重新生成今日任务。',
          confirmLabel: '应用到今天',
          cancelLabel: '明天生效',
        );
        if (applyToday == true) {
          await _applyToToday(showToast: false);
        }
      } catch (error) {
        if (!mounted) return;
        setState(() {
          _error = error.toString();
        });
      }
    }
  }

  Future<bool?> _showTodayApplyDialog({
    required String title,
    required String message,
    required String confirmLabel,
    required String cancelLabel,
  }) {
    return Future<bool?>.value(false);
  }

  @override
  Widget build(BuildContext context) {
    final plan = _plan;

    return Scaffold(
      appBar: AppBar(title: const Text('计划')),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '加载中...')
          : _error != null
          ? _PlanMessage(message: _error!, onRetry: _load)
          : plan == null
          ? _PlanMessage(message: '当前没有可编辑的计划。', onRetry: _load)
          : CrocodileRefreshIndicator(
              onRefresh: () => _load(showFullLoading: false),
              child: ListView(
                controller: _scrollController,
                physics: const AlwaysScrollableScrollPhysics(),
                padding: const EdgeInsets.all(16),
                children: [
                  _PlanHeroCard(plan: plan, wordbooks: _wordbooks),
                  if (_hasUnsavedChanges)
                    const Padding(
                      padding: EdgeInsets.only(bottom: 16),
                      child: _DirtyBanner(),
                    ),
                  _SectionCard(
                    title: '计划名称',
                    subtitle: '计划会同时影响今日、学习和报告的展示口径。',
                    child: TextField(
                      controller: _nameController,
                      maxLength: 50,
                      decoration: const InputDecoration(
                        border: OutlineInputBorder(),
                        labelText: '计划名称',
                      ),
                    ),
                  ),
                  _SectionCard(
                    title: '每日目标',
                    subtitle: '和 RN 一样，首页与学习流程统一按题量展示进度。新词、复习按每词 4 题换算。',
                    child: Column(
                      children: [
                        _StepperField(
                          label: '新词学习',
                          helper: '单位：题；需为 4 的倍数，对应四种固定题型',
                          controller: _newWordsController,
                          step: 4,
                          max: 180,
                        ),
                        const SizedBox(height: 12),
                        _StepperField(
                          label: '复习',
                          helper: '单位：题；按人格推荐题型占比分配',
                          controller: _reviewWordsController,
                          step: 5,
                          max: 200,
                        ),
                        const SizedBox(height: 12),
                        _StepperField(
                          label: '混合测试',
                          helper: '按题推进',
                          controller: _mixedController,
                          step: 5,
                          max: 50,
                        ),
                        const SizedBox(height: 12),
                        _StepperField(
                          label: '错词强化',
                          helper: '按题推进',
                          controller: _wrongWordsController,
                          step: 5,
                          max: planWrongWordTargetMax,
                        ),
                        const SizedBox(height: 12),
                        _StepperField(
                          label: '高频词',
                          helper: '从考研英一真题频次池中随机抽题',
                          controller: _highFrequencyController,
                          step: 5,
                          max: 100,
                        ),
                        const SizedBox(height: 12),
                        _StepperField(
                          label: '词根词缀',
                          helper:
                              '${int.tryParse(_rootAffixController.text) ?? (plan.rootAffixPerDay ?? 0)} 题',
                          controller: _rootAffixController,
                          step: 1,
                          max: 50,
                        ),
                      ],
                    ),
                  ),
                  _SectionCard(
                    title: '增长规则',
                    subtitle: '支持共享规则和分模式规则。保存后可以决定今天是否立刻采用新节奏。',
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SwitchListTile(
                          contentPadding: EdgeInsets.zero,
                          title: const Text(
                            '\u542f\u7528\u589e\u957f\u63a8\u8350',
                          ),
                          subtitle: const Text(
                            '\u5f00\u542f\u540e\u4f1a\u6309\u95f4\u9694\u5929\u6570\u9010\u6b65\u589e\u52a0\u6bcf\u65e5\u8ba1\u5212\u91cf',
                          ),
                          value: _growthRuleEnabled,
                          onChanged: (value) {
                            setState(() => _growthRuleEnabled = value);
                            _notifyDirtyChanged();
                          },
                        ),
                        const SizedBox(height: 8),
                        Wrap(
                          spacing: 10,
                          runSpacing: 10,
                          children: [
                            _ModeToggleChip(
                              label: '全部共享',
                              selected: _growthRuleMode == 'shared',
                              onTap: () {
                                setState(() => _growthRuleMode = 'shared');
                                _notifyDirtyChanged();
                              },
                            ),
                            _ModeToggleChip(
                              label: '分别设置',
                              selected: _growthRuleMode == 'perMode',
                              onTap: () {
                                setState(() => _growthRuleMode = 'perMode');
                                _notifyDirtyChanged();
                              },
                            ),
                          ],
                        ),
                        const SizedBox(height: 16),
                        _RuleEditorCard(
                          title: '统一增长规则',
                          subtitle: '所有模式共用：每 N 天增加 M 个计划单位。',
                          intervalController: _growthIntervalController,
                          incrementController: _growthIncrementController,
                        ),
                        if (_growthRuleMode == 'perMode') ...[
                          const SizedBox(height: 12),
                          for (final mode in _growthModes) ...[
                            _RuleEditorCard(
                              title: _growthModeLabel(mode),
                              subtitle: '该模式单独设置增长节奏。',
                              intervalController:
                                  _perModeIntervalControllers[mode]!,
                              incrementController:
                                  _perModeIncrementControllers[mode]!,
                            ),
                            const SizedBox(height: 12),
                          ],
                        ],
                      ],
                    ),
                  ),
                  _SectionCard(
                    title: '词书管理',
                    subtitle: '保持和 RN 一样：词书选择先保存，再决定是否把变化应用到今天。',
                    child: Column(
                      children: _wordbooks
                          .map(
                            (wordbook) => ListTile(
                              contentPadding: EdgeInsets.zero,
                              title: Text(wordbook.name),
                              subtitle: Text(
                                '${wordbook.totalEntries} 词 · ${wordbook.category}',
                              ),
                              // ignore: deprecated_member_use
                              leading: Radio<int>(
                                value: wordbook.id,
                                // ignore: deprecated_member_use
                                groupValue: _selectedWordbookId,
                                // ignore: deprecated_member_use
                                onChanged: (_) =>
                                    _toggleWordbook(wordbook, true),
                              ),
                              onTap: () => _toggleWordbook(wordbook, true),
                            ),
                          )
                          .toList(growable: false),
                    ),
                  ),
                  Row(
                    children: [
                      Expanded(
                        child: FilledButton(
                          onPressed: _saving ? null : _save,
                          child: Text(_saving ? '保存中...' : '保存计划'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: OutlinedButton(
                          onPressed: _applying ? null : _applyToToday,
                          child: Text(_applying ? '同步中...' : '同步到今日'),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
    );
  }
}

class _GrowthRule {
  const _GrowthRule({required this.intervalDays, required this.increment});

  final int intervalDays;
  final int increment;
}

String _growthModeLabel(String mode) {
  return switch (mode) {
    'newWord' => '新词',
    'review' => '复习',
    'mixedTest' => '混合',
    'wrongWordReinforcement' => '错词',
    'highFrequency' => '高频词',
    'rootAffix' => '词根词缀',
    _ => mode,
  };
}

class _DirtyBanner extends StatelessWidget {
  const _DirtyBanner();

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: const Color(0xFFF7E8C5),
        borderRadius: BorderRadius.circular(12),
      ),
      child: const Padding(
        padding: EdgeInsets.all(12),
        child: Text('你有未保存的计划改动。保存后再决定是否同步到 Today，会更接近 RN 的使用路径。'),
      ),
    );
  }
}

class _PlanHeroCard extends StatelessWidget {
  const _PlanHeroCard({required this.plan, required this.wordbooks});

  final PlanSummary plan;
  final List<WordbookSummary> wordbooks;

  @override
  Widget build(BuildContext context) {
    final activeWordbooks = wordbooks
        .where((wordbook) => wordbook.isActive)
        .toList();
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      color: Theme.of(context).colorScheme.primary,
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              plan.name,
              style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                color: Colors.white,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              '计划编辑、词书选择、增长规则会直接影响 Today 与 Study 的主流程。',
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                color: Colors.white.withValues(alpha: 0.88),
              ),
            ),
            const SizedBox(height: 16),
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                _InfoPill(label: '新词/天', value: '${plan.newWordsPerDay}'),
                _InfoPill(label: '复习/天', value: '${plan.reviewWordsPerDay}'),
                _InfoPill(label: '词书', value: '${activeWordbooks.length}'),
              ],
            ),
            if (activeWordbooks.isNotEmpty) ...[
              const SizedBox(height: 12),
              Text(
                '已选词书：${activeWordbooks.map((wordbook) => wordbook.name).join('、')}',
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: Colors.white70),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

class _ModeToggleChip extends StatelessWidget {
  const _ModeToggleChip({
    required this.label,
    required this.selected,
    required this.onTap,
  });

  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ChoiceChip(
      label: Text(label),
      selected: selected,
      onSelected: (_) => onTap(),
    );
  }
}

class _RuleEditorCard extends StatelessWidget {
  const _RuleEditorCard({
    required this.title,
    required this.subtitle,
    required this.intervalController,
    required this.incrementController,
  });

  final String title;
  final String subtitle;
  final TextEditingController intervalController;
  final TextEditingController incrementController;

  @override
  Widget build(BuildContext context) {
    return Container(
      decoration: BoxDecoration(
        color: const Color(0xFFF7FAFF),
        borderRadius: BorderRadius.circular(12),
      ),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          Text(
            subtitle,
            style: Theme.of(
              context,
            ).textTheme.bodySmall?.copyWith(color: Colors.black54),
          ),
          const SizedBox(height: 12),
          _StepperField(
            label: '增长间隔',
            helper: '多少天后提量',
            controller: intervalController,
            step: 1,
            max: 365,
            compact: true,
          ),
          const SizedBox(height: 10),
          _StepperField(
            label: '增长增量',
            helper: '每次增加多少',
            controller: incrementController,
            step: 1,
            max: 100,
            compact: true,
          ),
        ],
      ),
    );
  }
}

class _StepperField extends StatelessWidget {
  const _StepperField({
    required this.label,
    required this.helper,
    required this.controller,
    required this.step,
    required this.max,
    this.compact = false,
  });

  final String label;
  final String helper;
  final TextEditingController controller;
  final int step;
  final int max;
  final bool compact;

  void _adjust(int delta) {
    final current = int.tryParse(controller.text) ?? 0;
    var next = (current + delta).clamp(0, max);
    if (step == 4) {
      next -= next % 4;
    }
    controller.text = '$next';
  }

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(label, style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 4),
              Text(
                helper,
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: Colors.black54),
              ),
            ],
          ),
        ),
        IconButton(
          onPressed: () => _adjust(-step),
          icon: const Icon(Icons.remove_circle_outline),
          visualDensity: compact ? VisualDensity.compact : null,
        ),
        SizedBox(
          width: compact ? 64 : 72,
          child: TextField(
            controller: controller,
            keyboardType: TextInputType.number,
            textAlign: TextAlign.center,
            decoration: const InputDecoration(
              isDense: true,
              border: OutlineInputBorder(),
            ),
          ),
        ),
        IconButton(
          onPressed: () => _adjust(step),
          icon: const Icon(Icons.add_circle_outline),
          visualDensity: compact ? VisualDensity.compact : null,
        ),
      ],
    );
  }
}

class _PlanMessage extends StatelessWidget {
  const _PlanMessage({required this.message, required this.onRetry});

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

class _InfoPill extends StatelessWidget {
  const _InfoPill({required this.label, required this.value});

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

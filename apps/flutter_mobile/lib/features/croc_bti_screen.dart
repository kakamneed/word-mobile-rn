import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'croc_bti_model.dart';

class CrocBtiScreen extends StatefulWidget {
  const CrocBtiScreen({
    super.key,
    required this.sdk,
    this.userId,
    this.initialPlan,
    this.onApplied,
  });

  final WordSdk sdk;
  final String? userId;
  final PlanSummary? initialPlan;
  final VoidCallback? onApplied;

  @override
  State<CrocBtiScreen> createState() => _CrocBtiScreenState();
}

class _CrocBtiScreenState extends State<CrocBtiScreen> {
  final Map<String, int> _answers = {};
  Map<String, int>? _editedPlanInput;
  Map<String, Map<String, int>>? _editedQuestionTypeWeights;
  String? _editedResultCode;
  PlanSummary? _plan;
  int _dailyLearningMinutes = 40;
  bool _growthRuleEnabled = true;
  bool _loading = true;
  bool _saving = false;
  bool _showResult = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _plan = widget.initialPlan;
    _loadInitialState();
  }

  Future<void> _loadInitialState() async {
    try {
      final savedAnswers = await loadSavedCrocBtiAnswers(userId: widget.userId);
      final savedMinutes = await loadSavedCrocBtiDailyMinutes(
        userId: widget.userId,
      );
      final restoredProfile = await widget.sdk.crocBti.getProfile();
      final restoredAnswers = _profileAnswers(restoredProfile);
      final restoredMinutes = (restoredProfile?['dailyLearningMinutes'] as num?)
          ?.toInt();
      _answers
        ..clear()
        ..addAll(savedAnswers.isNotEmpty ? savedAnswers : restoredAnswers);
      _dailyLearningMinutes = (restoredMinutes ?? savedMinutes)
          .clamp(10, 240)
          .toInt();
      _showResult = hasCompleteCrocBtiAnswers(_answers);
    } catch (_) {
      // Corrupt local answers should not block retaking the test.
    }
    await _loadPlan();
  }

  Future<void> _loadPlan() async {
    if (_plan != null) {
      setState(() => _loading = false);
      return;
    }
    try {
      final plan = await widget.sdk.plan.getActivePlan();
      if (!mounted) return;
      setState(() {
        _plan = plan;
        _loading = false;
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error.toString();
        _loading = false;
      });
    }
  }

  void _ensureEditableState(CrocBtiResult result) {
    if (_editedResultCode == result.code &&
        _editedPlanInput != null &&
        _editedQuestionTypeWeights != null) {
      return;
    }
    _editedResultCode = result.code;
    _editedPlanInput = crocBtiPlanInputForDailyMinutes(
      result.weights,
      _dailyLearningMinutes,
    );
    _editedQuestionTypeWeights = {
      for (final mode in result.questionTypeWeightsByMode.entries)
        mode.key: Map<String, int>.from(mode.value),
    };
    _growthRuleEnabled = _plan?.growthRuleEnabled ?? true;
  }

  void _updateDailyLearningMinutes(CrocBtiResult result, int minutes) {
    setState(() {
      _dailyLearningMinutes = minutes.clamp(10, 240).toInt();
      _editedPlanInput = crocBtiPlanInputForDailyMinutes(
        result.weights,
        _dailyLearningMinutes,
      );
    });
  }

  void _updatePlanInput(String key, int value) {
    setState(() {
      final current = _editedPlanInput;
      if (current == null || !current.containsKey(key)) return;
      current[key] = value.clamp(0, 240).toInt();
    });
  }

  void _updateQuestionTypeWeight(String mode, String questionType, int value) {
    setState(() {
      final weights = _editedQuestionTypeWeights?[mode];
      if (weights == null || !weights.containsKey(questionType)) return;
      _editedQuestionTypeWeights![mode] = _rebalanceQuestionTypeWeights(
        weights,
        questionType,
        value.clamp(0, 100).toInt(),
      );
    });
  }

  void _updateGrowthRuleEnabled(bool value) {
    setState(() => _growthRuleEnabled = value);
  }

  Future<void> _applyResult(CrocBtiResult result) async {
    final plan = _plan;
    if (plan == null) return;
    _ensureEditableState(result);
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      await saveCrocBtiAnswers(_answers, userId: widget.userId);
      await saveCrocBtiDailyMinutes(
        _dailyLearningMinutes,
        userId: widget.userId,
      );
      final planInput = Map<String, int>.from(
        _editedPlanInput ?? result.planInput,
      );
      final questionTypeWeightsByMode = {
        for (final entry
            in (_editedQuestionTypeWeights ?? result.questionTypeWeightsByMode)
                .entries)
          entry.key: Map<String, int>.from(entry.value),
      };
      await widget.sdk.crocBti.saveProfile(
        profile: _profilePayload(
          result: result,
          planInput: planInput,
          questionTypeWeightsByMode: questionTypeWeightsByMode,
        ),
      );
      await widget.sdk.plan.savePlan(
        planId: plan.id,
        input: crocBtiPlanInputFor(
          plan,
          result,
          planInput: planInput,
          questionTypeWeightsByMode: questionTypeWeightsByMode,
          growthRuleEnabled: _growthRuleEnabled,
        ),
      );
      final applied = await widget.sdk.plan.applySavedPlanToToday();
      await widget.sdk.sync.flushPendingToCloud();
      if (!mounted) return;
      setState(() => _plan = applied);
      widget.onApplied?.call();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('${result.title} 已应用到今日计划')));
      Navigator.of(context).maybePop(true);
    } catch (error) {
      if (!mounted) return;
      setState(() => _error = error.toString());
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  Map<String, dynamic> _profilePayload({
    required CrocBtiResult result,
    required Map<String, int> planInput,
    required Map<String, Map<String, int>> questionTypeWeightsByMode,
  }) {
    return <String, dynamic>{
      'resultCode': result.code,
      'title': result.title,
      'summary': result.summary,
      'advice': result.advice,
      'answers': Map<String, int>.from(_answers),
      'axisScores': {
        for (final entry in result.axisScores.entries)
          entry.key: {
            'score': entry.value.score,
            'selectedTrait': entry.value.selectedTrait,
            'strength': entry.value.strength,
          },
      },
      'weights': Map<String, int>.from(result.weights),
      'planInput': planInput,
      'questionTypeWeightsByMode': questionTypeWeightsByMode,
      'dailyLearningMinutes': _dailyLearningMinutes,
      'growthRuleEnabled': _growthRuleEnabled,
      'source': 'croc_bti',
      'version': DateTime.now().millisecondsSinceEpoch,
      'evaluatedAt': DateTime.now().toUtc().toIso8601String(),
    };
  }

  @override
  Widget build(BuildContext context) {
    final result = evaluateCrocBti(_answers);
    final allAnswered = _answers.length == crocBtiQuestions.length;
    if (_showResult) {
      _ensureEditableState(result);
    }
    return Scaffold(
      appBar: AppBar(
        title: const Text('鳄bti'),
        actions: [
          TextButton(
            onPressed: allAnswered
                ? () => setState(() => _showResult = true)
                : null,
            child: const Text('结果'),
          ),
        ],
      ),
      body: _loading
          ? const CrocodileLoadingAnimation(label: '加载中...')
          : _error != null
          ? _MessageState(message: _error!, onRetry: _loadPlan)
          : _plan == null
          ? _MessageState(message: '还没有可应用的学习计划', onRetry: _loadPlan)
          : _showResult
          ? _ResultView(
              result: result,
              dailyLearningMinutes: _dailyLearningMinutes,
              planInput: _editedPlanInput!,
              questionTypeWeightsByMode: _editedQuestionTypeWeights!,
              saving: _saving,
              onDailyLearningMinutesChanged: (minutes) =>
                  _updateDailyLearningMinutes(result, minutes),
              onPlanInputChanged: _updatePlanInput,
              onQuestionTypeWeightChanged: _updateQuestionTypeWeight,
              growthRuleEnabled: _growthRuleEnabled,
              onGrowthRuleEnabledChanged: _updateGrowthRuleEnabled,
              onRetake: () async {
                await clearSavedCrocBtiAnswers(userId: widget.userId);
                if (!mounted) return;
                setState(() {
                  _answers.clear();
                  _editedPlanInput = null;
                  _editedQuestionTypeWeights = null;
                  _editedResultCode = null;
                  _dailyLearningMinutes = 40;
                  _showResult = false;
                });
              },
              onApply: () => _applyResult(result),
            )
          : _QuestionView(
              answers: _answers,
              dailyLearningMinutes: _dailyLearningMinutes,
              onAnswerChanged: (id, value) {
                setState(() => _answers[id] = value);
              },
              onDailyLearningMinutesChanged: (minutes) {
                setState(() {
                  _dailyLearningMinutes = minutes.clamp(10, 240).toInt();
                  _editedPlanInput = null;
                });
              },
              onShowResult: allAnswered
                  ? () {
                      saveCrocBtiAnswers(_answers, userId: widget.userId);
                      saveCrocBtiDailyMinutes(
                        _dailyLearningMinutes,
                        userId: widget.userId,
                      );
                      setState(() => _showResult = true);
                    }
                  : null,
            ),
    );
  }
}

class _QuestionView extends StatelessWidget {
  const _QuestionView({
    required this.answers,
    required this.dailyLearningMinutes,
    required this.onAnswerChanged,
    required this.onDailyLearningMinutesChanged,
    required this.onShowResult,
  });

  final Map<String, int> answers;
  final int dailyLearningMinutes;
  final void Function(String questionId, int value) onAnswerChanged;
  final void Function(int minutes) onDailyLearningMinutesChanged;
  final VoidCallback? onShowResult;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(18),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('学习人格测试', style: Theme.of(context).textTheme.labelLarge),
                const SizedBox(height: 8),
                Text(
                  '测出你的鳄bti学习人格',
                  style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 8),
                const Text('完成题目后，将推荐每日计划、各模式权重与题型占比。'),
                const SizedBox(height: 14),
                LinearProgressIndicator(
                  value: answers.length / crocBtiQuestions.length,
                ),
                const SizedBox(height: 8),
                Text('${answers.length}/${crocBtiQuestions.length} 已回答'),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        for (var i = 0; i < crocBtiQuestions.length; i++) ...[
          _QuestionCard(
            index: i + 1,
            question: crocBtiQuestions[i],
            selectedValue: answers[crocBtiQuestions[i].id],
            onChanged: onAnswerChanged,
          ),
          const SizedBox(height: 12),
        ],
        _DailyMinutesQuestionCard(
          minutes: dailyLearningMinutes,
          onChanged: onDailyLearningMinutesChanged,
        ),
        const SizedBox(height: 12),
        FilledButton(
          onPressed: onShowResult,
          child: Text(onShowResult == null ? '答完后查看结果' : '查看我的鳄bti'),
        ),
      ],
    );
  }
}

Map<String, int> _profileAnswers(Map<String, dynamic>? profile) {
  final raw = profile?['answers'];
  if (raw is! Map) return const <String, int>{};
  final validQuestionIds = crocBtiQuestions
      .map((question) => question.id)
      .toSet();
  return {
    for (final entry in raw.entries)
      if (validQuestionIds.contains('${entry.key}'))
        '${entry.key}': _intFromProfileValue(entry.value).clamp(-2, 2),
  };
}

int _intFromProfileValue(Object? value) => value is num ? value.toInt() : 0;

class _QuestionCard extends StatelessWidget {
  const _QuestionCard({
    required this.index,
    required this.question,
    required this.selectedValue,
    required this.onChanged,
  });

  final int index;
  final CrocBtiQuestion question;
  final int? selectedValue;
  final void Function(String questionId, int value) onChanged;

  @override
  Widget build(BuildContext context) {
    final labels = const {
      -2: '非常不同意',
      -1: '有点不同意',
      0: '说不准',
      1: '有点同意',
      2: '非常同意',
    };
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '$index',
              style: TextStyle(
                color: Theme.of(context).colorScheme.primary,
                fontWeight: FontWeight.w800,
              ),
            ),
            const SizedBox(height: 6),
            Text(question.text, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final entry in labels.entries)
                  ChoiceChip(
                    label: Text(entry.value),
                    selected: selectedValue == entry.key,
                    onSelected: (_) => onChanged(question.id, entry.key),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _DailyMinutesQuestionCard extends StatelessWidget {
  const _DailyMinutesQuestionCard({
    required this.minutes,
    required this.onChanged,
  });

  final int minutes;
  final void Function(int minutes) onChanged;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '每日学习时间',
              style: TextStyle(
                color: Theme.of(context).colorScheme.primary,
                fontWeight: FontWeight.w800,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              '你每天通常能花多少分钟背单词？',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                Expanded(
                  child: Slider(
                    value: minutes.toDouble(),
                    min: 10,
                    max: 120,
                    divisions: 22,
                    label: '$minutes 分钟',
                    onChanged: (value) => onChanged(value.round()),
                  ),
                ),
                SizedBox(
                  width: 72,
                  child: Text('$minutes 分钟', textAlign: TextAlign.end),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _ResultView extends StatelessWidget {
  const _ResultView({
    required this.result,
    required this.dailyLearningMinutes,
    required this.planInput,
    required this.questionTypeWeightsByMode,
    required this.growthRuleEnabled,
    required this.saving,
    required this.onDailyLearningMinutesChanged,
    required this.onPlanInputChanged,
    required this.onQuestionTypeWeightChanged,
    required this.onGrowthRuleEnabledChanged,
    required this.onRetake,
    required this.onApply,
  });

  final CrocBtiResult result;
  final int dailyLearningMinutes;
  final Map<String, int> planInput;
  final Map<String, Map<String, int>> questionTypeWeightsByMode;
  final bool growthRuleEnabled;
  final bool saving;
  final void Function(int minutes) onDailyLearningMinutesChanged;
  final void Function(String key, int value) onPlanInputChanged;
  final void Function(String mode, String questionType, int value)
  onQuestionTypeWeightChanged;
  final ValueChanged<bool> onGrowthRuleEnabledChanged;
  final VoidCallback onRetake;
  final VoidCallback onApply;

  @override
  Widget build(BuildContext context) {
    final maxWeight = result.weights.values.fold<int>(
      0,
      (max, value) => value > max ? value : max,
    );
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(22),
            child: Column(
              children: [
                SizedBox(
                  height: 180,
                  child: Image.asset(
                    result.assetPath,
                    fit: BoxFit.contain,
                    filterQuality: FilterQuality.medium,
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  result.code,
                  style: TextStyle(
                    color: Theme.of(context).colorScheme.primary,
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  result.title,
                  style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                    fontWeight: FontWeight.w900,
                  ),
                ),
                const SizedBox(height: 10),
                Text(result.summary, textAlign: TextAlign.center),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('学习模式推荐', style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 12),
                for (final item in const [
                  ('newWords', '新词'),
                  ('review', '复习'),
                  ('mixedTest', '混测'),
                  ('wrongWordReview', '错词'),
                  ('contextExamples', '语境'),
                  ('activeRecall', '回忆'),
                ])
                  _WeightRow(
                    label: item.$2,
                    value: result.weights[item.$1]!,
                    scaleMax: maxWeight,
                  ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        _PlanInputCard(
          dailyLearningMinutes: dailyLearningMinutes,
          planInput: planInput,
          growthRuleEnabled: growthRuleEnabled,
          enabled: !saving,
          onDailyLearningMinutesChanged: onDailyLearningMinutesChanged,
          onPlanInputChanged: onPlanInputChanged,
          onGrowthRuleEnabledChanged: onGrowthRuleEnabledChanged,
        ),
        const SizedBox(height: 12),
        _QuestionTypeWeightsCard(
          result: result,
          weightsByMode: questionTypeWeightsByMode,
          enabled: !saving,
          onChanged: onQuestionTypeWeightChanged,
        ),
        const SizedBox(height: 12),
        FilledButton(
          onPressed: saving ? null : onApply,
          child: Text(saving ? '应用中...' : '应用到今日'),
        ),
        TextButton(
          onPressed: saving ? null : onRetake,
          child: const Text('重新测试'),
        ),
      ],
    );
  }
}

class _PlanInputCard extends StatelessWidget {
  const _PlanInputCard({
    required this.dailyLearningMinutes,
    required this.planInput,
    required this.growthRuleEnabled,
    required this.enabled,
    required this.onDailyLearningMinutesChanged,
    required this.onPlanInputChanged,
    required this.onGrowthRuleEnabledChanged,
  });

  final int dailyLearningMinutes;
  final Map<String, int> planInput;
  final bool growthRuleEnabled;
  final bool enabled;
  final void Function(int minutes) onDailyLearningMinutesChanged;
  final void Function(String key, int value) onPlanInputChanged;
  final ValueChanged<bool> onGrowthRuleEnabledChanged;

  @override
  Widget build(BuildContext context) {
    final totalQuestions = dailyLearningMinutes * 4;
    final planSliderMax = ((totalQuestions * 0.55).round()).clamp(20, 320);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('今日计划推荐', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 6),
            Text(
              '先选择每日学习时间，再微调各模式任务数量。',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 14),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              title: const Text('\u542f\u7528\u589e\u957f\u63a8\u8350'),
              subtitle: const Text(
                '\u548c\u8ba1\u5212\u9875\u4e00\u81f4\uff0c\u5f00\u542f\u540e\u4f1a\u9010\u6b65\u63d0\u9ad8\u4eca\u65e5\u76ee\u6807',
              ),
              value: growthRuleEnabled,
              onChanged: enabled ? onGrowthRuleEnabledChanged : null,
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                const SizedBox(width: 112, child: Text('每日时间')),
                Expanded(
                  child: Slider(
                    value: dailyLearningMinutes.toDouble(),
                    min: 10,
                    max: 120,
                    divisions: 22,
                    label: '$dailyLearningMinutes 分钟',
                    onChanged: enabled
                        ? (value) =>
                              onDailyLearningMinutesChanged(value.round())
                        : null,
                  ),
                ),
                SizedBox(
                  width: 76,
                  child: Text(
                    '$dailyLearningMinutes 分钟',
                    textAlign: TextAlign.end,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            for (final item in const [
              ('newWordsPerDay', '新词题', 0),
              ('reviewWordsPerDay', '复习', 0),
              ('mixedTestPerDay', '混测', 0),
              ('wrongWordTestPerDay', '错词', 0),
              ('highFrequencyPerDay', '高频词', 0),
              ('rootAffixPerDay', '词根', 0),
            ])
              _PlanCountSlider(
                keyName: item.$1,
                label: item.$2,
                value: planInput[item.$1] ?? 0,
                min: item.$3,
                max: planSliderMax,
                step: item.$1 == 'newWordsPerDay' ? 4 : 1,
                enabled: enabled,
                onChanged: onPlanInputChanged,
              ),
          ],
        ),
      ),
    );
  }
}

class _PlanCountSlider extends StatelessWidget {
  const _PlanCountSlider({
    required this.keyName,
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.step,
    required this.enabled,
    required this.onChanged,
  });

  final String keyName;
  final String label;
  final int value;
  final int min;
  final int max;
  final int step;
  final bool enabled;
  final void Function(String key, int value) onChanged;

  @override
  Widget build(BuildContext context) {
    final clampedValue = value < min ? min : (value > max ? max : value);
    final divisions = ((max - min) / step).round().clamp(1, 240);
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(child: Text(label)),
              Text('$value', textAlign: TextAlign.end),
            ],
          ),
          Slider(
            value: clampedValue.toDouble(),
            min: min.toDouble(),
            max: max.toDouble(),
            divisions: divisions,
            label: '$value',
            onChanged: enabled
                ? (next) {
                    final rounded = next.round();
                    final stepped = rounded - (rounded % step);
                    onChanged(keyName, stepped.clamp(min, max).toInt());
                  }
                : null,
          ),
        ],
      ),
    );
  }
}

class _QuestionTypeWeightsCard extends StatelessWidget {
  const _QuestionTypeWeightsCard({
    required this.result,
    required this.weightsByMode,
    required this.enabled,
    required this.onChanged,
  });

  final CrocBtiResult result;
  final Map<String, Map<String, int>> weightsByMode;
  final bool enabled;
  final void Function(String mode, String questionType, int value) onChanged;

  @override
  Widget build(BuildContext context) {
    final modes = const ['review', 'mixedTest', 'wrongWordReinforcement'];
    final reasons = _questionTypeReasonsFor(result);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('题型推荐占比', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 6),
            Text(
              '确认应用后，复习、混测、错词会按这些题型比例出题；新词学习保持原来的四题型顺序，词根词缀保持原流程。',
              style: Theme.of(context).textTheme.bodySmall,
            ),
            const SizedBox(height: 12),
            Text('推荐原因', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 6),
            for (final reason in reasons)
              Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: Text(
                  '· $reason',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
            const SizedBox(height: 12),
            for (final mode in modes)
              if (weightsByMode[mode] != null)
                _ModeQuestionTypeEditor(
                  mode: mode,
                  weights: weightsByMode[mode]!,
                  enabled: enabled,
                  onChanged: onChanged,
                ),
          ],
        ),
      ),
    );
  }
}

List<String> _questionTypeReasonsFor(CrocBtiResult result) {
  final reasons = <String>['新词学习先稳定建立初见印象，因此不参与题型定制，仍按原来的四题型顺序推进。'];
  final code = result.code;
  if (code.contains('V')) {
    reasons.add('你更偏向词库冲锋，混测会保留较多英文选中文，先把识别速度拉起来。');
  } else {
    reasons.add('你更依赖语境理解，混测会增加无中文翻译例句，训练从句子里判断词义。');
  }
  if (code.contains('O')) {
    reasons.add('你适合主动召回，错词和混测会增加挖空补全、中文选英文，逼近真正会用。');
  } else {
    reasons.add('你适合先稳住识别链路，复习会保留更多输入中文释义与英文选中文。');
  }
  if (code.contains('R')) {
    reasons.add('你偏复盘型，复习和错词会提高输入题比例，用更强反馈巩固薄弱词。');
  } else {
    reasons.add('你偏开拓型，题型定制主要放在混测里，避免新词阶段被高阻力题型拖慢。');
  }
  if (code.contains('A')) {
    reasons.add('你适合语感路径，因此无翻译例句题占比会更高，减少对中文提示的依赖。');
  } else {
    reasons.add('你适合考试压测路径，因此混测会更强调限时判断、拼写补全和错题强化。');
  }
  return reasons;
}

class _ModeQuestionTypeEditor extends StatelessWidget {
  const _ModeQuestionTypeEditor({
    required this.mode,
    required this.weights,
    required this.enabled,
    required this.onChanged,
  });

  final String mode;
  final Map<String, int> weights;
  final bool enabled;
  final void Function(String mode, String questionType, int value) onChanged;

  @override
  Widget build(BuildContext context) {
    final total = weights.values.fold<int>(0, (sum, value) => sum + value);
    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  _modeLabel(mode),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              Text('$total%'),
            ],
          ),
          const SizedBox(height: 8),
          for (final entry in weights.entries)
            _QuestionTypeSlider(
              mode: mode,
              questionType: entry.key,
              value: entry.value,
              enabled: enabled,
              onChanged: onChanged,
            ),
        ],
      ),
    );
  }
}

class _QuestionTypeSlider extends StatelessWidget {
  const _QuestionTypeSlider({
    required this.mode,
    required this.questionType,
    required this.value,
    required this.enabled,
    required this.onChanged,
  });

  final String mode;
  final String questionType;
  final int value;
  final bool enabled;
  final void Function(String mode, String questionType, int value) onChanged;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        children: [
          SizedBox(width: 96, child: Text(_questionTypeLabel(questionType))),
          Expanded(
            child: Stack(
              alignment: Alignment.centerLeft,
              children: [
                ClipRRect(
                  borderRadius: BorderRadius.circular(999),
                  child: LinearProgressIndicator(
                    minHeight: 6,
                    value: (value / 60).clamp(0.0, 1.0),
                  ),
                ),
                SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    trackHeight: 0,
                    activeTrackColor: Colors.transparent,
                    inactiveTrackColor: Colors.transparent,
                    overlayShape: SliderComponentShape.noOverlay,
                    thumbShape: const RoundSliderThumbShape(
                      enabledThumbRadius: 8,
                    ),
                  ),
                  child: Slider(
                    value: value < 0 ? 0 : (value > 60 ? 60 : value).toDouble(),
                    min: 0,
                    max: 60,
                    divisions: 12,
                    label: '$value%',
                    onChanged: enabled
                        ? (next) => onChanged(mode, questionType, next.round())
                        : null,
                  ),
                ),
              ],
            ),
          ),
          SizedBox(width: 46, child: Text('$value%', textAlign: TextAlign.end)),
        ],
      ),
    );
  }
}

class _WeightRow extends StatelessWidget {
  const _WeightRow({
    required this.label,
    required this.value,
    required this.scaleMax,
  });

  final String label;
  final int value;
  final int scaleMax;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: Row(
        children: [
          SizedBox(width: 54, child: Text(label)),
          Expanded(
            child: LinearProgressIndicator(
              minHeight: 6,
              value: scaleMax <= 0 ? 0 : (value / scaleMax).clamp(0.0, 1.0),
            ),
          ),
          const SizedBox(width: 10),
          SizedBox(width: 46, child: Text('$value%', textAlign: TextAlign.end)),
        ],
      ),
    );
  }
}

String _modeLabel(String mode) => switch (mode) {
  'newWord' => '新词学习',
  'review' => '复习',
  'mixedTest' => '混合测试',
  'wrongWordReinforcement' => '错词强化',
  _ => mode,
};

String _questionTypeLabel(String questionType) => switch (questionType) {
  'enToCnChoice' => '英文选中文',
  'exampleToCnChoice' => '例句选中文',
  'exampleToCnChoiceNoTranslation' => '纯例句选中文',
  'cnToEnChoice' => '中文选英文',
  'enToCnInput' => '英文输中文',
  'wordSkeletonInput' => '单词补全',
  _ => questionType,
};

Map<String, int> _rebalanceQuestionTypeWeights(
  Map<String, int> current,
  String changedKey,
  int changedValue,
) {
  final next = Map<String, int>.from(current);
  next[changedKey] = changedValue;
  final otherKeys = next.keys.where((key) => key != changedKey).toList();
  if (otherKeys.isEmpty) {
    next[changedKey] = 100;
    return next;
  }

  final remaining = (100 - changedValue).clamp(0, 100).toInt();
  final otherTotal = otherKeys.fold<int>(
    0,
    (sum, key) => sum + (current[key] ?? 0),
  );
  var assigned = 0;
  for (var index = 0; index < otherKeys.length; index += 1) {
    final key = otherKeys[index];
    final value = index == otherKeys.length - 1
        ? remaining - assigned
        : otherTotal <= 0
        ? (remaining / otherKeys.length).round()
        : (((current[key] ?? 0) / otherTotal) * remaining).round();
    next[key] = value.clamp(0, 100).toInt();
    assigned += next[key]!;
  }

  return normalizeCrocBtiQuestionTypeWeightsByMode({'mode': next})['mode']!;
}

class _MessageState extends StatelessWidget {
  const _MessageState({required this.message, required this.onRetry});

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

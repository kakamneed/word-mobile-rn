import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import '../supabase/auth_session_manager.dart';
import 'auth_screen.dart';
import 'croc_bti_model.dart';

class OnboardingFlow extends StatefulWidget {
  const OnboardingFlow({
    super.key,
    required this.appState,
    this.replayMode = false,
    this.onReplayFinished,
  });

  final AppState appState;
  final bool replayMode;
  final VoidCallback? onReplayFinished;

  @override
  State<OnboardingFlow> createState() => _OnboardingFlowState();
}

class _OnboardingFlowState extends State<OnboardingFlow> {
  final _pageController = PageController();
  final _newWordsController = TextEditingController(text: '20');
  final _reviewWordsController = TextEditingController(text: '40');
  final _mixedController = TextEditingController(text: '10');
  final Map<String, int> _crocBtiAnswers = {};

  int _step = 0;
  bool _savingPlan = false;
  String? _error;
  CrocBtiResult? _crocBtiResult;

  @override
  void initState() {
    super.initState();
    _loadCurrentPlan();
  }

  @override
  void dispose() {
    _pageController.dispose();
    _newWordsController.dispose();
    _reviewWordsController.dispose();
    _mixedController.dispose();
    super.dispose();
  }

  Future<void> _loadCurrentPlan() async {
    try {
      final plan = await widget.appState.sdk.plan.getActivePlan();
      if (!mounted || plan == null) return;
      setState(() {
        _newWordsController.text = '${plan.newWordsPerDay}';
        _reviewWordsController.text = '${plan.reviewWordsPerDay}';
        _mixedController.text = '${plan.mixedTestPerDay}';
      });
    } catch (_) {
      // Onboarding can still continue with the conservative defaults above.
    }
  }

  Future<void> _openAuth(AuthEntryMode mode) async {
    final authState = await Navigator.of(context).push<AuthAccountState>(
      MaterialPageRoute(
        builder: (_) => AuthScreen(
          sdk: widget.appState.sdk,
          initialMode: mode,
          onAuthChanged: widget.appState.applyAuthState,
        ),
      ),
    );
    if (authState != null) {
      widget.appState.applyAuthState(authState);
    }
  }

  Future<void> _goNext() async {
    if (_step == 2) {
      final saved = await _savePlan();
      if (!saved) return;
    }
    if (_step >= 3) {
      if (widget.replayMode) {
        widget.onReplayFinished?.call();
        if (mounted) {
          Navigator.of(context).pop(true);
        }
        return;
      }
      await widget.appState.completeOnboarding();
      return;
    }
    final next = _step + 1;
    setState(() => _step = next);
    await _pageController.animateToPage(
      next,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
    );
  }

  Future<bool> _savePlan() async {
    final plan = await widget.appState.sdk.plan.getActivePlan();
    if (plan == null) {
      return true;
    }
    setState(() {
      _savingPlan = true;
      _error = null;
    });
    try {
      final saved = await widget.appState.sdk.plan.savePlan(
        planId: plan.id,
        input: _buildPlanInput(plan),
      );
      await widget.appState.sdk.plan.applySavedPlanToToday();
      if (!mounted) return true;
      _newWordsController.text = '${saved.newWordsPerDay}';
      _reviewWordsController.text = '${saved.reviewWordsPerDay}';
      _mixedController.text = '${saved.mixedTestPerDay}';
      return true;
    } catch (error) {
      if (!mounted) return false;
      setState(() => _error = error.toString());
      return false;
    } finally {
      if (mounted) {
        setState(() => _savingPlan = false);
      }
    }
  }

  Map<String, dynamic> _buildPlanInput(PlanSummary plan) {
    final crocBtiResult = _crocBtiResult;
    if (crocBtiResult != null) {
      return crocBtiPlanInputFor(plan, crocBtiResult);
    }
    return <String, dynamic>{
      'name': plan.name,
      'newWordsPerDay': _parseCount(_newWordsController, plan.newWordsPerDay, 0, 100),
      'reviewWordsPerDay': _parseCount(_reviewWordsController, plan.reviewWordsPerDay, 0, 200),
      'mixedTestPerDay': _parseCount(_mixedController, plan.mixedTestPerDay, 0, 50),
        'wrongWordTestPerDay': plan.wrongWordTestPerDay,
        'highFrequencyPerDay': plan.highFrequencyPerDay,
        'rootAffixPerDay': plan.rootAffixPerDay ?? 0,
      'growthRuleMode': plan.growthRuleMode,
      'growthIntervalDays': plan.growthIntervalDays,
      'growthIncrement': plan.growthIncrement,
      'sharedGrowthRule': <String, dynamic>{
        'intervalDays': plan.growthIntervalDays,
        'increment': plan.growthIncrement,
      },
      'growthRulesByMode': plan.growthRulesByMode ?? const <String, dynamic>{},
    };
  }

  Future<void> _jumpToStep(int step) async {
    setState(() => _step = step);
    await _pageController.animateToPage(
      step,
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
    );
  }

  int _parseCount(
    TextEditingController controller,
    int fallback,
    int min,
    int max,
  ) {
    final value = int.tryParse(controller.text.trim()) ?? fallback;
    return value.clamp(min, max);
  }

  @override
  Widget build(BuildContext context) {
    final signedIn = widget.appState.authState.isSignedIn;
    return Scaffold(
      appBar: AppBar(
        title: const Text('开始使用'),
        actions: [
          TextButton(
            onPressed: widget.replayMode
                ? () => Navigator.of(context).pop(false)
                : widget.appState.completeOnboarding,
            child: const Text('跳过'),
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 8, 20, 12),
              child: _ProgressHeader(step: _step),
            ),
            Expanded(
              child: PageView(
                controller: _pageController,
                physics: const NeverScrollableScrollPhysics(),
                children: [
                  _LoginStep(
                    signedIn: signedIn,
                    email: widget.appState.authState.userEmail,
                    onSignIn: () => _openAuth(AuthEntryMode.signIn),
                    onSignUp: () => _openAuth(AuthEntryMode.signUp),
                    onSkip: _goNext,
                  ),
                  _CrocBtiOnboardingStep(
                    answers: _crocBtiAnswers,
                    result: _crocBtiResult,
                    onAnswerChanged: (questionId, value) {
                      setState(() => _crocBtiAnswers[questionId] = value);
                    },
                    onUseResult: () {
                      setState(() {
                        _crocBtiResult = evaluateCrocBti(_crocBtiAnswers);
                      });
                      _jumpToStep(2);
                    },
                    onSkip: () {
                      setState(() => _crocBtiResult = null);
                      _jumpToStep(2);
                    },
                  ),
                  _PlanStep(
                    newWordsController: _newWordsController,
                    reviewWordsController: _reviewWordsController,
                    mixedController: _mixedController,
                    error: _error,
                    crocBtiResult: _crocBtiResult,
                  ),
                  const _ExplainStep(),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 12, 20, 20),
              child: SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: _savingPlan ? null : _goNext,
                  child: Text(
                    _savingPlan
                        ? '正在保存...'
                        : switch (_step) {
                            0 => signedIn ? '继续设置计划' : '先跳过，继续体验',
                            1 => '保存计划',
                            _ => '进入应用',
                          },
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ProgressHeader extends StatelessWidget {
  const _ProgressHeader({required this.step});

  final int step;

  @override
  Widget build(BuildContext context) {
    final labels = const ['账号', '计划', '说明'];
    return Row(
      children: [
        for (var i = 0; i < labels.length; i++) ...[
          Expanded(
            child: Column(
              children: [
                AnimatedContainer(
                  duration: const Duration(milliseconds: 180),
                  height: 4,
                  decoration: BoxDecoration(
                    color: i <= step
                        ? Theme.of(context).colorScheme.primary
                        : Theme.of(context).colorScheme.surfaceContainerHighest,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
                const SizedBox(height: 6),
                Text(labels[i], style: Theme.of(context).textTheme.labelMedium),
              ],
            ),
          ),
          if (i != labels.length - 1) const SizedBox(width: 8),
        ],
      ],
    );
  }
}

class _LoginStep extends StatelessWidget {
  const _LoginStep({
    required this.signedIn,
    required this.email,
    required this.onSignIn,
    required this.onSignUp,
    required this.onSkip,
  });

  final bool signedIn;
  final String? email;
  final VoidCallback onSignIn;
  final VoidCallback onSignUp;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Icon(
          signedIn ? Icons.verified_user_outlined : Icons.cloud_sync_outlined,
          size: 52,
          color: Theme.of(context).colorScheme.primary,
        ),
        const SizedBox(height: 20),
        Text(
          signedIn ? '账号已准备好' : '登录后可保留云端进度',
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 10),
        Text(
          signedIn
              ? '${email ?? '当前账号'} 已登录。接下来为这个新用户设置第一份学习计划。'
              : '你可以现在登录或注册，也可以跳过。核心学习流程会继续保留在本机，之后仍可从头像入口登录。',
        ),
        const SizedBox(height: 24),
        if (!signedIn) ...[
          FilledButton.icon(
            onPressed: onSignIn,
            icon: const Icon(Icons.login),
            label: const Text('登录'),
          ),
          const SizedBox(height: 10),
          OutlinedButton.icon(
            onPressed: onSignUp,
            icon: const Icon(Icons.person_add_alt_1),
            label: const Text('注册新账号'),
          ),
          TextButton(onPressed: onSkip, child: const Text('暂不登录')),
        ],
      ],
    );
  }
}

class _PlanStep extends StatelessWidget {
  const _PlanStep({
    required this.newWordsController,
    required this.reviewWordsController,
    required this.mixedController,
    required this.error,
    required this.crocBtiResult,
  });

  final TextEditingController newWordsController;
  final TextEditingController reviewWordsController;
  final TextEditingController mixedController;
  final String? error;
  final CrocBtiResult? crocBtiResult;

  @override
  Widget build(BuildContext context) {
    final result = crocBtiResult;
    if (result != null) {
      return ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Text(
            '鳄bti 已接管初始计划',
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 10),
          Text('你的类型是 ${result.title}。初始计划会按测试结果分配新词、复习、混测和错题权重。'),
          const SizedBox(height: 16),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(result.code, style: TextStyle(color: Theme.of(context).colorScheme.primary, fontWeight: FontWeight.w800)),
                  const SizedBox(height: 4),
                  Text(result.title, style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800)),
                  const SizedBox(height: 8),
                  Text(result.advice),
                  const SizedBox(height: 12),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      Chip(label: Text('新词 ${result.planInput['newWordsPerDay']}')),
                      Chip(label: Text('复习 ${result.planInput['reviewWordsPerDay']}')),
                      Chip(label: Text('混测 ${result.planInput['mixedTestPerDay']}')),
                      Chip(label: Text('错题 ${result.planInput['wrongWordTestPerDay']}')),
                    ],
                  ),
                ],
              ),
            ),
          ),
          if (error != null) ...[
            const SizedBox(height: 16),
            Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
          ],
        ],
      );
    }

    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Text(
          '先定一个轻量计划',
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 10),
        const Text('这里设置每日词数，保存后会立即应用到今日任务。后续可以在“计划”页随时细调。'),
        const SizedBox(height: 20),
        _CountField(
          controller: newWordsController,
          label: '每日新词',
          helper: '建议从 15 到 30 个开始',
          step: 5,
          max: 100,
        ),
        const SizedBox(height: 12),
        _CountField(
          controller: reviewWordsController,
          label: '每日复习',
          helper: '复习量可以高于新词量',
          step: 10,
          max: 200,
        ),
        const SizedBox(height: 12),
        _CountField(
          controller: mixedController,
          label: '混合测试',
          helper: '用于检查掌握情况',
          step: 5,
          max: 50,
        ),
        if (error != null) ...[
          const SizedBox(height: 16),
          Text(error!, style: TextStyle(color: Theme.of(context).colorScheme.error)),
        ],
      ],
    );
  }
}

class _CrocBtiOnboardingStep extends StatelessWidget {
  const _CrocBtiOnboardingStep({
    required this.answers,
    required this.result,
    required this.onAnswerChanged,
    required this.onUseResult,
    required this.onSkip,
  });

  final Map<String, int> answers;
  final CrocBtiResult? result;
  final void Function(String questionId, int value) onAnswerChanged;
  final VoidCallback onUseResult;
  final VoidCallback onSkip;

  @override
  Widget build(BuildContext context) {
    final allAnswered = answers.length == crocBtiQuestions.length;
    final existingResult = result;
    return ListView(
      padding: const EdgeInsets.all(20),
      children: [
        Text(
          '鳄bti 学习人格',
          style: Theme.of(context).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 10),
        const Text('可跳过。完成后会用你的学习人格生成更合适的初始计划。'),
        if (existingResult != null) ...[
          const SizedBox(height: 12),
          Card(
            child: ListTile(
              title: Text('${existingResult.code} · ${existingResult.title}'),
              subtitle: Text(existingResult.summary),
            ),
          ),
        ],
        const SizedBox(height: 12),
        LinearProgressIndicator(value: answers.length / crocBtiQuestions.length),
        const SizedBox(height: 8),
        Text('${answers.length}/${crocBtiQuestions.length} 已回答'),
        const SizedBox(height: 16),
        for (var i = 0; i < crocBtiQuestions.length; i++) ...[
          _OnboardingBtiQuestionCard(
            index: i + 1,
            question: crocBtiQuestions[i],
            selectedValue: answers[crocBtiQuestions[i].id],
            onChanged: onAnswerChanged,
          ),
          const SizedBox(height: 12),
        ],
        Row(
          children: [
            Expanded(
              child: OutlinedButton(
                onPressed: onSkip,
                child: const Text('跳过'),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              flex: 2,
              child: FilledButton(
                onPressed: allAnswered ? onUseResult : null,
                child: Text(allAnswered ? '使用鳄bti结果' : '${answers.length}/${crocBtiQuestions.length}'),
              ),
            ),
          ],
        ),
      ],
    );
  }
}

class _OnboardingBtiQuestionCard extends StatelessWidget {
  const _OnboardingBtiQuestionCard({
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
      -2: '很不同意',
      -1: '不同意',
      0: '不确定',
      1: '同意',
      2: '很同意',
    };
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('$index', style: TextStyle(color: Theme.of(context).colorScheme.primary, fontWeight: FontWeight.w800)),
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

class _CountField extends StatelessWidget {
  const _CountField({
    required this.controller,
    required this.label,
    required this.helper,
    required this.step,
    required this.max,
  });

  final TextEditingController controller;
  final String label;
  final String helper;
  final int step;
  final int max;

  void _adjust(int delta) {
    final current = int.tryParse(controller.text.trim()) ?? 0;
    controller.text = '${(current + delta).clamp(0, max)}';
  }

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(label, style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 4),
                  Text(helper, style: Theme.of(context).textTheme.bodySmall),
                ],
              ),
            ),
            IconButton(
              tooltip: '减少',
              onPressed: () => _adjust(-step),
              icon: const Icon(Icons.remove_circle_outline),
            ),
            SizedBox(
              width: 64,
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
              tooltip: '增加',
              onPressed: () => _adjust(step),
              icon: const Icon(Icons.add_circle_outline),
            ),
          ],
        ),
      ),
    );
  }
}

class _ExplainStep extends StatelessWidget {
  const _ExplainStep();

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(20),
      children: const [
        _FeatureTile(
          icon: Icons.today_outlined,
          title: '今日',
          body: '集中处理今天该学的新词、复习和测试。',
        ),
        _FeatureTile(
          icon: Icons.tune_outlined,
          title: '计划',
          body: '调整每日词数、词书和增长规则。',
        ),
        _FeatureTile(
          icon: Icons.menu_book_outlined,
          title: '错词',
          body: '自动沉淀薄弱词，随时回炉强化。',
        ),
        _FeatureTile(
          icon: Icons.query_stats_outlined,
          title: '报告',
          body: '查看学习量、正确率和近期趋势。',
        ),
        _FeatureTile(
          icon: Icons.auto_awesome_outlined,
          title: 'AI',
          body: '生成阅读材料，并把生词导入学习流程。',
        ),
      ],
    );
  }
}

class _FeatureTile extends StatelessWidget {
  const _FeatureTile({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(body),
    );
  }
}

import 'dart:convert';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import '../supabase/auth_session_manager.dart';
import 'ai_screen.dart';
import 'auth_screen.dart';
import 'plan_screen.dart';
import 'reports_screen.dart';
import 'study_screen.dart';
import 'wrong_words_screen.dart';

class TodayShellScreen extends StatefulWidget {
  const TodayShellScreen({
    super.key,
    required this.appState,
    this.onOpenPlan,
    this.onOpenStudy,
    this.onOpenReports,
    this.onOpenWrongWords,
    this.onOpenAi,
    this.onOpenAccount,
  });

  final AppState appState;
  final Future<void> Function()? onOpenPlan;
  final Future<void> Function(String mode, ResumeSessionHint? hint)?
  onOpenStudy;
  final Future<void> Function()? onOpenReports;
  final Future<void> Function()? onOpenWrongWords;
  final Future<void> Function({bool generateOnOpen, bool showPassageFirst})?
  onOpenAi;
  final Future<void> Function()? onOpenAccount;

  @override
  State<TodayShellScreen> createState() => _TodayShellScreenState();
}

class _TodayShellScreenState extends State<TodayShellScreen> {
  int _reloadToken = 0;
  bool _aiGenerating = false;
  String? _aiMessage;

  Future<_TodayHomeBundle> _loadHomeBundle() async {
    var today = await widget.appState.sdk.today.getTodayHomeState();

    PlanSummary? activePlan;
    TodayAiPassageContext? aiContext;
    List<AiPassageHistoryItem> aiHistory = const [];
    TodayRewardState? rewardState;
    SyncStatus? syncStatus;

    try {
      activePlan = await widget.appState.sdk.plan.getActivePlan();
    } catch (_) {}

    try {
      aiContext = await widget.appState.sdk.ai.getTodayAiPassageContext();
    } catch (_) {}

    try {
      aiHistory = await widget.appState.sdk.ai.getAiPassageHistory();
    } catch (_) {}

    try {
      rewardState = await widget.appState.sdk.rewards.getTodayRewardState();
    } catch (_) {}

    try {
      syncStatus = await widget.appState.sdk.sync.getSyncStatus();
    } catch (_) {}

    final needsTodaySnapshot = !_hasUsableSnapshot(
      today.todaySnapshot ?? const <String, dynamic>{},
    );
    if (needsTodaySnapshot && activePlan != null) {
      try {
        await widget.appState.sdk.plan.applySavedPlanToToday();
        today = await widget.appState.sdk.today.getTodayHomeState();
      } catch (_) {}
    }

    return _TodayHomeBundle(
      today: today,
      activePlan: activePlan,
      aiContext: aiContext,
      aiHistory: aiHistory,
      rewardState: rewardState,
      syncStatus: syncStatus,
    );
  }

  Future<void> _generateAiPassage(TodayAiPassageContext? aiContext) async {
    if (aiContext == null || _aiGenerating) return;

    final wrongWords = aiContext.generationWrongWords
        .take(6)
        .toList(growable: false);
    final targetWords = aiContext.generationTargetWords
        .take(6)
        .toList(growable: false);

    if (!aiContext.tasksComplete) {
      setState(() {
        _aiMessage =
            '\u8bf7\u5148\u5b8c\u6210\u4eca\u65e5\u4efb\u52a1\uff0c\u518d\u751f\u6210 AI \u77ed\u6587\u3002';
      });
      return;
    }

    if (wrongWords.isEmpty && targetWords.isEmpty) {
      setState(() {
        _aiMessage =
            '\u5f53\u524d\u6ca1\u6709\u53ef\u7528\u4e8e\u751f\u6210\u77ed\u6587\u7684\u9519\u8bcd\u3002';
      });
      return;
    }

    setState(() {
      _aiGenerating = true;
      _aiMessage = null;
    });

    try {
      await widget.appState.sdk.ai.generateAiPassage(
        wrongWords: wrongWords,
        targetWords: targetWords,
        level: 'intermediate',
      );
      _triggerReload();
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _aiMessage = error.toString();
      });
    } finally {
      if (mounted) {
        setState(() {
          _aiGenerating = false;
        });
      }
    }
  }

  Future<void> _openPlan() async {
    if (widget.onOpenPlan != null) {
      await widget.onOpenPlan!.call();
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(builder: (_) => PlanScreen(sdk: widget.appState.sdk)),
    );
    _triggerReload();
  }

  Future<void> _openStudy([
    String mode = 'newWord',
    ResumeSessionHint? resumeHint,
  ]) async {
    if (widget.onOpenStudy != null) {
      await widget.onOpenStudy!.call(mode, resumeHint);
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => StudyScreen(
          sdk: widget.appState.sdk,
          mode: mode,
          resumeHint: resumeHint,
        ),
      ),
    );
    _triggerReload();
  }

  // ignore: unused_element
  Future<void> _openReports() async {
    if (widget.onOpenReports != null) {
      await widget.onOpenReports!.call();
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => ReportsScreen(sdk: widget.appState.sdk),
      ),
    );
  }

  // ignore: unused_element
  Future<void> _openWrongWords() async {
    if (widget.onOpenWrongWords != null) {
      await widget.onOpenWrongWords!.call();
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => WrongWordsScreen(sdk: widget.appState.sdk),
      ),
    );
  }

  Future<void> _openAi({
    bool generateOnOpen = false,
    bool showPassageFirst = false,
  }) async {
    if (widget.onOpenAi != null) {
      await widget.onOpenAi!.call(
        generateOnOpen: generateOnOpen,
        showPassageFirst: showPassageFirst,
      );
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => AiScreen(
          sdk: widget.appState.sdk,
          generateOnOpen: generateOnOpen,
          showPassageFirst: showPassageFirst,
        ),
      ),
    );
  }

  // ignore: unused_element
  Future<void> _openAccount() async {
    if (widget.onOpenAccount != null) {
      await widget.onOpenAccount!.call();
      return;
    }
    await Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) =>
            AuthScreen(onAuthChanged: widget.appState.applyAuthState),
      ),
    );
    _triggerReload();
  }

  Future<void> _applyPlanToToday() async {
    try {
      await widget.appState.sdk.plan.applySavedPlanToToday();
      _triggerReload();
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('已同步计划到 Today')));
      }
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('同步失败：$error')));
      }
    }
  }

  void _triggerReload() {
    if (!mounted) return;
    setState(() {
      _reloadToken++;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Today'),
        actions: [
          IconButton(
            onPressed: () => widget.appState.retryInitialize(),
            tooltip: 'Refresh bootstrap',
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: FutureBuilder<_TodayHomeBundle>(
        key: ValueKey(_reloadToken),
        future: _loadHomeBundle(),
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }

          if (snapshot.hasError) {
            final error = snapshot.error;
            final message = error is Exception
                ? error.toString()
                : 'Unknown today error';
            return _SectionCard(
              title: 'Today load failed',
              child: Text(message),
            );
          }

          final bundle = snapshot.data;
          if (bundle == null) {
            return const _SectionCard(
              title: 'No today payload',
              child: Text('Bridge returned no Today payload.'),
            );
          }

          final snapshotMap = _snapshotOrPlanFallback(
            bundle.today.todaySnapshot,
            bundle.activePlan,
          );
          final hasSnapshot = _hasUsableSnapshot(snapshotMap);
          final primaryAction = _buildPrimaryAction(
            snapshotMap,
            hasSnapshot: hasSnapshot,
            hasActivePlan: bundle.activePlan != null,
          );
          final completion = hasSnapshot
              ? _calculateCompletion(snapshotMap, bundle.activePlan)
              : null;
          final taskItems = _buildTaskItems(snapshotMap, bundle.activePlan);
          final tasksComplete =
              bundle.aiContext?.tasksComplete ?? ((completion ?? 0) >= 100);

          Future<void> handlePrimaryAction() async {
            switch (primaryAction.mode) {
              case 'planSetup':
                await _openPlan();
                return;
              case 'syncToday':
                await _applyPlanToToday();
                return;
              case 'done':
                await _openAi();
                return;
              default:
                await _openStudy(primaryAction.mode);
                return;
            }
          }

          return ListView(
            padding: const EdgeInsets.all(16),
            children: [
              _PrimaryActionCard(
                action: primaryAction,
                completion: completion,
                todayDate: bundle.today.todayDate,
                onStart: handlePrimaryAction,
              ),
              _TaskBreakdownCard(
                items: taskItems,
                onStartStudy: _openStudy,
                hasSnapshot: hasSnapshot,
                onOpenPlan: _openPlan,
                onSyncToday: _applyPlanToToday,
                hasActivePlan: bundle.activePlan != null,
              ),
              _AiShortcutCard(
                sdk: widget.appState.sdk,
                context: bundle.aiContext,
                history: bundle.aiHistory,
                isGenerating: _aiGenerating,
                message: _aiMessage,
                onGenerate: () => _generateAiPassage(bundle.aiContext),
              ),
              _RewardSlotMachineCard(
                sdk: widget.appState.sdk,
                tasksComplete: tasksComplete,
                rewardState: bundle.rewardState,
              ),
              _SectionCard(
                title: 'Developer diagnostics',
                child: ExpansionTile(
                  tilePadding: EdgeInsets.zero,
                  childrenPadding: EdgeInsets.zero,
                  title: const Text('Show bootstrap / account / sync details'),
                  children: [
                    Text('Today date: ${bundle.today.todayDate}'),
                    Text(
                      'App ready: ${widget.appState.bootstrapState?.appReady ?? false}',
                    ),
                    Text(
                      'First run required: '
                      '${widget.appState.bootstrapState?.firstRunRequired ?? false}',
                    ),
                    Text(
                      'Account: ${_accountPhaseLabel(widget.appState.authPhase, widget.appState.authState.userEmail)}',
                    ),
                    Text(
                      'Local study allowed: ${widget.appState.authState.allowsLocalStudy}',
                    ),
                    Text(
                      'Cloud sync eligible: ${widget.appState.authState.allowsCloudWork}',
                    ),
                    if (widget.appState.authMessage != null)
                      Text('Account note: ${widget.appState.authMessage}'),
                    const SizedBox(height: 12),
                    if (bundle.syncStatus != null)
                      _SyncStatusSummary(status: bundle.syncStatus!),
                    if (bundle.syncStatus == null)
                      const Text('Sync status unavailable in this build.'),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _TodayHomeBundle {
  const _TodayHomeBundle({
    required this.today,
    required this.activePlan,
    required this.aiContext,
    required this.aiHistory,
    required this.rewardState,
    required this.syncStatus,
  });

  final TodayHomeState today;
  final PlanSummary? activePlan;
  final TodayAiPassageContext? aiContext;
  final List<AiPassageHistoryItem> aiHistory;
  final TodayRewardState? rewardState;
  final SyncStatus? syncStatus;
}

class _PrimaryAction {
  const _PrimaryAction({
    required this.label,
    required this.count,
    required this.mode,
    required this.isDone,
    required this.buttonLabel,
    required this.description,
  });

  final String label;
  final int count;
  final String mode;
  final bool isDone;
  final String buttonLabel;
  final String description;
}

class _TaskItemViewModel {
  const _TaskItemViewModel({
    required this.label,
    required this.helper,
    required this.completed,
    required this.target,
    required this.color,
    required this.mode,
  });

  final String label;
  final String helper;
  final int completed;
  final int target;
  final Color color;
  final String mode;
}

bool _hasUsableSnapshot(Map<String, dynamic> snapshot) {
  if (snapshot.isEmpty) return false;
  return snapshot.keys.any((key) => key.toLowerCase().contains('target'));
}

Map<String, dynamic> _snapshotOrPlanFallback(
  Map<String, dynamic>? snapshot,
  PlanSummary? activePlan,
) {
  final safeSnapshot = snapshot ?? const <String, dynamic>{};
  if (_hasUsableSnapshot(safeSnapshot)) {
    return safeSnapshot;
  }
  if (activePlan == null) {
    return safeSnapshot;
  }
  return <String, dynamic>{
    'date': DateTime.now().toIso8601String().split('T').first,
    'newWordsTarget': activePlan.newWordsPerDay * 4,
    'newWordsBaseTarget': activePlan.newWordsPerDay * 4,
    'newWordsCarryoverTarget': 0,
    'newWordsCompleted': 0,
    'reviewWordsTarget': activePlan.reviewWordsPerDay * 4,
    'reviewWordsBaseTarget': activePlan.reviewWordsPerDay * 4,
    'reviewWordsCarryoverTarget': 0,
    'reviewWordsCompleted': 0,
    'mixedTestTarget': activePlan.mixedTestPerDay,
    'mixedTestBaseTarget': activePlan.mixedTestPerDay,
    'mixedTestCarryoverTarget': 0,
    'mixedTestCompleted': 0,
    'wrongWordTestTarget': activePlan.wrongWordTestPerDay,
    'wrongWordTestBaseTarget': activePlan.wrongWordTestPerDay,
    'wrongWordTestCarryoverTarget': 0,
    'wrongWordTestCompleted': 0,
    'rootAffixTarget': activePlan.rootAffixPerDay ?? 0,
    'rootAffixBaseTarget': activePlan.rootAffixPerDay ?? 0,
    'rootAffixCarryoverTarget': 0,
    'rootAffixCompleted': 0,
  };
}

_PrimaryAction _buildPrimaryAction(
  Map<String, dynamic> snapshot, {
  required bool hasSnapshot,
  required bool hasActivePlan,
}) {
  if (!hasSnapshot) {
    if (hasActivePlan) {
      return const _PrimaryAction(
        label: '生成今日任务',
        count: 0,
        mode: 'syncToday',
        isDone: false,
        buttonLabel: '同步到 Today',
        description: '当前已有计划，但今天的任务快照还没有生成。',
      );
    }
    return const _PrimaryAction(
      label: '先配置计划',
      count: 0,
      mode: 'planSetup',
      isDone: false,
      buttonLabel: '前往计划页',
      description: '先在计划页设置每天任务量和词书，再开始学习。',
    );
  }

  final newWordsCompleted = _intValue(snapshot, 'newWordsCompleted');
  final newWordsTarget = _intValue(snapshot, 'newWordsTarget');
  if (newWordsCompleted < newWordsTarget) {
    return _PrimaryAction(
      label: '学习新词',
      count: newWordsTarget - newWordsCompleted,
      mode: 'newWord',
      isDone: false,
      buttonLabel: '进入新词学习',
      description: '先把今天的新词任务推进起来。',
    );
  }

  final reviewCompleted = _intValue(snapshot, 'reviewWordsCompleted');
  final reviewTarget = _intValue(snapshot, 'reviewWordsTarget');
  if (reviewCompleted < reviewTarget) {
    return _PrimaryAction(
      label: '继续复习',
      count: reviewTarget - reviewCompleted,
      mode: 'review',
      isDone: false,
      buttonLabel: '进入复习',
      description: '回收旧词，巩固今天的学习稳定度。',
    );
  }

  final mixedCompleted = _intValue(snapshot, 'mixedTestCompleted');
  final mixedTarget = _intValue(snapshot, 'mixedTestTarget');
  if (mixedCompleted < mixedTarget) {
    return _PrimaryAction(
      label: '开始混合测试',
      count: mixedTarget - mixedCompleted,
      mode: 'mixedTest',
      isDone: false,
      buttonLabel: '进入混合测试',
      description: '现在进入综合测试，检查今天的掌握情况。',
    );
  }

  final wrongWordsCompleted = _intValue(snapshot, 'wrongWordTestCompleted');
  final wrongWordsTarget = _intValue(snapshot, 'wrongWordTestTarget');
  if (wrongWordsCompleted < wrongWordsTarget) {
    return _PrimaryAction(
      label: '进行错词强化',
      count: wrongWordsTarget - wrongWordsCompleted,
      mode: 'wrongWordReinforcement',
      isDone: false,
      buttonLabel: '进入错词强化',
      description: '优先回收今天暴露出来的薄弱点。',
    );
  }

  final rootAffixCompleted = _intValue(snapshot, 'rootAffixCompleted');
  final rootAffixTarget = _intValue(snapshot, 'rootAffixTarget');
  if (rootAffixCompleted < rootAffixTarget) {
    return _PrimaryAction(
      label: '学习词根词缀',
      count: rootAffixTarget - rootAffixCompleted,
      mode: 'rootAffix',
      isDone: false,
      buttonLabel: '进入词根词缀',
      description: '补充结构化记忆，帮助长期保持。',
    );
  }

  return const _PrimaryAction(
    label: '今日任务完成',
    count: 0,
    mode: 'done',
    isDone: true,
    buttonLabel: '查看 AI / 复盘',
    description: '你可以回顾报告、错词本或进入 AI 短文。',
  );
}

int _calculateCompletion(
  Map<String, dynamic> snapshot,
  PlanSummary? activePlan,
) {
  final newWordsTarget = _displayTarget(snapshot, 'newWord', activePlan);
  final reviewWordsTarget = _displayTarget(snapshot, 'review', activePlan);
  final mixedTestTarget = _displayTarget(snapshot, 'mixedTest', activePlan);
  final wrongWordTarget = _displayTarget(
    snapshot,
    'wrongWordReinforcement',
    activePlan,
  );
  final rootAffixTarget = _displayTarget(snapshot, 'rootAffix', activePlan);
  final total =
      newWordsTarget +
      reviewWordsTarget +
      mixedTestTarget +
      wrongWordTarget +
      rootAffixTarget;
  final completed =
      _cappedCompleted(snapshot, 'newWordsCompleted', newWordsTarget) +
      _cappedCompleted(snapshot, 'reviewWordsCompleted', reviewWordsTarget) +
      _cappedCompleted(snapshot, 'mixedTestCompleted', mixedTestTarget) +
      _cappedCompleted(snapshot, 'wrongWordTestCompleted', wrongWordTarget) +
      _cappedCompleted(snapshot, 'rootAffixCompleted', rootAffixTarget);
  if (total <= 0) return 0;
  return ((completed / total) * 100).round().clamp(0, 100);
}

int _cappedCompleted(Map<String, dynamic> snapshot, String key, int target) {
  final completed = _intValue(snapshot, key);
  if (target <= 0) return 0;
  return completed.clamp(0, target);
}

List<_TaskItemViewModel> _buildTaskItems(
  Map<String, dynamic> snapshot,
  PlanSummary? activePlan,
) {
  return [
    _TaskItemViewModel(
      label: '新词学习',
      helper: '建立今日新词基础',
      target: _displayTarget(snapshot, 'newWord', activePlan),
      completed: _cappedCompleted(
        snapshot,
        'newWordsCompleted',
        _displayTarget(snapshot, 'newWord', activePlan),
      ),
      color: const Color(0xFF2F8F6A),
      mode: 'newWord',
    ),
    _TaskItemViewModel(
      label: '复习',
      helper: '回顾旧词，巩固记忆',
      target: _displayTarget(snapshot, 'review', activePlan),
      completed: _cappedCompleted(
        snapshot,
        'reviewWordsCompleted',
        _displayTarget(snapshot, 'review', activePlan),
      ),
      color: const Color(0xFF2F6C8F),
      mode: 'review',
    ),
    _TaskItemViewModel(
      label: '混合测试',
      helper: '综合检验今日状态',
      target: _displayTarget(snapshot, 'mixedTest', activePlan),
      completed: _cappedCompleted(
        snapshot,
        'mixedTestCompleted',
        _displayTarget(snapshot, 'mixedTest', activePlan),
      ),
      color: const Color(0xFF8F5A2F),
      mode: 'mixedTest',
    ),
    _TaskItemViewModel(
      label: '错词强化',
      helper: '回收今天的薄弱点',
      target: _displayTarget(snapshot, 'wrongWordReinforcement', activePlan),
      completed: _cappedCompleted(
        snapshot,
        'wrongWordTestCompleted',
        _displayTarget(snapshot, 'wrongWordReinforcement', activePlan),
      ),
      color: const Color(0xFF8F3B4D),
      mode: 'wrongWordReinforcement',
    ),
    _TaskItemViewModel(
      label: '词根词缀',
      helper: '补充结构化记忆',
      target: _displayTarget(snapshot, 'rootAffix', activePlan),
      completed: _cappedCompleted(
        snapshot,
        'rootAffixCompleted',
        _displayTarget(snapshot, 'rootAffix', activePlan),
      ),
      color: const Color(0xFF7C52A1),
      mode: 'rootAffix',
    ),
  ].where((item) => item.target > 0).toList(growable: false);
}

int _displayTarget(
  Map<String, dynamic> snapshot,
  String mode,
  PlanSummary? activePlan,
) {
  final snapshotTarget = switch (mode) {
    'newWord' => _intValue(snapshot, 'newWordsTarget'),
    'review' => _intValue(snapshot, 'reviewWordsTarget'),
    'mixedTest' => _intValue(snapshot, 'mixedTestTarget'),
    'wrongWordReinforcement' => _intValue(snapshot, 'wrongWordTestTarget'),
    'rootAffix' => _intValue(snapshot, 'rootAffixTarget'),
    _ => 0,
  };
  if (snapshotTarget > 0 || activePlan == null) {
    return snapshotTarget;
  }
  return switch (mode) {
    'newWord' => activePlan.newWordsPerDay * 4,
    'review' => activePlan.reviewWordsPerDay * 4,
    'mixedTest' => activePlan.mixedTestPerDay,
    'wrongWordReinforcement' => activePlan.wrongWordTestPerDay,
    'rootAffix' => activePlan.rootAffixPerDay ?? 0,
    _ => 0,
  };
}

List<String> todayTaskBreakdownModesForTest(
  Map<String, dynamic> snapshot,
  PlanSummary? activePlan,
) {
  return _buildTaskItems(
    snapshot,
    activePlan,
  ).map((item) => item.mode).toList(growable: false);
}

List<String> todayTaskBreakdownRowsForTest(
  Map<String, dynamic> snapshot,
  PlanSummary? activePlan,
) {
  return _buildTaskItems(snapshot, activePlan)
      .map((item) => '${item.mode}:${item.completed}/${item.target}')
      .toList(growable: false);
}

int todayCompletionForTest(
  Map<String, dynamic> snapshot,
  PlanSummary? activePlan,
) {
  return _calculateCompletion(snapshot, activePlan);
}

int _intValue(Map<String, dynamic> source, String key) {
  final value = source[key];
  if (value is int) return value;
  if (value is num) return value.toInt();
  return 0;
}

String _accountPhaseLabel(AuthAccountPhase phase, String? userEmail) {
  return switch (phase) {
    AuthAccountPhase.uninitialized => 'uninitialized',
    AuthAccountPhase.checking => 'checking session',
    AuthAccountPhase.notConfigured => 'not configured',
    AuthAccountPhase.guestLocalOnly => 'guest local-only',
    AuthAccountPhase.signedInActive => userEmail ?? 'signed in',
    AuthAccountPhase.signedInExpired => 'signed in expired',
    AuthAccountPhase.signedOutRetainedLocal => 'signed out, local retained',
    AuthAccountPhase.accountDeletedOrRevoked => 'account deleted or revoked',
    AuthAccountPhase.error => 'auth error',
  };
}

class _PrimaryActionCard extends StatelessWidget {
  const _PrimaryActionCard({
    required this.action,
    required this.completion,
    required this.todayDate,
    required this.onStart,
  });

  final _PrimaryAction action;
  final int? completion;
  final String todayDate;
  final Future<void> Function() onStart;

  @override
  Widget build(BuildContext context) {
    final color = action.isDone
        ? const Color(0xFF2F8F6A)
        : const Color(0xFF1F6F5E);
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      color: color,
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              todayDate,
              style: Theme.of(
                context,
              ).textTheme.labelLarge?.copyWith(color: Colors.white70),
            ),
            const SizedBox(height: 8),
            Text(
              action.label,
              style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                color: Colors.white,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 8),
            Text(
              action.description,
              style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                color: Colors.white.withValues(alpha: 0.92),
              ),
            ),
            if (completion != null) ...[
              const SizedBox(height: 16),
              ClipRRect(
                borderRadius: BorderRadius.circular(999),
                child: LinearProgressIndicator(
                  value: completion! / 100,
                  minHeight: 10,
                  backgroundColor: Colors.white24,
                  valueColor: const AlwaysStoppedAnimation<Color>(Colors.white),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                '今日完成度 $completion%',
                style: Theme.of(
                  context,
                ).textTheme.bodyMedium?.copyWith(color: Colors.white70),
              ),
            ],
            if (action.count > 0) ...[
              const SizedBox(height: 8),
              Text(
                '剩余 ${action.count} 个任务单位',
                style: Theme.of(
                  context,
                ).textTheme.bodySmall?.copyWith(color: Colors.white70),
              ),
            ],
            const SizedBox(height: 20),
            FilledButton.tonal(
              onPressed: onStart,
              style: FilledButton.styleFrom(
                backgroundColor: Colors.white,
                foregroundColor: color,
                minimumSize: const Size.fromHeight(48),
              ),
              child: Text(action.buttonLabel),
            ),
          ],
        ),
      ),
    );
  }
}

class _TaskBreakdownCard extends StatelessWidget {
  const _TaskBreakdownCard({
    required this.items,
    required this.onStartStudy,
    required this.hasSnapshot,
    required this.onOpenPlan,
    required this.onSyncToday,
    required this.hasActivePlan,
  });

  final List<_TaskItemViewModel> items;
  final Future<void> Function(String mode) onStartStudy;
  final bool hasSnapshot;
  final Future<void> Function() onOpenPlan;
  final Future<void> Function() onSyncToday;
  final bool hasActivePlan;

  @override
  Widget build(BuildContext context) {
    return _SectionCard(
      title: '今日任务拆解',
      trailing: TextButton.icon(
        onPressed: onOpenPlan,
        icon: const Icon(Icons.edit_outlined, size: 18),
        label: const Text('修改'),
      ),
      child: !hasSnapshot
          ? Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  hasActivePlan
                      ? '今天的任务还没有从当前计划展开。先同步到 Today，再开始学习。'
                      : '当前还没有可执行的 Today 任务，请先去计划页配置。',
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(
                      child: FilledButton(
                        onPressed: hasActivePlan ? onSyncToday : onOpenPlan,
                        child: Text(hasActivePlan ? '同步到 Today' : '前往计划页'),
                      ),
                    ),
                  ],
                ),
              ],
            )
          : Column(
              children: [
                for (var i = 0; i < items.length; i++) ...[
                  _TaskProgressRow(
                    item: items[i],
                    onTap: () => onStartStudy(items[i].mode),
                  ),
                  if (i != items.length - 1) const SizedBox(height: 12),
                ],
              ],
            ),
    );
  }
}

class _TaskProgressRow extends StatelessWidget {
  const _TaskProgressRow({required this.item, required this.onTap});

  final _TaskItemViewModel item;
  final Future<void> Function() onTap;

  @override
  Widget build(BuildContext context) {
    final progress = item.target <= 0 ? 0.0 : item.completed / item.target;
    final done = item.completed >= item.target;
    return InkWell(
      onTap: done ? null : onTap,
      borderRadius: BorderRadius.circular(16),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            width: 12,
            height: 12,
            margin: const EdgeInsets.only(top: 6),
            decoration: BoxDecoration(
              color: item.color,
              borderRadius: BorderRadius.circular(999),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        item.label,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                    ),
                    Text('${item.completed}/${item.target}'),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  item.helper,
                  style: Theme.of(
                    context,
                  ).textTheme.bodySmall?.copyWith(color: Colors.black54),
                ),
                const SizedBox(height: 8),
                ClipRRect(
                  borderRadius: BorderRadius.circular(999),
                  child: LinearProgressIndicator(
                    value: progress.clamp(0.0, 1.0),
                    minHeight: 8,
                    backgroundColor: item.color.withValues(alpha: 0.15),
                    valueColor: AlwaysStoppedAnimation<Color>(item.color),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          Icon(
            done ? Icons.check_circle : Icons.chevron_right,
            color: item.color,
          ),
        ],
      ),
    );
  }
}

class _AiShortcutCard extends StatefulWidget {
  const _AiShortcutCard({
    required this.sdk,
    required this.context,
    required this.history,
    required this.isGenerating,
    required this.message,
    required this.onGenerate,
  });

  final WordSdk sdk;
  final TodayAiPassageContext? context;
  final List<AiPassageHistoryItem> history;
  final bool isGenerating;
  final String? message;
  final Future<void> Function() onGenerate;

  @override
  State<_AiShortcutCard> createState() => _AiShortcutCardState();
}

class _AiShortcutCardState extends State<_AiShortcutCard> {
  AiPassage? _passage;
  String? _loadedPassageId;
  bool _loadingPassage = false;

  @override
  void initState() {
    super.initState();
    _loadLatestPassage();
  }

  @override
  void didUpdateWidget(covariant _AiShortcutCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    final oldId = oldWidget.history.isEmpty
        ? null
        : oldWidget.history.first.passageId;
    final nextId = widget.history.isEmpty
        ? null
        : widget.history.first.passageId;
    if (oldId != nextId) {
      _passage = null;
      _loadedPassageId = null;
      _loadLatestPassage();
    }
  }

  Future<void> _loadLatestPassage() async {
    if (widget.history.isEmpty) return;
    final passageId = widget.history.first.passageId;
    if (_loadedPassageId == passageId || _loadingPassage) return;

    setState(() {
      _loadingPassage = true;
    });

    try {
      final passage = await widget.sdk.ai.getAiPassage(passageId);
      if (!mounted) return;
      setState(() {
        _passage = passage;
        _loadedPassageId = passageId;
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _loadedPassageId = passageId;
      });
    } finally {
      if (mounted) {
        setState(() {
          _loadingPassage = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final wrongWordCount = widget.context?.wrongWords.length ?? 0;
    final ready = widget.context?.tasksComplete ?? false;
    final latest = widget.history.isEmpty ? null : widget.history.first;
    final hasPassage = _passage != null;

    return _SectionCard(
      title: 'AI \u77ed\u6587\u603b\u7ed3',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            ready
                ? hasPassage
                      ? '\u6700\u8fd1\u4e00\u7bc7 AI \u77ed\u6587\u5df2\u751f\u6210\uff0c\u53ef\u4ee5\u76f4\u63a5\u9605\u8bfb\u3002'
                      : '\u4eca\u65e5\u4efb\u52a1\u5df2\u5b8c\u6210\uff0c\u53ef\u4ee5\u751f\u6210 AI \u77ed\u6587\u3002'
                : '\u5b8c\u6210\u4eca\u65e5\u4efb\u52a1\u540e\u518d\u751f\u6210 AI \u77ed\u6587\u3002',
          ),
          const SizedBox(height: 8),
          Text('\u4eca\u65e5\u9519\u8bcd\u6570\uff1a$wrongWordCount'),
          if (latest != null) ...[
            const SizedBox(height: 8),
            Text('\u6700\u8fd1\u4e00\u7bc7\uff1a${latest.title}'),
          ],
          const SizedBox(height: 12),
          if (hasPassage)
            _TodayAiPassagePreview(passage: _passage!)
          else if (_loadingPassage)
            const LinearProgressIndicator(minHeight: 2)
          else
            FilledButton.tonal(
              onPressed: widget.isGenerating || !ready
                  ? null
                  : widget.onGenerate,
              child: Text(
                widget.isGenerating
                    ? '\u751f\u6210\u4e2d...'
                    : '\u751f\u6210 AI \u77ed\u6587',
              ),
            ),
          if (widget.message != null) ...[
            const SizedBox(height: 12),
            Text(
              widget.message!,
              style: const TextStyle(color: Colors.redAccent),
            ),
          ],
        ],
      ),
    );
  }
}

class _TodayAiPassagePreview extends StatelessWidget {
  const _TodayAiPassagePreview({required this.passage});

  final AiPassage passage;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          passage.title,
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 6),
        Text('Status: ${passage.validationStatus}'),
        if (passage.failureReason != null) ...[
          const SizedBox(height: 6),
          Text(
            passage.failureReason!,
            style: const TextStyle(color: Colors.redAccent),
          ),
        ],
        if (passage.blocks.isNotEmpty) ...[
          const SizedBox(height: 10),
          for (final block in passage.blocks)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: _TodayAiBlockView(block: block),
            ),
        ],
      ],
    );
  }
}

class _TodayAiBlockView extends StatelessWidget {
  const _TodayAiBlockView({required this.block});

  final dynamic block;

  @override
  Widget build(BuildContext context) {
    if (block is String) {
      return Text(block as String);
    }

    if (block is Map) {
      final segments = block['segments'];
      if (segments is List) {
        return RichText(
          text: TextSpan(
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(color: Colors.black87),
            children: segments
                .map<InlineSpan>((segment) {
                  if (segment is! Map) {
                    return TextSpan(text: '$segment');
                  }

                  final text = '${segment['text'] ?? ''}';
                  final gloss = '${segment['glossZh'] ?? ''}';
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
                      if (gloss.isNotEmpty)
                        TextSpan(
                          text: '\uff08$gloss\uff09',
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

      return Text('${block['text'] ?? block['content'] ?? block}');
    }

    return Text('$block');
  }
}

class _RewardSlotMachineCard extends StatefulWidget {
  const _RewardSlotMachineCard({
    required this.sdk,
    required this.tasksComplete,
    required this.rewardState,
  });

  final WordSdk sdk;
  final bool tasksComplete;
  final TodayRewardState? rewardState;

  @override
  State<_RewardSlotMachineCard> createState() => _RewardSlotMachineCardState();
}

class _RewardSlotMachineCardState extends State<_RewardSlotMachineCard> {
  late Future<List<_RewardItem>> _catalogFuture;
  TodayRewardState? _rewardState;
  _RewardItem? _previewReward;
  bool _spinning = false;
  bool _saving = false;
  String? _message;

  @override
  void initState() {
    super.initState();
    _catalogFuture = _loadRewardCatalog();
    _rewardState = widget.rewardState;
  }

  @override
  void didUpdateWidget(covariant _RewardSlotMachineCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.rewardState?.rewardId != widget.rewardState?.rewardId) {
      _rewardState = widget.rewardState;
    }
  }

  Future<void> _pullLever(List<_RewardItem> rewards) async {
    if (!widget.tasksComplete || _spinning || rewards.isEmpty) return;
    if (_rewardState?.hasReward ?? false) return;

    final random = Random();
    setState(() {
      _spinning = true;
      _message = null;
    });

    for (var i = 0; i < 14; i++) {
      await Future<void>.delayed(Duration(milliseconds: 45 + i * 8));
      if (!mounted) return;
      setState(() {
        _previewReward = rewards[random.nextInt(rewards.length)];
      });
    }

    final picked = rewards[random.nextInt(rewards.length)];
    if (!mounted) return;
    setState(() {
      _previewReward = picked;
      _spinning = false;
    });
  }

  Future<void> _saveReward() async {
    final reward = _previewReward;
    if (reward == null || (_rewardState?.hasReward ?? false)) {
      return;
    }

    setState(() {
      _saving = true;
      _message = null;
    });

    try {
      if (reward.assetPath != null) {
        await widget.sdk.rewards.saveRewardImageToGallery(
          reward.assetPath!,
          reward.id,
        );
      }
      final state = await widget.sdk.rewards.saveTodayReward(reward.id);
      if (!mounted) return;
      setState(() {
        _rewardState = state;
        _previewReward = reward;
        _message = '已保存到相册';
      });
    } catch (error) {
      if (!mounted) return;
      setState(() {
        _message = '保存失败：$error';
      });
    } finally {
      if (mounted) {
        setState(() {
          _saving = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<List<_RewardItem>>(
      future: _catalogFuture,
      builder: (context, snapshot) {
        final rewards = snapshot.data ?? const <_RewardItem>[];
        final claimedReward =
            _findReward(rewards, _rewardState?.rewardId) ?? _previewReward;
        final hasClaim = _rewardState?.hasReward ?? false;
        final ready = widget.tasksComplete && !hasClaim;
        final locked = !widget.tasksComplete;
        final canSave =
            ready && !_spinning && !_saving && _previewReward != null;
        final title = hasClaim
            ? '今日奖励已领取'
            : locked
            ? '完成任务后领取奖励'
            : _previewReward == null
            ? '拉动拉杆抽取奖励'
            : '奖励已抽取';

        return _SectionCard(
          title: '今日奖励',
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: Theme.of(context).textTheme.titleSmall),
              const SizedBox(height: 10),
              Row(
                crossAxisAlignment: CrossAxisAlignment.center,
                children: [
                  _RewardLever(
                    enabled: ready && !_spinning && rewards.isNotEmpty,
                    spinning: _spinning,
                    claimed: hasClaim,
                    onPull: () => _pullLever(rewards),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: _RewardImagePanel(
                      reward: claimedReward,
                      spinning: _spinning,
                      locked: locked,
                    ),
                  ),
                ],
              ),
              if (!locked && !hasClaim && _previewReward != null) ...[
                const SizedBox(height: 12),
                Align(
                  alignment: Alignment.centerRight,
                  child: FilledButton.tonalIcon(
                    onPressed: canSave ? _saveReward : null,
                    icon: const Icon(Icons.save_alt_outlined),
                    label: Text(_saving ? '保存中' : '保存奖励'),
                  ),
                ),
              ],
              if (_message != null) ...[
                const SizedBox(height: 8),
                Text(
                  _message!,
                  style: TextStyle(
                    color: _message!.startsWith('保存失败')
                        ? Colors.redAccent
                        : const Color(0xFF3F8F72),
                  ),
                ),
              ],
            ],
          ),
        );
      },
    );
  }
}

class _RewardLever extends StatelessWidget {
  const _RewardLever({
    required this.enabled,
    required this.spinning,
    required this.claimed,
    required this.onPull,
  });

  final bool enabled;
  final bool spinning;
  final bool claimed;
  final VoidCallback onPull;

  @override
  Widget build(BuildContext context) {
    final active = enabled || spinning || claimed;
    final metalColor = active ? const Color(0xFF3E4A45) : Colors.black26;
    final knobColor = active ? const Color(0xFFE0565B) : Colors.black26;
    final cabinetColor = active ? const Color(0xFFEAF4F2) : Colors.black12;
    return Semantics(
      button: true,
      enabled: enabled,
      label: '抽取今日奖励',
      child: GestureDetector(
        onTap: enabled ? onPull : null,
        child: SizedBox(
          width: 88,
          height: 148,
          child: Stack(
            clipBehavior: Clip.none,
            children: [
              Positioned(
                bottom: 0,
                left: 12,
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 180),
                  width: 52,
                  height: 116,
                  decoration: BoxDecoration(
                    color: cabinetColor,
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.black12),
                    boxShadow: const [
                      BoxShadow(
                        color: Colors.black12,
                        blurRadius: 8,
                        offset: Offset(0, 4),
                      ),
                    ],
                  ),
                  child: Column(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      Container(
                        width: 34,
                        height: 10,
                        decoration: BoxDecoration(
                          color: active
                              ? const Color(0xFF3F8F72)
                              : Colors.black12,
                          borderRadius: BorderRadius.circular(999),
                        ),
                      ),
                      const SizedBox(height: 12),
                      Container(
                        width: 34,
                        height: 34,
                        decoration: BoxDecoration(
                          color: Colors.white.withValues(alpha: 0.58),
                          borderRadius: BorderRadius.circular(6),
                          border: Border.all(color: Colors.black12),
                        ),
                      ),
                      const SizedBox(height: 12),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          _LeverScrew(active: active),
                          const SizedBox(width: 14),
                          _LeverScrew(active: active),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
              Positioned(
                left: 48,
                top: 58,
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 180),
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    color: metalColor,
                    shape: BoxShape.circle,
                    border: Border.all(color: Colors.white70, width: 3),
                    boxShadow: const [
                      BoxShadow(
                        color: Colors.black26,
                        blurRadius: 6,
                        offset: Offset(0, 3),
                      ),
                    ],
                  ),
                ),
              ),
              Positioned(
                left: 38,
                top: spinning ? 48 : 8,
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 240),
                  curve: Curves.easeInOutCubic,
                  width: 38,
                  height: 38,
                  decoration: BoxDecoration(
                    color: knobColor,
                    shape: BoxShape.circle,
                    border: Border.all(color: Colors.white70, width: 2),
                    boxShadow: active
                        ? const [
                            BoxShadow(
                              color: Colors.black26,
                              blurRadius: 8,
                              offset: Offset(0, 4),
                            ),
                          ]
                        : null,
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

class _LeverScrew extends StatelessWidget {
  const _LeverScrew({required this.active});

  final bool active;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 7,
      height: 7,
      decoration: BoxDecoration(
        color: active ? const Color(0xFF3E4A45) : Colors.black26,
        shape: BoxShape.circle,
      ),
    );
  }
}

class _RewardImagePanel extends StatelessWidget {
  const _RewardImagePanel({
    required this.reward,
    required this.spinning,
    required this.locked,
  });

  final _RewardItem? reward;
  final bool spinning;
  final bool locked;

  @override
  Widget build(BuildContext context) {
    final item = reward;
    final colors =
        item?.colors ??
        const [Color(0xFFEAF4F2), Color(0xFF4A9B78), Color(0xFFF4C95D)];
    return AnimatedSwitcher(
      duration: const Duration(milliseconds: 180),
      child: AspectRatio(
        key: ValueKey('${item?.id ?? 'empty'}-$spinning-$locked'),
        aspectRatio: 1.05,
        child: Container(
          width: double.infinity,
          padding: const EdgeInsets.all(8),
          decoration: BoxDecoration(
            gradient: LinearGradient(colors: colors),
            borderRadius: BorderRadius.circular(8),
            border: Border.all(color: Colors.black12),
          ),
          child: locked
              ? const Center(child: Icon(Icons.lock_outline, size: 42))
              : item?.assetPath != null
              ? ClipRRect(
                  borderRadius: BorderRadius.circular(6),
                  child: Image.asset(
                    item!.assetPath!,
                    fit: BoxFit.contain,
                    width: double.infinity,
                    height: double.infinity,
                  ),
                )
              : Stack(
                  children: [
                    Positioned.fill(
                      child: DecoratedBox(
                        decoration: BoxDecoration(
                          gradient: RadialGradient(
                            center: Alignment.topLeft,
                            radius: 1.2,
                            colors: colors,
                          ),
                        ),
                      ),
                    ),
                    Center(
                      child: Icon(
                        spinning ? Icons.blur_on : Icons.image_outlined,
                        color: Colors.white.withValues(alpha: 0.9),
                        size: 44,
                      ),
                    ),
                  ],
                ),
        ),
      ),
    );
  }
}

class _RewardItem {
  const _RewardItem({
    required this.id,
    required this.title,
    required this.rarity,
    required this.assetPath,
    required this.colors,
  });

  final String id;
  final String title;
  final String rarity;
  final String? assetPath;
  final List<Color> colors;

  factory _RewardItem.fromJson(Map<String, dynamic> json) {
    final palette = json['palette'] as List<dynamic>? ?? const [];
    return _RewardItem(
      id: json['id'] as String,
      title: json['title'] as String? ?? json['id'] as String,
      rarity: json['rarity'] as String? ?? 'common',
      assetPath: json['assetPath'] as String?,
      colors: palette
          .whereType<String>()
          .map(_parseHexColor)
          .toList(growable: false),
    );
  }
}

Future<List<_RewardItem>> _loadRewardCatalog() async {
  final raw = await rootBundle.loadString('assets/rewards/manifest.json');
  final decoded = jsonDecode(raw);
  if (decoded is! Map<String, dynamic>) return const [];
  final rewards = decoded['rewards'];
  if (rewards is! List) return const [];
  return rewards
      .whereType<Map<String, dynamic>>()
      .map(_RewardItem.fromJson)
      .toList(growable: false);
}

_RewardItem? _findReward(List<_RewardItem> rewards, String? rewardId) {
  if (rewardId == null || rewardId.isEmpty) return null;
  for (final reward in rewards) {
    if (reward.id == rewardId) return reward;
  }
  return null;
}

Color _parseHexColor(String value) {
  final hex = value.replaceFirst('#', '');
  final parsed = int.tryParse(hex.length == 6 ? 'FF$hex' : hex, radix: 16);
  return Color(parsed ?? 0xFFEAF4F2);
}

class _SyncStatusSummary extends StatelessWidget {
  const _SyncStatusSummary({required this.status});

  final SyncStatus status;

  @override
  Widget build(BuildContext context) {
    final domains = status.domainsPending
        .map((item) => '${item.domain}: ${item.pendingCount}')
        .join(', ');
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('Transport configured: ${status.transportConfigured}'),
        Text('Sync enabled: ${status.syncEnabled}'),
        Text('Account sync state: ${status.accountSyncState}'),
        Text('Pending queue items: ${status.pendingCount}'),
        Text('Domains pending: ${domains.isEmpty ? 'none' : domains}'),
        Text('Last success: ${status.lastSyncSucceededAt ?? 'never'}'),
        Text('Last error: ${status.lastSyncErrorCode ?? 'none'}'),
      ],
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.child, this.trailing});

  final String title;
  final Widget child;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                ?trailing,
              ],
            ),
            const SizedBox(height: 8),
            child,
          ],
        ),
      ),
    );
  }
}

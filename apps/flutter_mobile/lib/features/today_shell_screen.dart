import 'dart:convert';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import '../supabase/announcement_service.dart';
import '../supabase/auth_session_manager.dart';
import '../supabase/supabase_config.dart';
import '../widgets/crocodile_frame_animation.dart';
import 'ai_screen.dart';
import 'auth_screen.dart';
import 'plan_screen.dart';
import 'reports_screen.dart';
import 'study_screen.dart';
import 'wrong_words_screen.dart';

const todayStudyHandoffKey = Key('today-study-handoff');
const todayAiShortcutKey = Key('today-ai-shortcut');

class TodayShellScreen extends StatefulWidget {
  const TodayShellScreen({
    super.key,
    required this.appState,
    this.refreshSeed = 0,
    this.onOpenPlan,
    this.onOpenStudy,
    this.onOpenReports,
    this.onOpenWrongWords,
    this.onOpenAi,
    this.onOpenAccount,
  });

  final AppState appState;
  final int refreshSeed;
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
  final _announcementService = AnnouncementService();
  _TodayHomeBundle? _cachedBundle;
  Object? _loadError;
  int _loadGeneration = 0;
  bool _loading = true;
  bool _aiGenerating = false;
  String? _aiMessage;

  @override
  void initState() {
    super.initState();
    _refreshHomeBundle(showFullLoading: true);
  }

  @override
  void didUpdateWidget(covariant TodayShellScreen oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.refreshSeed != widget.refreshSeed) {
      _refreshHomeBundle(showFullLoading: _cachedBundle == null);
    }
  }

  Future<_TodayHomeBundle> _loadHomeBundle() async {
    final todayFuture = widget.appState.sdk.today.getTodayHomeState();
    final activePlanFuture = _optionalLoad(
      widget.appState.sdk.plan.getActivePlan,
    );
    final aiContextFuture = _optionalLoad(
      widget.appState.sdk.ai.getTodayAiPassageContext,
    );
    final aiHistoryFuture = _optionalLoad(
      widget.appState.sdk.ai.getAiPassageHistory,
      fallback: const <AiPassageHistoryItem>[],
    );
    final rewardStateFuture = _optionalLoad(
      widget.appState.sdk.rewards.getTodayRewardState,
    );
    final announcementsFuture = _optionalLoad(
      _announcementService.fetchVisibleAnnouncements,
      fallback: const <CloudAnnouncement>[],
    );
    final syncStatusFuture = _optionalLoad(() async {
      if (widget.appState.authState.allowsCloudWork) {
        return widget.appState.sdk.sync.flushPendingToCloud();
      }
      return widget.appState.sdk.sync.getSyncStatus();
    });

    final today = await todayFuture;
    final activePlan = _activePlanFromToday(today) ?? await activePlanFuture;

    final aiContext = await aiContextFuture;
    final aiHistory = await aiHistoryFuture ?? const <AiPassageHistoryItem>[];
    final rewardState = await rewardStateFuture;
    final announcements =
        await announcementsFuture ?? const <CloudAnnouncement>[];
    final syncStatus = await syncStatusFuture;

    return _TodayHomeBundle(
      today: today,
      activePlan: activePlan,
      aiContext: aiContext,
      aiHistory: aiHistory,
      rewardState: rewardState,
      announcements: announcements,
      syncStatus: syncStatus,
    );
  }

  Future<void> _dismissAnnouncement(CloudAnnouncement announcement) async {
    await _announcementService.dismiss(announcement.id);
    if (!mounted) return;
    final current = _cachedBundle;
    if (current == null) return;
    setState(() {
      _cachedBundle = current.copyWith(
        announcements: current.announcements
            .where((item) => item.id != announcement.id)
            .toList(growable: false),
      );
    });
  }

  Future<T?> _optionalLoad<T>(
    Future<T> Function() loader, {
    T? fallback,
  }) async {
    try {
      return await loader();
    } catch (_) {
      return fallback;
    }
  }

  Future<void> _refreshHomeBundle({bool showFullLoading = false}) async {
    final generation = ++_loadGeneration;
    if (showFullLoading && mounted) {
      setState(() {
        _loading = true;
        _loadError = null;
      });
    }

    try {
      final bundle = await _loadHomeBundle();
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _cachedBundle = bundle;
        _loadError = null;
        _loading = false;
      });
    } catch (error) {
      if (!mounted || generation != _loadGeneration) return;
      setState(() {
        _loadError = error;
        _loading = false;
      });
    }
  }

  Future<void> _generateAiPassage(TodayAiPassageContext? aiContext) async {
    if (!widget.appState.isSignedIn) {
      setState(() {
        _aiMessage = '请先登录账号，再生成 AI 短文。';
      });
      return;
    }

    if (aiContext == null || _aiGenerating) return;

    final wrongWords = aiContext.generationWrongWords.toList(growable: false);
    final targetWords = aiContext.generationTargetWords.toList(growable: false);

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
        date: aiContext.date,
      );
      if (widget.appState.authState.allowsCloudWork) {
        await widget.appState.sdk.sync.flushPendingToCloud();
      }
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
    final navigator = Navigator.of(context);
    final effectiveHint =
        resumeHint ?? await widget.appState.sdk.study.getResumeSessionHint();
    if (!mounted) return;
    final effectiveMode = mode;
    final hintForScreen =
        effectiveHint.hasResume && effectiveHint.mode == effectiveMode
        ? effectiveHint
        : null;
    if (widget.onOpenStudy != null) {
      await widget.onOpenStudy!.call(effectiveMode, hintForScreen);
      return;
    }
    await navigator.push(
      MaterialPageRoute(
        builder: (_) => StudyScreen(
          sdk: widget.appState.sdk,
          mode: effectiveMode,
          resumeHint: hintForScreen,
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
          isSignedIn: widget.appState.isSignedIn,
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
        builder: (_) => AuthScreen(
          sdk: widget.appState.sdk,
          onAuthChanged: widget.appState.applyAuthState,
        ),
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
    _refreshHomeBundle(showFullLoading: _cachedBundle == null);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('今日'),
        actions: [
          IconButton(
            onPressed: () => widget.appState.retryInitialize(),
            tooltip: '刷新启动状态',
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: _buildBody(context),
    );
  }

  Widget _buildBody(BuildContext context) {
    final cachedBundle = _cachedBundle;
    if (cachedBundle == null) {
      if (_loading) {
        return const CrocodileLoadingAnimation(label: '加载中...');
      }
      if (_loadError != null) {
        final message = _loadError is Exception
            ? _loadError.toString()
            : '${_loadError ?? ''}';
        return _SectionCard(title: 'Today load failed', child: Text(message));
      }
      return const _SectionCard(
        title: 'No today payload',
        child: Text('Bridge returned no Today payload.'),
      );
    }
    final snapshot = AsyncSnapshot<_TodayHomeBundle>.withData(
      ConnectionState.done,
      cachedBundle,
    );
    if (snapshot.connectionState != ConnectionState.done) {
      return const CrocodileLoadingAnimation(label: '加载中...');
    }

    if (snapshot.hasError) {
      final error = snapshot.error;
      final message = error is Exception ? error.toString() : '未知今日页错误';
      return _SectionCard(title: '今日页加载失败', child: Text(message));
    }

    final bundle = snapshot.data;
    if (bundle == null) {
      return const _SectionCard(title: '暂无今日数据', child: Text('学习引擎没有返回今日页数据。'));
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
          if (widget.appState.authState.allowsCloudWork) {
            await widget.appState.sdk.sync.flushPendingToCloud();
          }
          return;
        case 'done':
          await _openAi();
          return;
        default:
          await _openStudy(primaryAction.mode);
          return;
      }
    }

    return CrocodileRefreshIndicator(
      onRefresh: () => _refreshHomeBundle(),
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        children: [
          if (bundle.announcements.isNotEmpty)
            _AnnouncementBanner(
              announcement: bundle.announcements.first,
              onDismiss: () => _dismissAnnouncement(bundle.announcements.first),
            ),
          KeyedSubtree(
            key: todayStudyHandoffKey,
            child: _PrimaryActionCard(
              action: primaryAction,
              completion: completion,
              todayDate: bundle.today.todayDate,
              onStart: handlePrimaryAction,
            ),
          ),
          _TaskBreakdownCard(
            items: taskItems,
            onStartStudy: _openStudy,
            hasSnapshot: hasSnapshot,
            onOpenPlan: _openPlan,
            onSyncToday: _applyPlanToToday,
            hasActivePlan: bundle.activePlan != null,
          ),
          KeyedSubtree(
            key: todayAiShortcutKey,
            child: _AiShortcutCard(
              sdk: widget.appState.sdk,
              context: bundle.aiContext,
              history: bundle.aiHistory,
              isSignedIn: widget.appState.isSignedIn,
              isGenerating: _aiGenerating,
              message: _aiMessage,
              onGenerate: () => _generateAiPassage(bundle.aiContext),
            ),
          ),
          _RewardSlotMachineCard(
            sdk: widget.appState.sdk,
            isSignedIn: widget.appState.isSignedIn,
            tasksComplete: tasksComplete,
            rewardState: bundle.rewardState,
          ),
          _SectionCard(
            title: '开发诊断',
            child: ExpansionTile(
              tilePadding: EdgeInsets.zero,
              childrenPadding: EdgeInsets.zero,
              title: const Text('查看启动、账号与同步详情'),
              children: [
                Text('今日日期：${bundle.today.todayDate}'),
                Text(
                  '应用就绪：${widget.appState.bootstrapState?.appReady ?? false}',
                ),
                Text(
                  '需要首次初始化：'
                  '${widget.appState.bootstrapState?.firstRunRequired ?? false}',
                ),
                Text(
                  '账号：${_accountPhaseLabel(widget.appState.authPhase, widget.appState.authState.userEmail)}',
                ),
                Text('允许本地学习：${widget.appState.authState.allowsLocalStudy}'),
                Text('可用云同步：${widget.appState.authState.allowsCloudWork}'),
                if (widget.appState.authMessage != null)
                  Text('账号提示：${widget.appState.authMessage}'),
                const SizedBox(height: 12),
                if (bundle.syncStatus != null)
                  _SyncStatusSummary(
                    status: bundle.syncStatus!,
                    transportConfigured: SupabaseConfig.isConfigured,
                    syncEnabled: widget.appState.authState.allowsCloudWork,
                    onSyncNow: () async {
                      await widget.appState.sdk.sync.flushPendingToCloud();
                      if (context.mounted) {
                        await _refreshHomeBundle();
                      }
                    },
                  ),
                if (bundle.syncStatus == null) const Text('当前版本无法读取同步状态。'),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

PlanSummary? _activePlanFromToday(TodayHomeState today) {
  final raw = today.activePlan;
  if (raw == null || raw.isEmpty) return null;
  try {
    return PlanSummary.fromJson(raw);
  } catch (_) {
    return null;
  }
}

class _TodayHomeBundle {
  const _TodayHomeBundle({
    required this.today,
    required this.activePlan,
    required this.aiContext,
    required this.aiHistory,
    required this.rewardState,
    required this.announcements,
    required this.syncStatus,
  });

  final TodayHomeState today;
  final PlanSummary? activePlan;
  final TodayAiPassageContext? aiContext;
  final List<AiPassageHistoryItem> aiHistory;
  final TodayRewardState? rewardState;
  final List<CloudAnnouncement> announcements;
  final SyncStatus? syncStatus;

  _TodayHomeBundle copyWith({List<CloudAnnouncement>? announcements}) {
    return _TodayHomeBundle(
      today: today,
      activePlan: activePlan,
      aiContext: aiContext,
      aiHistory: aiHistory,
      rewardState: rewardState,
      announcements: announcements ?? this.announcements,
      syncStatus: syncStatus,
    );
  }
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
    required this.plannedTarget,
    required this.color,
    required this.mode,
  });

  final String label;
  final String helper;
  final int completed;
  final int target;
  final int plannedTarget;
  final Color color;
  final String mode;

  bool get canStart => target > 0 && completed < target;
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

@visibleForTesting
Map<String, dynamic> todaySnapshotOrPlanFallbackForTest(
  Map<String, dynamic>? snapshot,
  PlanSummary? activePlan,
) {
  return _snapshotOrPlanFallback(snapshot, activePlan);
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
        buttonLabel: '同步到今日',
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
          plannedTarget: _planTarget('newWord', activePlan),
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
          plannedTarget: _planTarget('review', activePlan),
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
          plannedTarget: _planTarget('mixedTest', activePlan),
          color: const Color(0xFF8F5A2F),
          mode: 'mixedTest',
        ),
        _TaskItemViewModel(
          label: '错词强化',
          helper: '回收今天的薄弱点',
          target: _displayTarget(
            snapshot,
            'wrongWordReinforcement',
            activePlan,
          ),
          completed: _cappedCompleted(
            snapshot,
            'wrongWordTestCompleted',
            _displayTarget(snapshot, 'wrongWordReinforcement', activePlan),
          ),
          plannedTarget: _planTarget('wrongWordReinforcement', activePlan),
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
          plannedTarget: _planTarget('rootAffix', activePlan),
          color: const Color(0xFF7C52A1),
          mode: 'rootAffix',
        ),
      ]
      .where((item) => item.target > 0 || item.plannedTarget > 0)
      .toList(growable: false);
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
  final hasSnapshotTarget = switch (mode) {
    'newWord' => snapshot.containsKey('newWordsTarget'),
    'review' => snapshot.containsKey('reviewWordsTarget'),
    'mixedTest' => snapshot.containsKey('mixedTestTarget'),
    'wrongWordReinforcement' => snapshot.containsKey('wrongWordTestTarget'),
    'rootAffix' => snapshot.containsKey('rootAffixTarget'),
    _ => false,
  };
  if (hasSnapshotTarget || activePlan == null) {
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

int _planTarget(String mode, PlanSummary? activePlan) {
  if (activePlan == null) return 0;
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
    AuthAccountPhase.emailVerificationPending => 'email verification pending',
    AuthAccountPhase.passwordResetEmailSent => 'password reset email sent',
    AuthAccountPhase.passwordUpdated => 'password updated',
    AuthAccountPhase.signedInNeedsBind => 'signed in, checking data',
    AuthAccountPhase.signedInActive => userEmail ?? 'signed in',
    AuthAccountPhase.signedInExpired => 'signed in expired',
    AuthAccountPhase.signedOutRetainedLocal => 'signed out, local retained',
    AuthAccountPhase.accountDeletedOrRevoked => 'account deleted or revoked',
    AuthAccountPhase.error => 'auth error',
  };
}

class _AnnouncementBanner extends StatelessWidget {
  const _AnnouncementBanner({
    required this.announcement,
    required this.onDismiss,
  });

  final CloudAnnouncement announcement;
  final Future<void> Function() onDismiss;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = _announcementColors(scheme, announcement.level);
    return Card(
      margin: const EdgeInsets.only(bottom: 16),
      color: colors.background,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 14, 8, 14),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(_announcementIcon(announcement.level), color: colors.content),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    announcement.title,
                    style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: colors.content,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Text(
                    announcement.body,
                    style: Theme.of(
                      context,
                    ).textTheme.bodyMedium?.copyWith(color: colors.content),
                  ),
                ],
              ),
            ),
            IconButton(
              onPressed: onDismiss,
              tooltip: 'Dismiss announcement',
              icon: const Icon(Icons.close),
              color: colors.content,
            ),
          ],
        ),
      ),
    );
  }

  _AnnouncementColors _announcementColors(
    ColorScheme scheme,
    AnnouncementLevel level,
  ) {
    return switch (level) {
      AnnouncementLevel.success => _AnnouncementColors(
        background: scheme.tertiaryContainer,
        content: scheme.onTertiaryContainer,
      ),
      AnnouncementLevel.warning => _AnnouncementColors(
        background: const Color(0xFFFFF1C2),
        content: const Color(0xFF4F3900),
      ),
      AnnouncementLevel.critical => _AnnouncementColors(
        background: scheme.errorContainer,
        content: scheme.onErrorContainer,
      ),
      AnnouncementLevel.info => _AnnouncementColors(
        background: scheme.secondaryContainer,
        content: scheme.onSecondaryContainer,
      ),
    };
  }

  IconData _announcementIcon(AnnouncementLevel level) {
    return switch (level) {
      AnnouncementLevel.success => Icons.check_circle_outline,
      AnnouncementLevel.warning => Icons.warning_amber_outlined,
      AnnouncementLevel.critical => Icons.error_outline,
      AnnouncementLevel.info => Icons.campaign_outlined,
    };
  }
}

class _AnnouncementColors {
  const _AnnouncementColors({required this.background, required this.content});

  final Color background;
  final Color content;
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
    final color = Theme.of(context).colorScheme.primary;
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
                        child: Text(hasActivePlan ? '同步到今日' : '前往计划页'),
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
    final done = item.target > 0 && item.completed >= item.target;
    return InkWell(
      onTap: item.canStart ? onTap : null,
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
            done
                ? Icons.check_circle
                : item.canStart
                ? Icons.chevron_right
                : Icons.lock_outline,
            color: item.canStart || done ? item.color : Colors.black38,
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
    required this.isSignedIn,
    required this.isGenerating,
    required this.message,
    required this.onGenerate,
  });

  final WordSdk sdk;
  final TodayAiPassageContext? context;
  final List<AiPassageHistoryItem> history;
  final bool isSignedIn;
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
    final oldId = _todayHistoryItem(
      oldWidget.context,
      oldWidget.history,
    )?.passageId;
    final nextId = _todayHistoryItem(widget.context, widget.history)?.passageId;
    if (oldId != nextId) {
      _passage = null;
      _loadedPassageId = null;
      _loadLatestPassage();
    }
  }

  Future<void> _loadLatestPassage() async {
    final todayItem = _todayHistoryItem(widget.context, widget.history);
    if (todayItem == null) return;
    final passageId = todayItem.passageId;
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
    final canGenerate = widget.isSignedIn && ready && !widget.isGenerating;
    final latest = _todayHistoryItem(widget.context, widget.history);
    final hasPassage = _passage != null;

    return _SectionCard(
      title: 'AI \u77ed\u6587\u603b\u7ed3',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            !widget.isSignedIn
                ? '登录后可以生成 AI 短文。'
                : ready
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
              onPressed: canGenerate ? widget.onGenerate : null,
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

AiPassageHistoryItem? _todayHistoryItem(
  TodayAiPassageContext? context,
  List<AiPassageHistoryItem> history,
) {
  final today = context?.date;
  if (today == null || today.isEmpty) return null;
  for (final item in history) {
    if (item.date == today || item.generatedAt.startsWith(today)) {
      return item;
    }
  }
  return null;
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
        Text('状态：${passage.validationStatus}'),
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
                      if (gloss.isNotEmpty)
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

      return Text('${block['text'] ?? block['content'] ?? block}');
    }

    return Text('$block');
  }
}

class _RewardSlotMachineCard extends StatefulWidget {
  const _RewardSlotMachineCard({
    required this.sdk,
    required this.isSignedIn,
    required this.tasksComplete,
    required this.rewardState,
  });

  final WordSdk sdk;
  final bool isSignedIn;
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
    if (!widget.isSignedIn) {
      setState(() {
        _message = '请先登录账号，再抽取奖励。';
      });
      return;
    }
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
    if (!widget.isSignedIn) {
      setState(() {
        _message = '请先登录账号，再保存奖励。';
      });
      return;
    }

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
        final ready = widget.isSignedIn && widget.tasksComplete && !hasClaim;
        final locked = !widget.isSignedIn || !widget.tasksComplete;
        final canSave =
            ready && !_spinning && !_saving && _previewReward != null;
        final title = !widget.isSignedIn
            ? '登录后可抽取今日奖励'
            : hasClaim
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
    final knobColor = active ? const Color(0xFFE0565B) : Colors.black26;
    final cabinetColor = active
        ? Theme.of(context).colorScheme.primary.withValues(alpha: 0.14)
        : Colors.black12;
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
  const _SyncStatusSummary({
    required this.status,
    required this.transportConfigured,
    required this.syncEnabled,
    required this.onSyncNow,
  });

  final SyncStatus status;
  final bool transportConfigured;
  final bool syncEnabled;
  final Future<void> Function() onSyncNow;

  String _formatCloudRestore(Map<String, dynamic> restore) {
    final succeeded = restore['succeeded'] == true;
    final studyPointRows = restore['studyPointRows'] ?? 0;
    final restoredStudyPoints = restore['restoredStudyPoints'] ?? 0;
    final reportRows = restore['reportSnapshotRows'] ?? 0;
    final restoredReports = restore['restoredReportSnapshots'] ?? 0;
    final wrongWordRows = restore['wrongWordRows'] ?? 0;
    final restoredWrongWords = restore['restoredWordHints'] ?? 0;
    final aiPassageRows = restore['aiPassageRows'] ?? 0;
    final restoredAiPassages = restore['restoredAiPassages'] ?? 0;
    final error = restore['error'];
    if (!succeeded) {
      return '失败 ${error ?? ''}'.trim();
    }
    return '成功 学习 $studyPointRows/$restoredStudyPoints · 错词 $wrongWordRows/$restoredWrongWords · 报告 $reportRows/$restoredReports · AI $aiPassageRows/$restoredAiPassages';
  }

  String _formatLocalRestoreDiagnostics(Map<String, dynamic> diagnostics) {
    final studyResults = diagnostics['studyResultsCount'] ?? 0;
    final restoredResults = diagnostics['cloudRestoreStudyResultsCount'] ?? 0;
    final wrongWords = diagnostics['wrongWordVisibleCount'] ?? 0;
    final reports = diagnostics['reportsHistoryCount'] ?? 0;
    final aiPassages = diagnostics['aiPassageCount'] ?? 0;
    final highlightedAi = diagnostics['aiPassagesWithWordSegments'] ?? 0;
    return '本机可读 学习结果 $studyResults（云恢复 $restoredResults） · 错词 $wrongWords · 报告 $reports · AI $aiPassages（高亮 $highlightedAi）';
  }

  @override
  Widget build(BuildContext context) {
    final domains = status.domainsPending
        .map((item) => '${item.domain}: ${item.pendingCount}')
        .join(', ');
    final canSyncNow =
        transportConfigured && syncEnabled && status.pendingCount > 0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('传输已配置：$transportConfigured'),
        Text('同步已启用：$syncEnabled'),
        Text('账号同步状态：${status.accountSyncState}'),
        Text('待同步队列：${status.pendingCount}'),
        Text('待同步领域：${domains.isEmpty ? '无' : domains}'),
        Text('上次上传成功：${status.lastSyncSucceededAt ?? '从未'}'),
        Text('上次上传错误：${status.lastSyncErrorCode ?? '无'}'),
        if (status.lastCloudRestore != null)
          Text('云端恢复：${_formatCloudRestore(status.lastCloudRestore!)}'),
        if (status.localCloudRestoreDiagnostics != null)
          Text(
            '恢复诊断：${_formatLocalRestoreDiagnostics(status.localCloudRestoreDiagnostics!)}',
          ),
        const SizedBox(height: 8),
        FilledButton(
          onPressed: canSyncNow ? onSyncNow : null,
          child: const Text('立即同步'),
        ),
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

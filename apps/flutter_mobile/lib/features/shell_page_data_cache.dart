import 'package:flutter/foundation.dart';

import '../sdk/sdk.dart';

enum ShellPageDataScope { today, plan, wrongWords, reports, ai }

class ShellPageDataCache {
  ShellPageDataCache({required WordSdk sdk})
    : _loadToday = sdk.today.getTodayHomeState,
      _loadActivePlan = sdk.plan.getActivePlan,
      _loadWordbooks = sdk.plan.getWordbooks,
      _loadTodayAiContext = sdk.ai.getTodayAiPassageContext,
      _loadAiHistory = sdk.ai.getAiPassageHistory;

  @visibleForTesting
  ShellPageDataCache.forTesting({
    required Future<TodayHomeState> Function() loadToday,
    required Future<PlanSummary?> Function() loadActivePlan,
    required Future<List<WordbookSummary>> Function() loadWordbooks,
    required Future<TodayAiPassageContext?> Function() loadTodayAiContext,
    required Future<List<AiPassageHistoryItem>> Function() loadAiHistory,
  }) : _loadToday = loadToday,
       _loadActivePlan = loadActivePlan,
       _loadWordbooks = loadWordbooks,
       _loadTodayAiContext = loadTodayAiContext,
       _loadAiHistory = loadAiHistory;

  final Future<TodayHomeState> Function() _loadToday;
  final Future<PlanSummary?> Function() _loadActivePlan;
  final Future<List<WordbookSummary>> Function() _loadWordbooks;
  final Future<TodayAiPassageContext?> Function() _loadTodayAiContext;
  final Future<List<AiPassageHistoryItem>> Function() _loadAiHistory;

  Future<TodayHomeState>? _todayHomeState;
  Future<PlanSummary?>? _activePlan;
  Future<List<WordbookSummary>>? _wordbooks;
  Future<TodayAiPassageContext?>? _todayAiContext;
  Future<List<AiPassageHistoryItem>>? _aiHistory;

  Future<TodayHomeState> loadToday({bool refresh = false}) {
    if (refresh) {
      _todayHomeState = null;
    }
    return _todayHomeState ??= _loadToday();
  }

  Future<PlanSummary?> loadActivePlan({bool refresh = false}) {
    if (refresh) {
      _activePlan = null;
    }
    return _activePlan ??= _loadActivePlan();
  }

  Future<List<WordbookSummary>> loadWordbooks({bool refresh = false}) {
    if (refresh) {
      _wordbooks = null;
    }
    return _wordbooks ??= _loadWordbooks();
  }

  Future<TodayAiPassageContext?> loadTodayAiContext({bool refresh = false}) {
    if (refresh) {
      _todayAiContext = null;
    }
    return _todayAiContext ??= _loadTodayAiContext();
  }

  Future<List<AiPassageHistoryItem>> loadAiHistory({bool refresh = false}) {
    if (refresh) {
      _aiHistory = null;
    }
    return _aiHistory ??= _loadAiHistory();
  }

  void preload(ShellPageDataScope scope) {
    switch (scope) {
      case ShellPageDataScope.today:
        loadToday().ignore();
      case ShellPageDataScope.plan:
        loadActivePlan().ignore();
        loadWordbooks().ignore();
      case ShellPageDataScope.ai:
        loadTodayAiContext().ignore();
        loadAiHistory().ignore();
      case ShellPageDataScope.wrongWords:
      case ShellPageDataScope.reports:
        break;
    }
  }

  void invalidate(ShellPageDataScope scope) {
    switch (scope) {
      case ShellPageDataScope.today:
        _todayHomeState = null;
      case ShellPageDataScope.plan:
        _activePlan = null;
        _wordbooks = null;
      case ShellPageDataScope.ai:
        _todayAiContext = null;
        _aiHistory = null;
      case ShellPageDataScope.wrongWords:
      case ShellPageDataScope.reports:
        break;
    }
  }

  void invalidateAll() {
    for (final scope in ShellPageDataScope.values) {
      invalidate(scope);
    }
  }
}

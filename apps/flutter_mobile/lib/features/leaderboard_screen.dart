import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../supabase/leaderboard_service.dart';
import '../supabase/supabase_config.dart';
import '../widgets/crocodile_frame_animation.dart';

class LeaderboardScreen extends StatefulWidget {
  LeaderboardScreen({
    super.key,
    required this.sdk,
    LeaderboardService? service,
  }) : service = service ?? LeaderboardService();

  final WordSdk sdk;
  final LeaderboardService service;

  @override
  State<LeaderboardScreen> createState() => _LeaderboardScreenState();
}

class _LeaderboardScreenState extends State<LeaderboardScreen> {
  LeaderboardMetric _metric = LeaderboardMetric.totalQuestions;
  LeaderboardPeriod _period = LeaderboardPeriod.weekly;
  bool _loading = true;
  bool _refreshingSummary = false;
  String? _message;
  List<LeaderboardEntry> _entries = const [];

  @override
  void initState() {
    super.initState();
    _load(refreshSummary: true);
  }

  Future<void> _load({
    bool refreshSummary = false,
    bool showFullLoading = true,
  }) async {
    setState(() {
      if (showFullLoading) _loading = true;
      _message = null;
    });
    try {
      if (!SupabaseConfig.isConfigured) {
        setState(() {
          _entries = const [];
          _message = 'Supabase 还没有配置，暂时无法加载排行榜。';
        });
        return;
      }
      if (refreshSummary) {
        await _refreshMySummary();
      }
      final entries = await widget.service.fetchLeaderboard(
        metric: _metric,
        period: _period,
      );
      setState(() {
        _entries = entries;
        _message = entries.isEmpty ? '当前榜单还没有数据。' : null;
      });
    } catch (error) {
      setState(() {
        _entries = const [];
        _message = _leaderboardErrorMessage(error);
      });
    } finally {
      if (mounted) {
        setState(() {
          _loading = false;
        });
      }
    }
  }

  Future<void> _refreshMySummary() async {
    setState(() {
      _refreshingSummary = true;
    });
    try {
      final signedIn = await widget.service.isSignedIn;
      if (!signedIn) {
        setState(() {
          _message = '登录后会发布你的学习数据。';
        });
        return;
      }
      final reports = await widget.sdk.reports.getReportsOverview();
      await widget.service.refreshFromReports(
        reports,
        metric: _metric,
        period: _period,
      );
    } finally {
      if (mounted) {
        setState(() {
          _refreshingSummary = false;
        });
      }
    }
  }

  String _leaderboardErrorMessage(Object error) {
    final raw = error.toString();
    if (raw.contains('PGRST202') ||
        raw.contains('refresh_leaderboard_summary') ||
        raw.contains('get_leaderboard')) {
      return '排行榜云端结构还不是最新版。请先在 Supabase 执行最新排行榜 SQL，并刷新 schema cache。';
    }
    return '排行榜加载失败：$raw';
  }

  @override
  Widget build(BuildContext context) {
    final showPeriodSelector = _metric.usesPeriod;
    return Scaffold(
      appBar: AppBar(
        title: const Text('排行榜'),
        actions: [
          IconButton(
            tooltip: '刷新',
            onPressed: _loading ? null : () => _load(refreshSummary: true),
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Column(
        children: [
          _MetricSelector(
            selected: _metric,
            onSelected: (metric) {
              setState(() {
                _metric = metric;
              });
              _load(refreshSummary: true);
            },
          ),
          if (showPeriodSelector)
            _PeriodSelector(
              selected: _period,
              onSelected: (period) {
                setState(() {
                  _period = period;
                });
                _load(refreshSummary: true);
              },
            ),
          _LeaderboardScopeLabel(metric: _metric, period: _period),
          if (_refreshingSummary) const LinearProgressIndicator(),
          Expanded(
            child: _loading
                ? const CrocodileLoadingAnimation(label: '加载中...')
                : _entries.isEmpty
                    ? _EmptyLeaderboard(message: _message)
                    : CrocodileRefreshIndicator(
                        onRefresh: () => _load(
                          refreshSummary: true,
                          showFullLoading: false,
                        ),
                        child: ListView.separated(
                          padding: const EdgeInsets.all(16),
                          itemBuilder: (context, index) {
                            final entry = _entries[index];
                            return _LeaderboardTile(
                              entry: entry,
                              metric: _metric,
                            );
                          },
                          separatorBuilder: (context, index) =>
                              const SizedBox(height: 8),
                          itemCount: _entries.length,
                        ),
                      ),
          ),
        ],
      ),
    );
  }
}

class _LeaderboardTile extends StatelessWidget {
  const _LeaderboardTile({required this.entry, required this.metric});

  final LeaderboardEntry entry;
  final LeaderboardMetric metric;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final highlight = entry.isCurrentUser;
    return Card(
      color: highlight ? colorScheme.primaryContainer : null,
      child: ListTile(
        leading: CircleAvatar(
          backgroundColor: highlight
              ? colorScheme.primary
              : colorScheme.surfaceContainerHighest,
          foregroundColor:
              highlight ? colorScheme.onPrimary : colorScheme.onSurfaceVariant,
          child: Text('${entry.rank}'),
        ),
        title: Text(entry.displayName),
        subtitle: Text(
          _summaryText(entry),
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        trailing: Text(
          _metricValue(entry, metric),
          style: Theme.of(context).textTheme.titleMedium,
        ),
      ),
    );
  }

  String _summaryText(LeaderboardEntry entry) {
    return '题数 ${entry.totalQuestions} | 正确率 ${entry.accuracyPercent.toStringAsFixed(1)}% | 混测 ${entry.mixedTestAccuracyPercent.toStringAsFixed(1)}% | 连续 ${entry.currentStreakDays}天';
  }

  String _metricValue(LeaderboardEntry entry, LeaderboardMetric metric) {
    return switch (metric) {
      LeaderboardMetric.totalQuestions => '${entry.totalQuestions}',
      LeaderboardMetric.accuracy =>
        '${entry.accuracyPercent.toStringAsFixed(1)}%',
      LeaderboardMetric.mixedAccuracy =>
        '${entry.mixedTestAccuracyPercent.toStringAsFixed(1)}%',
      LeaderboardMetric.currentStreak => '${entry.currentStreakDays}天',
    };
  }
}

class _MetricSelector extends StatelessWidget {
  const _MetricSelector({required this.selected, required this.onSelected});

  final LeaderboardMetric selected;
  final ValueChanged<LeaderboardMetric> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 56,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 10, 16, 8),
        itemBuilder: (context, index) {
          final metric = LeaderboardMetric.values[index];
          return ChoiceChip(
            label: Text(
              metric.label,
              style: const TextStyle(fontSize: 13),
            ),
            selected: selected == metric,
            onSelected: (_) => onSelected(metric),
            avatar:
                selected == metric ? const Icon(Icons.check, size: 16) : null,
            visualDensity: VisualDensity.compact,
          );
        },
        separatorBuilder: (context, index) => const SizedBox(width: 8),
        itemCount: LeaderboardMetric.values.length,
      ),
    );
  }
}

class _PeriodSelector extends StatelessWidget {
  const _PeriodSelector({required this.selected, required this.onSelected});

  final LeaderboardPeriod selected;
  final ValueChanged<LeaderboardPeriod> onSelected;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 50,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 6, 16, 8),
        itemBuilder: (context, index) {
          final period = LeaderboardPeriod.values[index];
          return ChoiceChip(
            label: Text(
              period.label,
              style: const TextStyle(fontSize: 13),
            ),
            selected: selected == period,
            onSelected: (_) => onSelected(period),
            visualDensity: VisualDensity.compact,
          );
        },
        separatorBuilder: (context, index) => const SizedBox(width: 8),
        itemCount: LeaderboardPeriod.values.length,
      ),
    );
  }
}

class _LeaderboardScopeLabel extends StatelessWidget {
  const _LeaderboardScopeLabel({required this.metric, required this.period});

  final LeaderboardMetric metric;
  final LeaderboardPeriod period;

  @override
  Widget build(BuildContext context) {
    final text = metric.usesPeriod
        ? '${period.label} · ${_periodRangeLabel(period)}'
        : '连续榜 · 每日更新';
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 0, 20, 4),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          text,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Colors.black54,
                fontWeight: FontWeight.w600,
              ),
        ),
      ),
    );
  }

  String _periodRangeLabel(LeaderboardPeriod period) {
    final now = DateTime.now();
    return switch (period) {
      LeaderboardPeriod.weekly => '${now.year}年第${_weekOfYear(now)}周',
      LeaderboardPeriod.monthly => '${now.year}年第${now.month}月',
      LeaderboardPeriod.allTime => '每日更新',
    };
  }

  int _weekOfYear(DateTime date) {
    final yearStart = DateTime(date.year, 1, 1);
    final days = date.difference(yearStart).inDays;
    return (days / 7).floor() + 1;
  }
}

class _EmptyLeaderboard extends StatelessWidget {
  const _EmptyLeaderboard({required this.message});

  final String? message;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          message ?? '当前榜单还没有数据。',
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodyLarge,
        ),
      ),
    );
  }
}

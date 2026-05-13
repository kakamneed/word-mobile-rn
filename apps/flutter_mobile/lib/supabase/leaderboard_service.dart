library;

import '../sdk/reports_client.dart';
import 'supabase_auth_service.dart';
import 'supabase_config.dart';

enum LeaderboardPeriod {
  weekly('weekly', '周榜'),
  monthly('monthly', '月榜'),
  allTime('all_time', '总榜');

  const LeaderboardPeriod(this.wireName, this.label);

  final String wireName;
  final String label;
}

enum LeaderboardMetric {
  totalQuestions('totalQuestions', '题数榜'),
  accuracy('accuracy', '正确率榜'),
  mixedAccuracy('mixedAccuracy', '混合测试正确率榜'),
  currentStreak('currentStreak', '连续榜');

  const LeaderboardMetric(this.wireName, this.label);

  final String wireName;
  final String label;

  bool get usesPeriod => this != LeaderboardMetric.currentStreak;
}

class LeaderboardEntry {
  const LeaderboardEntry({
    required this.rank,
    required this.isCurrentUser,
    required this.userId,
    required this.displayName,
    required this.totalQuestions,
    required this.correctCount,
    required this.accuracyPercent,
    required this.mixedTestTotalQuestions,
    required this.mixedTestCorrectCount,
    required this.mixedTestAccuracyPercent,
    required this.currentStreakDays,
    required this.updatedAt,
  });

  final int rank;
  final bool isCurrentUser;
  final String userId;
  final String displayName;
  final int totalQuestions;
  final int correctCount;
  final double accuracyPercent;
  final int mixedTestTotalQuestions;
  final int mixedTestCorrectCount;
  final double mixedTestAccuracyPercent;
  final int currentStreakDays;
  final DateTime? updatedAt;

  factory LeaderboardEntry.fromJson(Map<String, dynamic> json) {
    return LeaderboardEntry(
      rank: (json['rank'] as num?)?.toInt() ?? 0,
      isCurrentUser: json['is_current_user'] as bool? ?? false,
      userId: json['user_id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? 'Word learner',
      totalQuestions: (json['total_questions'] as num?)?.toInt() ?? 0,
      correctCount: (json['correct_count'] as num?)?.toInt() ?? 0,
      accuracyPercent: (json['accuracy_percent'] as num?)?.toDouble() ?? 0,
      mixedTestTotalQuestions:
          (json['mixed_test_total_questions'] as num?)?.toInt() ?? 0,
      mixedTestCorrectCount:
          (json['mixed_test_correct_count'] as num?)?.toInt() ?? 0,
      mixedTestAccuracyPercent:
          (json['mixed_test_accuracy_percent'] as num?)?.toDouble() ?? 0,
      currentStreakDays: (json['current_streak_days'] as num?)?.toInt() ?? 0,
      updatedAt: DateTime.tryParse('${json['updated_at'] ?? ''}'),
    );
  }
}

class LeaderboardService {
  LeaderboardService({SupabaseAuthService? authService})
      : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<bool> get isSignedIn async {
    if (!isConfigured) return false;
    await _authService.ensureInitialized();
    return _authService.client.auth.currentSession != null;
  }

  Future<void> refreshFromReports(
    ReportsOverview reports, {
    required LeaderboardMetric metric,
    LeaderboardPeriod period = LeaderboardPeriod.weekly,
  }) async {
    if (!isConfigured) return;
    await _authService.ensureInitialized();
    final client = _authService.client;
    final session = client.auth.currentSession;
    if (session == null) return;

    final uploadPeriod =
        metric.usesPeriod ? period : LeaderboardPeriod.allTime;
    final periodStart = _periodStart(DateTime.now(), uploadPeriod);
    final summary = _summaryForPeriod(reports, uploadPeriod, periodStart);
    final email = session.user.email ?? '';
    final displayName = await _loadCurrentProfileDisplayName(session.user.id);

    await client.rpc(
      'refresh_leaderboard_summary',
      params: {
        'p_display_name': displayName.isNotEmpty ? displayName : email,
        'p_total_questions': summary.totalQuestions,
        'p_correct_count': summary.correctCount,
        'p_mixed_test_total_questions': summary.mixedTestTotalQuestions,
        'p_mixed_test_correct_count': summary.mixedTestCorrectCount,
        'p_current_streak_days':
            (reports.streakInfo['currentStreak'] as num?)?.toInt() ?? 0,
        'p_summary_key': 'reports-${uploadPeriod.wireName}-$periodStart',
        'p_summary_at': DateTime.now().toUtc().toIso8601String(),
        'p_period': uploadPeriod.wireName,
        'p_period_start': periodStart,
      },
    );
  }

  Future<String> _loadCurrentProfileDisplayName(String userId) async {
    try {
      final rows = await _authService.client
          .from('profiles')
          .select('display_name')
          .eq('user_id', userId)
          .limit(1);
      if (rows.isNotEmpty) {
        final row = rows.first;
        return '${row['display_name'] ?? ''}'.trim();
      }
    } catch (_) {
      // Fall back to auth metadata/email when profile lookup is unavailable.
    }
    final metadata =
        _authService.client.auth.currentSession?.user.userMetadata;
    return '${metadata?['display_name'] ?? ''}'.trim();
  }

  Future<List<LeaderboardEntry>> fetchLeaderboard({
    required LeaderboardMetric metric,
    LeaderboardPeriod period = LeaderboardPeriod.weekly,
    int limit = 50,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase is not configured.');
    }
    await _authService.ensureInitialized();
    final queryPeriod = metric.usesPeriod ? period : LeaderboardPeriod.allTime;
    final response = await _authService.client.rpc(
      'get_leaderboard',
      params: {
        'p_metric': metric.wireName,
        'p_limit': limit,
        'p_period': queryPeriod.wireName,
        'p_period_start': _periodStart(DateTime.now(), queryPeriod),
      },
    );
    final rows = response as List<dynamic>? ?? const [];
    return rows
        .whereType<Map>()
        .map((row) => LeaderboardEntry.fromJson(row.cast<String, dynamic>()))
        .toList(growable: false);
  }

  _LeaderboardSummary _summaryForPeriod(
    ReportsOverview reports,
    LeaderboardPeriod period,
    String periodStart,
  ) {
    if (period == LeaderboardPeriod.allTime) {
      final mixed = _modeTotal(reports, 'mixedTest');
      return _LeaderboardSummary(
        totalQuestions: reports.totalQuestionsAnswered,
        correctCount: _correctCountFromReports(reports),
        mixedTestTotalQuestions: mixed.totalQuestions,
        mixedTestCorrectCount: mixed.correctCount,
      );
    }

    var totalQuestions = 0;
    var correctCount = 0;
    for (final row in reports.dailySeries.whereType<Map>()) {
      final date = '${row['date'] ?? ''}';
      if (!_dateBelongsToPeriod(date, period, periodStart)) continue;
      totalQuestions += (row['totalQuestions'] as num?)?.toInt() ?? 0;
      correctCount += (row['correctCount'] as num?)?.toInt() ?? 0;
    }
    final mixed = _modePeriodTotal(reports, 'mixedTest', period, periodStart);

    return _LeaderboardSummary(
      totalQuestions: totalQuestions,
      correctCount: correctCount.clamp(0, totalQuestions),
      mixedTestTotalQuestions: mixed.totalQuestions,
      mixedTestCorrectCount: mixed.correctCount,
    );
  }

  _QuestionCounts _modeTotal(ReportsOverview reports, String mode) {
    for (final row in reports.modeBreakdown.whereType<Map>()) {
      if ('${row['mode'] ?? ''}' != mode) continue;
      return _QuestionCounts(
        totalQuestions: (row['totalQuestions'] as num?)?.toInt() ?? 0,
        correctCount: (row['correctCount'] as num?)?.toInt() ?? 0,
      );
    }
    return const _QuestionCounts(totalQuestions: 0, correctCount: 0);
  }

  _QuestionCounts _modePeriodTotal(
    ReportsOverview reports,
    String mode,
    LeaderboardPeriod period,
    String periodStart,
  ) {
    final rows = reports.modeSeries[mode];
    if (rows is! List) {
      return const _QuestionCounts(totalQuestions: 0, correctCount: 0);
    }
    var totalQuestions = 0;
    var correctCount = 0;
    for (final row in rows.whereType<Map>()) {
      if (!_dateBelongsToPeriod('${row['date'] ?? ''}', period, periodStart)) {
        continue;
      }
      totalQuestions += (row['totalQuestions'] as num?)?.toInt() ?? 0;
      correctCount += (row['correctCount'] as num?)?.toInt() ?? 0;
    }
    return _QuestionCounts(
      totalQuestions: totalQuestions,
      correctCount: correctCount.clamp(0, totalQuestions),
    );
  }

  int _correctCountFromReports(ReportsOverview reports) {
    final total = reports.totalQuestionsAnswered;
    if (total <= 0) return 0;
    return ((reports.overallAccuracy / 100.0) * total).round().clamp(0, total);
  }

  bool _dateBelongsToPeriod(
    String date,
    LeaderboardPeriod period,
    String periodStart,
  ) {
    if (period == LeaderboardPeriod.allTime) return true;
    final parsed = DateTime.tryParse(date);
    final start = DateTime.tryParse(periodStart);
    if (parsed == null || start == null) return false;
    final end = switch (period) {
      LeaderboardPeriod.weekly => start.add(const Duration(days: 7)),
      LeaderboardPeriod.monthly => DateTime(start.year, start.month + 1),
      LeaderboardPeriod.allTime => DateTime(9999),
    };
    return !parsed.isBefore(start) && parsed.isBefore(end);
  }

  String _periodStart(DateTime now, LeaderboardPeriod period) {
    final local = DateTime(now.year, now.month, now.day);
    final start = switch (period) {
      LeaderboardPeriod.weekly =>
        local.subtract(Duration(days: local.weekday - DateTime.monday)),
      LeaderboardPeriod.monthly => DateTime(local.year, local.month),
      LeaderboardPeriod.allTime => DateTime(1970),
    };
    return start.toIso8601String().split('T').first;
  }
}

class _LeaderboardSummary {
  const _LeaderboardSummary({
    required this.totalQuestions,
    required this.correctCount,
    required this.mixedTestTotalQuestions,
    required this.mixedTestCorrectCount,
  });

  final int totalQuestions;
  final int correctCount;
  final int mixedTestTotalQuestions;
  final int mixedTestCorrectCount;
}

class _QuestionCounts {
  const _QuestionCounts({
    required this.totalQuestions,
    required this.correctCount,
  });

  final int totalQuestions;
  final int correctCount;
}

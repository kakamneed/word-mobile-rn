library;

import '../bridge/bridge.dart';

class ReportsOverview {
  final int totalStudyDays;
  final int totalWordsLearned;
  final int totalQuestionsAnswered;
  final double overallAccuracy;
  final Map<String, dynamic> streakInfo;
  final List<dynamic> modeBreakdown;
  final List<dynamic> last7Days;
  final List<dynamic> dailySeries;
  final Map<String, dynamic> modeSeries;

  const ReportsOverview({
    required this.totalStudyDays,
    required this.totalWordsLearned,
    required this.totalQuestionsAnswered,
    required this.overallAccuracy,
    required this.streakInfo,
    required this.modeBreakdown,
    required this.last7Days,
    required this.dailySeries,
    required this.modeSeries,
  });

  factory ReportsOverview.fromJson(Map<String, dynamic> json) => ReportsOverview(
        totalStudyDays: json['totalStudyDays'] as int,
        totalWordsLearned: json['totalWordsLearned'] as int,
        totalQuestionsAnswered: json['totalQuestionsAnswered'] as int,
        overallAccuracy: (json['overallAccuracy'] as num).toDouble(),
        streakInfo: json['streakInfo'] as Map<String, dynamic>? ?? const {},
        modeBreakdown: json['modeBreakdown'] as List<dynamic>? ?? const [],
        last7Days: json['last7Days'] as List<dynamic>? ?? const [],
        dailySeries: json['dailySeries'] as List<dynamic>? ?? const [],
        modeSeries: json['modeSeries'] as Map<String, dynamic>? ?? const {},
      );
}

class ReportsClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const ReportsClient(this._bridge, this._codec);

  Future<ReportsOverview> getReportsOverview() async {
    final raw = await _bridge.call('getReportsOverview');
    final json = _codec.decodeResponse(raw);
    return ReportsOverview.fromJson(json);
  }
}

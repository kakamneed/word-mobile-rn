/// Today client - typed SDK for the today home API.
library;

import '../bridge/bridge.dart';

/// Today home state returned by the Rust core.
class TodayHomeState {
  final String todayDate;
  final Map<String, dynamic>? activePlan;
  final Map<String, dynamic>? todaySnapshot;
  final List<dynamic> wordbooks;
  final Map<String, dynamic> dailyProgress;

  const TodayHomeState({
    required this.todayDate,
    this.activePlan,
    this.todaySnapshot,
    this.wordbooks = const [],
    required this.dailyProgress,
  });

  factory TodayHomeState.fromJson(Map<String, dynamic> json) => TodayHomeState(
        todayDate: json['todayDate'] as String,
        activePlan: json['activePlan'] as Map<String, dynamic>?,
        todaySnapshot: json['todaySnapshot'] as Map<String, dynamic>?,
        wordbooks: json['wordbooks'] as List<dynamic>? ?? const [],
        dailyProgress: json['dailyProgress'] as Map<String, dynamic>,
      );
}

/// Client for the today home API.
class TodayClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const TodayClient(this._bridge, this._codec);

  /// Get the today home state from the Rust core.
  Future<TodayHomeState> getTodayHomeState() async {
    final raw = await _bridge.call('getTodayHomeState');
    final json = _codec.decodeResponse(raw);
    return TodayHomeState.fromJson(json);
  }
}

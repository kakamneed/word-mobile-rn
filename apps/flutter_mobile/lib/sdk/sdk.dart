/// Flutter SDK layer - typed API surface over the Rust bridge.
///
/// Pages and features must use these clients, never the bridge directly.
library;

import '../bridge/bridge.dart';
import 'ai_client.dart';
import 'bootstrap_client.dart';
import 'plan_client.dart';
import 'reports_client.dart';
import 'reward_client.dart';
import 'study_client.dart';
import 'sync_client.dart';
import 'today_client.dart';
import 'wrong_words_client.dart';

export 'ai_client.dart';
export 'bootstrap_client.dart';
export 'plan_client.dart';
export 'reports_client.dart';
export 'reward_client.dart';
export 'study_client.dart';
export 'sync_client.dart';
export 'today_client.dart';
export 'wrong_words_client.dart';

/// The central SDK that provides typed clients for all Rust contract families.
class WordSdk {
  final AiClient ai;
  final BootstrapClient bootstrap;
  final TodayClient today;
  final PlanClient plan;
  final ReportsClient reports;
  final RewardClient rewards;
  final StudyClient study;
  final SyncClient sync;
  final WrongWordsClient wrongWords;

  const WordSdk._({
    required this.ai,
    required this.bootstrap,
    required this.today,
    required this.plan,
    required this.reports,
    required this.rewards,
    required this.study,
    required this.sync,
    required this.wrongWords,
  });

  /// Create the SDK with the default bridge and codec.
  factory WordSdk() {
    const bridge = RustBridge();
    const codec = BridgeCodec();
    return WordSdk._(
      ai: AiClient(bridge, codec),
      bootstrap: BootstrapClient(bridge, codec),
      today: TodayClient(bridge, codec),
      plan: PlanClient(bridge, codec),
      reports: ReportsClient(bridge, codec),
      rewards: RewardClient(bridge, codec),
      study: StudyClient(bridge, codec),
      sync: SyncClient(bridge, codec),
      wrongWords: WrongWordsClient(bridge, codec),
    );
  }
}

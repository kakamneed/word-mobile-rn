/// Flutter SDK layer - typed API surface over the Rust bridge.
///
/// Pages and features must use these clients, never the bridge directly.
library;

import 'package:flutter/foundation.dart';

import '../bridge/bridge.dart';
import 'ai_client.dart';
import 'bootstrap_client.dart';
import 'croc_bti_client.dart';
import 'exam_practice_client.dart';
import 'local_data_owner_client.dart';
import 'plan_client.dart';
import 'reports_client.dart';
import 'reward_client.dart';
import 'reward_image_client.dart';
import 'study_client.dart';
import 'sync_client.dart';
import 'today_client.dart';
import 'wrong_words_client.dart';

export 'ai_client.dart';
export 'bootstrap_client.dart';
export 'croc_bti_client.dart';
export 'exam_practice_client.dart';
export 'local_data_owner_client.dart';
export 'plan_client.dart';
export 'reports_client.dart';
export 'reward_client.dart';
export 'reward_image_client.dart';
export 'study_client.dart';
export 'sync_client.dart';
export 'today_client.dart';
export 'wrong_words_client.dart';

/// The central SDK that provides typed clients for all Rust contract families.
class WordSdk {
  final AiClient ai;
  final BootstrapClient bootstrap;
  final CrocBtiClient crocBti;
  final ExamPracticeClient examPractice;
  final LocalDataOwnerClient localDataOwner;
  final TodayClient today;
  final PlanClient plan;
  final ReportsClient reports;
  final RewardClient rewards;
  final RewardImageClient rewardImages;
  final StudyClient study;
  final SyncClient sync;
  final WrongWordsClient wrongWords;

  const WordSdk._({
    required this.ai,
    required this.bootstrap,
    required this.crocBti,
    required this.examPractice,
    required this.localDataOwner,
    required this.today,
    required this.plan,
    required this.reports,
    required this.rewards,
    required this.rewardImages,
    required this.study,
    required this.sync,
    required this.wrongWords,
  });

  /// Create the SDK with the default bridge and codec.
  factory WordSdk() {
    const bridge = RustBridge();
    const codec = BridgeCodec();
    return WordSdk.bridgeForTesting(bridge: bridge, codec: codec);
  }

  @visibleForTesting
  factory WordSdk.bridgeForTesting({
    required RustBridge bridge,
    BridgeCodec codec = const BridgeCodec(),
  }) {
    return WordSdk._(
      ai: AiClient(bridge, codec),
      bootstrap: BootstrapClient(bridge, codec),
      crocBti: CrocBtiClient(bridge, codec),
      examPractice: ExamPracticeClient(bridge, codec),
      localDataOwner: LocalDataOwnerClient(bridge, codec),
      today: TodayClient(bridge, codec),
      plan: PlanClient(bridge, codec),
      reports: ReportsClient(bridge, codec),
      rewards: RewardClient(bridge, codec),
      rewardImages: RewardImageClient(bridge, codec),
      study: StudyClient(bridge, codec),
      sync: SyncClient(bridge, codec),
      wrongWords: WrongWordsClient(bridge, codec),
    );
  }
}

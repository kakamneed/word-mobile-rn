/// Plan client - typed SDK for plan and wordbook APIs.
library;

import '../bridge/bridge.dart';

class PlanSummary {
  final int id;
  final String name;
  final int newWordsPerDay;
  final int reviewWordsPerDay;
  final int mixedTestPerDay;
  final int wrongWordTestPerDay;
  final int? rootAffixPerDay;
  final int growthIntervalDays;
  final int growthIncrement;
  final String growthRuleMode;
  final Map<String, dynamic>? sharedGrowthRule;
  final Map<String, dynamic>? growthRulesByMode;
  final Map<String, dynamic>? questionTypeWeightsByMode;

  const PlanSummary({
    required this.id,
    required this.name,
    required this.newWordsPerDay,
    required this.reviewWordsPerDay,
    required this.mixedTestPerDay,
    required this.wrongWordTestPerDay,
    this.rootAffixPerDay,
    required this.growthIntervalDays,
    required this.growthIncrement,
    required this.growthRuleMode,
    this.sharedGrowthRule,
    this.growthRulesByMode,
    this.questionTypeWeightsByMode,
  });

  factory PlanSummary.fromJson(Map<String, dynamic> json) {
    final sharedRule =
        (json['sharedGrowthRule'] as Map?)?.cast<String, dynamic>() ??
        <String, dynamic>{
          'intervalDays': json['growthIntervalDays'] as int? ?? 7,
          'increment': json['growthIncrement'] as int? ?? 5,
        };
    final mode = (json['growthRuleMode'] as String?) ?? 'shared';
    final modes = <String, dynamic>{
      ...?((json['growthRulesByMode'] as Map?)?.cast<String, dynamic>()),
    };
    for (final key in const [
      'newWord',
      'review',
      'mixedTest',
      'wrongWordReinforcement',
      'rootAffix',
    ]) {
      modes[key] ??= Map<String, dynamic>.from(sharedRule);
    }

    return PlanSummary(
      id: json['id'] as int,
      name: json['name'] as String,
      newWordsPerDay: json['newWordsPerDay'] as int,
      reviewWordsPerDay: json['reviewWordsPerDay'] as int,
      mixedTestPerDay: json['mixedTestPerDay'] as int,
      wrongWordTestPerDay: json['wrongWordTestPerDay'] as int,
      rootAffixPerDay: json['rootAffixPerDay'] as int?,
      growthIntervalDays:
          (sharedRule['intervalDays'] as num?)?.toInt() ??
          (json['growthIntervalDays'] as int? ?? 7),
      growthIncrement:
          (sharedRule['increment'] as num?)?.toInt() ??
          (json['growthIncrement'] as int? ?? 5),
      growthRuleMode: mode,
      sharedGrowthRule: sharedRule,
      growthRulesByMode: modes,
      questionTypeWeightsByMode:
          (json['questionTypeWeightsByMode'] as Map?)?.cast<String, dynamic>(),
    );
  }
}

class WordbookSummary {
  final int id;
  final String code;
  final String name;
  final String category;
  final int totalEntries;
  final bool isActive;

  const WordbookSummary({
    required this.id,
    required this.code,
    required this.name,
    required this.category,
    required this.totalEntries,
    required this.isActive,
  });

  factory WordbookSummary.fromJson(Map<String, dynamic> json) => WordbookSummary(
        id: json['id'] as int,
        code: json['code'] as String,
        name: json['name'] as String,
        category: json['category'] as String,
        totalEntries: json['totalEntries'] as int,
        isActive: json['isActive'] as bool,
      );
}

class PlanClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const PlanClient(this._bridge, this._codec);

  Future<PlanSummary?> getActivePlan() async {
    final raw = await _bridge.call('getActivePlan');
    final json = _codec.decodeResponse(raw);
    if (json.isEmpty) return null;
    return PlanSummary.fromJson(json);
  }

  Future<PlanSummary> savePlan({
    required int planId,
    required Map<String, dynamic> input,
  }) async {
    final request = <String, dynamic>{'planId': planId, 'input': input};
    final raw = await _bridge.call('savePlan', _codec.encodeRequest(request));
    final json = _codec.decodeResponse(raw);
    return PlanSummary.fromJson(json);
  }

  Future<PlanSummary> applySavedPlanToToday() async {
    final raw = await _bridge.call('applySavedPlanToToday');
    final json = _codec.decodeResponse(raw);
    return PlanSummary.fromJson(json);
  }

  Future<List<WordbookSummary>> getWordbooks() async {
    final raw = await _bridge.call('getWordbooks');
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(WordbookSummary.fromJson)
        .toList(growable: false);
  }

  Future<void> toggleWordbook({
    required int wordbookId,
    required bool isActive,
  }) async {
    await _bridge.callVoid(
      'toggleWordbook',
      _codec.encodeRequest({'wordbookId': wordbookId, 'isActive': isActive}),
    );
  }
}

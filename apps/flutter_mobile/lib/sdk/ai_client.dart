library;

import '../bridge/bridge.dart';

class TodayAiPassageContext {
  final String date;
  final bool tasksComplete;
  final List<dynamic> wrongWords;

  const TodayAiPassageContext({
    required this.date,
    required this.tasksComplete,
    required this.wrongWords,
  });

  factory TodayAiPassageContext.fromJson(Map<String, dynamic> json) =>
      TodayAiPassageContext(
        date: json['date'] as String,
        tasksComplete: json['tasksComplete'] as bool,
        wrongWords: json['wrongWords'] as List<dynamic>? ?? const [],
      );

  List<Map<String, dynamic>> get generationWrongWords => wrongWords
      .map(_normalizeWrongWord)
      .whereType<Map<String, dynamic>>()
      .where((item) => '${item['word'] ?? ''}'.trim().isNotEmpty)
      .toList(growable: false);

  List<String> get generationTargetWords => wrongWords
      .map(_extractWrongWordText)
      .where((word) => word.isNotEmpty)
      .toList(growable: false);
}

Map<String, dynamic>? _normalizeWrongWord(dynamic item) {
  if (item is! Map) return null;
  final normalized = <String, dynamic>{};
  for (final entry in item.entries) {
    normalized['${entry.key}'] = entry.value;
  }
  return normalized;
}

String _extractWrongWordText(dynamic item) {
  if (item is String) return item.trim();
  if (item is Map) return '${item['word'] ?? ''}'.trim();
  return '';
}

class AiPassageHistoryItem {
  final String passageId;
  final String title;
  final String preview;
  final String generatedAt;
  final String validationStatus;

  const AiPassageHistoryItem({
    required this.passageId,
    required this.title,
    required this.preview,
    required this.generatedAt,
    required this.validationStatus,
  });

  factory AiPassageHistoryItem.fromJson(Map<String, dynamic> json) => AiPassageHistoryItem(
        passageId: json['passageId'] as String,
        title: json['title'] as String,
        preview: json['preview'] as String,
        generatedAt: json['generatedAt'] as String,
        validationStatus: json['validationStatus'] as String,
      );
}

class AiPassage {
  final String passageId;
  final String title;
  final List<dynamic> blocks;
  final String validationStatus;
  final String? failureReason;

  const AiPassage({
    required this.passageId,
    required this.title,
    required this.blocks,
    required this.validationStatus,
    this.failureReason,
  });

  factory AiPassage.fromJson(Map<String, dynamic> json) => AiPassage(
        passageId: json['passageId'] as String,
        title: json['title'] as String,
        blocks: json['blocks'] as List<dynamic>? ?? const [],
        validationStatus: json['validationStatus'] as String,
        failureReason: json['failureReason'] as String?,
      );
}

class AiClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const AiClient(this._bridge, this._codec);

  Future<TodayAiPassageContext> getTodayAiPassageContext() async {
    final raw = await _bridge.call('getTodayAiPassageContext');
    final json = _codec.decodeResponse(raw);
    return TodayAiPassageContext.fromJson(json);
  }

  Future<List<AiPassageHistoryItem>> getAiPassageHistory() async {
    final raw = await _bridge.call('getAiPassageHistory');
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(AiPassageHistoryItem.fromJson)
        .toList(growable: false);
  }

  Future<AiPassage?> getAiPassage(String passageId) async {
    final raw = await _bridge.call('getAiPassage', passageId);
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is Map<String, dynamic>) {
      return AiPassage.fromJson(decoded);
    }
    return null;
  }

  Future<AiPassage> generateAiPassage({
    required List<dynamic> wrongWords,
    List<String> targetWords = const [],
    required String level,
  }) async {
    final raw = await _bridge.call(
      'generateAiPassage',
      _codec.encodeRequest({
        'wrongWords': wrongWords,
        'targetWords': targetWords,
        'level': level,
      }),
    );
    final json = _codec.decodeResponse(raw);
    return AiPassage.fromJson(json);
  }
}

library;

import '../bridge/bridge.dart';

class WrongWordEntry {
  final int entryId;
  final String word;
  final String? phoneticUs;
  final String? phoneticUk;
  final List<dynamic> meanings;
  final int errorCount;
  final String? lastWrongAt;
  final double priorityScore;
  final bool isActive;

  const WrongWordEntry({
    required this.entryId,
    required this.word,
    this.phoneticUs,
    this.phoneticUk,
    required this.meanings,
    required this.errorCount,
    this.lastWrongAt,
    required this.priorityScore,
    required this.isActive,
  });

  factory WrongWordEntry.fromJson(Map<String, dynamic> json) => WrongWordEntry(
        entryId: json['entryId'] as int,
        word: json['word'] as String,
        phoneticUs: json['phoneticUs'] as String?,
        phoneticUk: json['phoneticUk'] as String?,
        meanings: json['meanings'] as List<dynamic>? ?? const [],
        errorCount: json['errorCount'] as int,
        lastWrongAt: json['lastWrongAt'] as String?,
        priorityScore: (json['priorityScore'] as num).toDouble(),
        isActive: json['isActive'] as bool? ?? true,
      );
}

class WrongWordDetail {
  final int entryId;
  final String word;
  final String lemma;
  final String? phoneticUs;
  final String? phoneticUk;
  final String partOfSpeech;
  final List<dynamic> meanings;
  final List<dynamic> examples;
  final List<dynamic> errorHistory;
  final List<dynamic> riskBreakdown;
  final List<dynamic> relatedWords;

  const WrongWordDetail({
    required this.entryId,
    required this.word,
    required this.lemma,
    this.phoneticUs,
    this.phoneticUk,
    required this.partOfSpeech,
    required this.meanings,
    required this.examples,
    required this.errorHistory,
    required this.riskBreakdown,
    required this.relatedWords,
  });

  factory WrongWordDetail.fromJson(Map<String, dynamic> json) => WrongWordDetail(
        entryId: json['entryId'] as int,
        word: json['word'] as String,
        lemma: json['lemma'] as String? ?? '',
        phoneticUs: json['phoneticUs'] as String?,
        phoneticUk: json['phoneticUk'] as String?,
        partOfSpeech: json['partOfSpeech'] as String? ?? '',
        meanings: json['meanings'] as List<dynamic>? ?? const [],
        examples: json['examples'] as List<dynamic>? ?? const [],
        errorHistory: json['errorHistory'] as List<dynamic>? ?? const [],
        riskBreakdown: json['riskBreakdown'] as List<dynamic>? ?? const [],
        relatedWords: json['relatedWords'] as List<dynamic>? ?? const [],
      );
}

class WrongWordsClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const WrongWordsClient(this._bridge, this._codec);

  Future<List<WrongWordEntry>> getWrongWords([String filter = 'all']) async {
    final raw = await _bridge.call('getWrongWords', filter);
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(WrongWordEntry.fromJson)
        .toList(growable: false);
  }

  Future<WrongWordDetail> getWrongWordDetail(int entryId) async {
    final raw = await _bridge.call('getWrongWordDetail', entryId.toString());
    final json = _codec.decodeResponse(raw);
    return WrongWordDetail.fromJson(json);
  }
}

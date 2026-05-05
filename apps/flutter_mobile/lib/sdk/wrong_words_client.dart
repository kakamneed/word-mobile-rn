library;

import 'dart:convert';

import '../bridge/bridge.dart';

class WordHintSuggestion {
  final String id;
  final String word;
  final String style;
  final String label;
  final String text;
  final List<String> wordbookCodes;

  const WordHintSuggestion({
    required this.id,
    required this.word,
    required this.style,
    required this.label,
    required this.text,
    this.wordbookCodes = const [],
  });

  factory WordHintSuggestion.fromJson(Map<String, dynamic> json) =>
      WordHintSuggestion(
        id: _stringValue(json['id']),
        word: _stringValue(json['word']),
        style: _stringValue(json['style'], fallback: 'meaning'),
        label: json['label'] as String? ?? 'AI推荐',
        text: _stringValue(json['text']),
        wordbookCodes: (json['wordbookCodes'] as List<dynamic>? ?? const [])
            .map(_stringValue)
            .where((value) => value.isNotEmpty)
            .toList(growable: false),
      );
}

String _stringValue(Object? value, {String fallback = ''}) {
  if (value == null) return fallback;
  if (value is String) return value;
  return value.toString();
}

class WordHintState {
  final int entryId;
  final String? userHint;
  final String? hintSource;
  final bool hasHint;

  const WordHintState({
    required this.entryId,
    this.userHint,
    this.hintSource,
    required this.hasHint,
  });

  factory WordHintState.fromJson(Map<String, dynamic> json) => WordHintState(
        entryId: json['entryId'] as int,
        userHint: json['userHint'] as String?,
        hintSource: json['hintSource'] as String?,
        hasHint: json['hasHint'] as bool? ?? false,
      );
}

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
  final String entryKind;
  final String? userHint;
  final String? hintSource;
  final bool hasHint;
  final List<WordHintSuggestion> hintSuggestions;

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
    required this.entryKind,
    this.userHint,
    this.hintSource,
    this.hasHint = false,
    this.hintSuggestions = const [],
  });

  bool get isRootAffix => entryKind == 'rootAffix';

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
        entryKind: json['entryKind'] as String? ?? 'word',
        userHint: json['userHint'] as String?,
        hintSource: json['hintSource'] as String?,
        hasHint: json['hasHint'] as bool? ?? false,
        hintSuggestions: (json['hintSuggestions'] as List<dynamic>? ?? const [])
            .whereType<Map<String, dynamic>>()
            .map(WordHintSuggestion.fromJson)
            .toList(growable: false),
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
  final int errorCount;
  final String? userHint;
  final String? hintSource;
  final bool hasHint;
  final List<WordHintSuggestion> hintSuggestions;

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
    required this.errorCount,
    this.userHint,
    this.hintSource,
    this.hasHint = false,
    this.hintSuggestions = const [],
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
        errorCount: json['errorCount'] as int? ?? 0,
        userHint: json['userHint'] as String?,
        hintSource: json['hintSource'] as String?,
        hasHint: json['hasHint'] as bool? ?? false,
        hintSuggestions: (json['hintSuggestions'] as List<dynamic>? ?? const [])
            .whereType<Map<String, dynamic>>()
            .map(WordHintSuggestion.fromJson)
            .toList(growable: false),
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

  Future<WordHintState> saveWordHint({
    required int entryId,
    required String hintText,
    String source = 'user',
  }) async {
    final raw = await _bridge.call(
      'saveWordHint',
      jsonEncode({
        'entryId': entryId,
        'hintText': hintText,
        'source': source,
      }),
    );
    return WordHintState.fromJson(_codec.decodeResponse(raw));
  }

  Future<List<WordHintSuggestion>> getHintSuggestions(int entryId) async {
    final raw = await _bridge.call('getWordHintSuggestions', entryId.toString());
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(WordHintSuggestion.fromJson)
        .toList(growable: false);
  }
}

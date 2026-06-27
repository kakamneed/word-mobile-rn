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

  factory WrongWordDetail.fromJson(Map<String, dynamic> json) =>
      WrongWordDetail(
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

class WrongWordGraphPosition {
  final double x;
  final double y;
  final double z;

  const WrongWordGraphPosition({
    required this.x,
    required this.y,
    required this.z,
  });

  factory WrongWordGraphPosition.fromJson(Map<String, dynamic> json) =>
      WrongWordGraphPosition(
        x: (json['x'] as num? ?? 0).toDouble(),
        y: (json['y'] as num? ?? 0).toDouble(),
        z: (json['z'] as num? ?? 0).toDouble(),
      );

  Map<String, dynamic> toJson() => {'x': x, 'y': y, 'z': z};
}

class WrongWordGraphNode {
  final String id;
  final int entryId;
  final String entryKind;
  final String word;
  final String primaryGloss;
  final List<dynamic> meanings;
  final int wrongCountToday;
  final int wrongCountTotal;
  final String? lastWrongAt;
  final double priorityScore;
  final double masteryScore;
  final double urgencyScore;
  final WrongWordGraphPosition position;
  final bool isUserPlaced;
  final String? positionUpdatedAt;
  final List<String> sources;

  const WrongWordGraphNode({
    required this.id,
    required this.entryId,
    required this.entryKind,
    required this.word,
    required this.primaryGloss,
    required this.meanings,
    required this.wrongCountToday,
    required this.wrongCountTotal,
    this.lastWrongAt,
    required this.priorityScore,
    required this.masteryScore,
    required this.urgencyScore,
    required this.position,
    required this.isUserPlaced,
    this.positionUpdatedAt,
    required this.sources,
  });

  factory WrongWordGraphNode.fromJson(Map<String, dynamic> json) =>
      WrongWordGraphNode(
        id: _stringValue(json['id']),
        entryId: json['entryId'] as int? ?? 0,
        entryKind: _stringValue(json['entryKind'], fallback: 'word'),
        word: _stringValue(json['word']),
        primaryGloss: _stringValue(json['primaryGloss']),
        meanings: json['meanings'] as List<dynamic>? ?? const [],
        wrongCountToday: json['wrongCountToday'] as int? ?? 0,
        wrongCountTotal: json['wrongCountTotal'] as int? ?? 0,
        lastWrongAt: json['lastWrongAt'] as String?,
        priorityScore: (json['priorityScore'] as num? ?? 0).toDouble(),
        masteryScore: (json['masteryScore'] as num? ?? 0).toDouble(),
        urgencyScore: (json['urgencyScore'] as num? ?? 0).toDouble(),
        position: WrongWordGraphPosition.fromJson(
          json['position'] as Map<String, dynamic>? ?? const {},
        ),
        isUserPlaced: json['isUserPlaced'] as bool? ?? false,
        positionUpdatedAt: json['positionUpdatedAt'] as String?,
        sources: (json['sources'] as List<dynamic>? ?? const [])
            .map(_stringValue)
            .where((value) => value.isNotEmpty)
            .toList(growable: false),
      );

  WrongWordGraphNode copyWith({
    WrongWordGraphPosition? position,
    bool? isUserPlaced,
    String? positionUpdatedAt,
  }) => WrongWordGraphNode(
    id: id,
    entryId: entryId,
    entryKind: entryKind,
    word: word,
    primaryGloss: primaryGloss,
    meanings: meanings,
    wrongCountToday: wrongCountToday,
    wrongCountTotal: wrongCountTotal,
    lastWrongAt: lastWrongAt,
    priorityScore: priorityScore,
    masteryScore: masteryScore,
    urgencyScore: urgencyScore,
    position: position ?? this.position,
    isUserPlaced: isUserPlaced ?? this.isUserPlaced,
    positionUpdatedAt: positionUpdatedAt ?? this.positionUpdatedAt,
    sources: sources,
  );
}

class WrongWordGraphEdge {
  final String id;
  final String sourceNodeId;
  final String targetNodeId;
  final String relationType;
  final double weight;
  final List<dynamic> evidence;

  const WrongWordGraphEdge({
    required this.id,
    required this.sourceNodeId,
    required this.targetNodeId,
    required this.relationType,
    required this.weight,
    required this.evidence,
  });

  factory WrongWordGraphEdge.fromJson(Map<String, dynamic> json) =>
      WrongWordGraphEdge(
        id: _stringValue(json['id']),
        sourceNodeId: _stringValue(json['sourceNodeId']),
        targetNodeId: _stringValue(json['targetNodeId']),
        relationType: _stringValue(json['relationType']),
        weight: (json['weight'] as num? ?? 0).toDouble(),
        evidence: json['evidence'] as List<dynamic>? ?? const [],
      );
}

class WrongWordGraph {
  final int version;
  final String generatedAt;
  final List<WrongWordGraphNode> nodes;
  final List<WrongWordGraphEdge> edges;
  final Map<String, dynamic> coordinateSemantics;
  final List<dynamic> relationLegend;
  final Map<String, dynamic> viewportHint;

  const WrongWordGraph({
    required this.version,
    required this.generatedAt,
    required this.nodes,
    required this.edges,
    required this.coordinateSemantics,
    required this.relationLegend,
    required this.viewportHint,
  });

  factory WrongWordGraph.fromJson(Map<String, dynamic> json) => WrongWordGraph(
    version: json['version'] as int? ?? 1,
    generatedAt: _stringValue(json['generatedAt']),
    nodes: (json['nodes'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(WrongWordGraphNode.fromJson)
        .toList(growable: false),
    edges: (json['edges'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(WrongWordGraphEdge.fromJson)
        .toList(growable: false),
    coordinateSemantics:
        json['coordinateSemantics'] as Map<String, dynamic>? ?? const {},
    relationLegend: json['relationLegend'] as List<dynamic>? ?? const [],
    viewportHint: json['viewportHint'] as Map<String, dynamic>? ?? const {},
  );

  WrongWordGraph copyWith({List<WrongWordGraphNode>? nodes}) => WrongWordGraph(
    version: version,
    generatedAt: generatedAt,
    nodes: nodes ?? this.nodes,
    edges: edges,
    coordinateSemantics: coordinateSemantics,
    relationLegend: relationLegend,
    viewportHint: viewportHint,
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

  Future<WrongWordGraph> getWrongWordGraph() async {
    final raw = await _bridge.call('getWrongWordGraph');
    return WrongWordGraph.fromJson(_codec.decodeResponse(raw));
  }

  Future<WrongWordGraphNode> saveWrongWordGraphPosition({
    required int entryId,
    required WrongWordGraphPosition position,
    String entryKind = 'word',
  }) async {
    final raw = await _bridge.call(
      'saveWrongWordGraphPosition',
      jsonEncode({
        'entryId': entryId,
        'entryKind': entryKind,
        'position': position.toJson(),
      }),
    );
    final json = _codec.decodeResponse(raw);
    return WrongWordGraphNode.fromJson({
      'id': '${json['entryKind'] ?? entryKind}:${json['entryId'] ?? entryId}',
      'entryId': json['entryId'] ?? entryId,
      'entryKind': json['entryKind'] ?? entryKind,
      'word': '',
      'position': json['position'],
      'isUserPlaced': json['isUserPlaced'] ?? true,
      'positionUpdatedAt': json['positionUpdatedAt'],
    });
  }

  Future<WordHintState> saveWordHint({
    required int entryId,
    required String hintText,
    String source = 'user',
  }) async {
    final raw = await _bridge.call(
      'saveWordHint',
      jsonEncode({'entryId': entryId, 'hintText': hintText, 'source': source}),
    );
    return WordHintState.fromJson(_codec.decodeResponse(raw));
  }

  Future<List<WordHintSuggestion>> getHintSuggestions(int entryId) async {
    final raw = await _bridge.call(
      'getWordHintSuggestions',
      entryId.toString(),
    );
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! List) return const [];
    return decoded
        .whereType<Map<String, dynamic>>()
        .map(WordHintSuggestion.fromJson)
        .toList(growable: false);
  }
}

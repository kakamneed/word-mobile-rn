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
  final String? date;
  final String validationStatus;

  const AiPassageHistoryItem({
    required this.passageId,
    required this.title,
    required this.preview,
    required this.generatedAt,
    this.date,
    required this.validationStatus,
  });

  factory AiPassageHistoryItem.fromJson(Map<String, dynamic> json) =>
      AiPassageHistoryItem(
        passageId: json['passageId'] as String,
        title: json['title'] as String,
        preview: json['preview'] as String,
        generatedAt: json['generatedAt'] as String,
        date: json['date'] as String?,
        validationStatus: json['validationStatus'] as String,
      );
}

class AiPassage {
  final String passageId;
  final String title;
  final List<dynamic> blocks;
  final List<dynamic> wrongWords;
  final String validationStatus;
  final String? failureReason;
  final String? date;

  const AiPassage({
    required this.passageId,
    required this.title,
    required this.blocks,
    required this.wrongWords,
    required this.validationStatus,
    this.failureReason,
    this.date,
  });

  factory AiPassage.fromJson(Map<String, dynamic> json) {
    final wrongWords = json['wrongWords'] as List<dynamic>? ?? const [];
    return AiPassage(
      passageId: json['passageId'] as String,
      title: json['title'] as String,
      blocks: _normalizePassageBlocks(
        json['blocks'] as List<dynamic>? ?? const [],
        wrongWords,
      ),
      wrongWords: wrongWords,
      validationStatus: json['validationStatus'] as String,
      failureReason: json['failureReason'] as String?,
      date: json['date'] as String?,
    );
  }
}

List<dynamic> _normalizePassageBlocks(
  List<dynamic> blocks,
  List<dynamic> wrongWords,
) {
  if (_hasWordSegments(blocks) || wrongWords.isEmpty) return blocks;
  final lookup = <String, Map<String, dynamic>>{};
  for (final item in wrongWords) {
    final normalized = _normalizeWrongWord(item);
    final word = '${normalized?['word'] ?? ''}'.trim();
    if (word.isNotEmpty && normalized != null) {
      lookup[word.toLowerCase()] = normalized;
    }
  }
  if (lookup.isEmpty) return blocks;

  return blocks.map((block) {
    if (block is Map && block['segments'] is List) return block;
    final text = block is String
        ? block
        : block is Map
        ? '${block['text'] ?? block['content'] ?? ''}'
        : '';
    if (text.trim().isEmpty) return block;
    return {
      'blockType': block is Map ? block['blockType'] ?? 'paragraph' : 'paragraph',
      'segments': _highlightTextSegments(text, lookup),
    };
  }).toList(growable: false);
}

bool _hasWordSegments(List<dynamic> blocks) {
  for (final block in blocks) {
    if (block is! Map) continue;
    final segments = block['segments'];
    if (segments is! List) continue;
    for (final segment in segments) {
      if (segment is Map && segment['type'] == 'word') return true;
    }
  }
  return false;
}

List<Map<String, dynamic>> _highlightTextSegments(
  String text,
  Map<String, Map<String, dynamic>> wrongWords,
) {
  final lower = text.toLowerCase();
  var cursor = 0;
  final segments = <Map<String, dynamic>>[];
  while (cursor < text.length) {
    ({int start, int end, Map<String, dynamic> metadata})? best;
    for (final entry in wrongWords.entries) {
      final relative = lower.indexOf(entry.key, cursor);
      if (relative < 0) continue;
      final start = relative;
      final end = start + entry.key.length;
      final current = best;
      if (current == null ||
          start < current.start ||
          (start == current.start && end > current.end)) {
        best = (start: start, end: end, metadata: entry.value);
      }
    }
    final match = best;
    if (match == null) {
      segments.add({'type': 'text', 'text': text.substring(cursor)});
      break;
    }
    if (match.start > cursor) {
      segments.add({'type': 'text', 'text': text.substring(cursor, match.start)});
    }
    segments.add({
      'type': 'word',
      'text': text.substring(match.start, match.end),
      'entryId': (match.metadata['entryId'] as num?)?.toInt() ?? 0,
      'glossZh':
          '${match.metadata['primaryGloss'] ?? match.metadata['glossZh'] ?? ''}',
      'highlighted': true,
    });
    cursor = match.end;
  }
  return segments;
}

class AiWrongWordImportCandidate {
  final String candidateId;
  final String word;
  final String? meaning;
  final int occurrenceCount;
  final double confidence;
  final bool isDuplicate;
  final bool isHighFrequency;
  final String evidence;

  const AiWrongWordImportCandidate({
    required this.candidateId,
    required this.word,
    this.meaning,
    required this.occurrenceCount,
    required this.confidence,
    required this.isDuplicate,
    required this.isHighFrequency,
    required this.evidence,
  });

  factory AiWrongWordImportCandidate.fromJson(Map<String, dynamic> json) {
    final word = '${json['word'] ?? ''}'.trim();
    final occurrenceCount = (json['occurrenceCount'] as num?)?.toInt() ?? 1;
    return AiWrongWordImportCandidate(
      candidateId: '${json['candidateId'] ?? word.toLowerCase()}',
      word: word,
      meaning: json['meaning'] as String?,
      occurrenceCount: occurrenceCount,
      confidence: (json['confidence'] as num?)?.toDouble() ?? 0.0,
      isDuplicate: json['isDuplicate'] as bool? ?? false,
      isHighFrequency: json['isHighFrequency'] as bool? ?? occurrenceCount >= 3,
      evidence: '${json['evidence'] ?? 'Imported evidence'}',
    );
  }

  Map<String, dynamic> toJson() => {
    'candidateId': candidateId,
    'word': word,
    if (meaning != null) 'meaning': meaning,
    'occurrenceCount': occurrenceCount,
    'confidence': confidence,
    'isDuplicate': isDuplicate,
    'isHighFrequency': isHighFrequency,
    'evidence': evidence,
  };
}

class AiWrongWordImportSource {
  final String sourceType;
  final String sourceName;
  final String? textContent;
  final String? bytesBase64;
  final String? mimeType;

  const AiWrongWordImportSource({
    required this.sourceType,
    required this.sourceName,
    this.textContent,
    this.bytesBase64,
    this.mimeType,
  });

  factory AiWrongWordImportSource.fromJson(Map<String, dynamic> json) {
    return AiWrongWordImportSource(
      sourceType: json['sourceType'] as String? ?? 'text',
      sourceName: json['sourceName'] as String? ?? 'imported-source',
      textContent: json['textContent'] as String?,
      bytesBase64: json['bytesBase64'] as String?,
      mimeType: json['mimeType'] as String?,
    );
  }
}

class AiWrongWordImportAnalysis {
  final String batchId;
  final String sourceType;
  final String sourceName;
  final List<AiWrongWordImportCandidate> candidates;
  final List<String> warnings;

  const AiWrongWordImportAnalysis({
    required this.batchId,
    required this.sourceType,
    required this.sourceName,
    required this.candidates,
    required this.warnings,
  });

  factory AiWrongWordImportAnalysis.fromJson(Map<String, dynamic> json) {
    final decodedCandidates = json['candidates'] as List<dynamic>? ?? const [];
    return AiWrongWordImportAnalysis(
      batchId: json['batchId'] as String? ?? 'preview-import',
      sourceType: json['sourceType'] as String? ?? 'text',
      sourceName:
          json['sourceName'] as String? ??
          json['sourceType'] as String? ??
          'imported-source',
      candidates: decodedCandidates
          .whereType<Map<String, dynamic>>()
          .map(AiWrongWordImportCandidate.fromJson)
          .where((candidate) => candidate.word.isNotEmpty)
          .toList(growable: false),
      warnings: (json['warnings'] as List<dynamic>? ?? const [])
          .map((item) => '$item')
          .where((item) => item.trim().isNotEmpty)
          .toList(growable: false),
    );
  }
}

class AiWrongWordImportCommitResult {
  final bool persisted;
  final List<AiWrongWordImportCandidate> added;
  final List<AiWrongWordImportCandidate> skipped;
  final List<AiWrongWordImportCandidate> highFrequency;

  const AiWrongWordImportCommitResult({
    required this.persisted,
    required this.added,
    required this.skipped,
    required this.highFrequency,
  });

  factory AiWrongWordImportCommitResult.fromJson(Map<String, dynamic> json) {
    List<AiWrongWordImportCandidate> candidatesFrom(String key) {
      final raw = json[key] as List<dynamic>? ?? const [];
      return raw
          .whereType<Map<String, dynamic>>()
          .map(AiWrongWordImportCandidate.fromJson)
          .toList(growable: false);
    }

    final added = candidatesFrom('added');
    final highFrequency = candidatesFrom('highFrequency');
    return AiWrongWordImportCommitResult(
      persisted: json['persisted'] as bool? ?? false,
      added: added,
      skipped: candidatesFrom('skipped'),
      highFrequency: highFrequency.isEmpty
          ? added.where((candidate) => candidate.isHighFrequency).toList()
          : highFrequency,
    );
  }
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
    String? date,
    String? style,
  }) async {
    final request = <String, dynamic>{
      'wrongWords': wrongWords,
      'targetWords': targetWords,
      'level': level,
    };
    if (date != null) {
      request['date'] = date;
    }
    if (style != null && style.trim().isNotEmpty) {
      request['style'] = style.trim();
    }
    final raw = await _bridge.call(
      'generateAiPassage',
      _codec.encodeRequest(request),
    );
    final json = _codec.decodeResponse(raw);
    return AiPassage.fromJson(json);
  }

  Future<AiWrongWordImportAnalysis> analyzeWrongWordImport({
    required String sourceType,
    required String sourceName,
    String? textContent,
    String? bytesBase64,
    String? mimeType,
  }) async {
    final request = <String, dynamic>{
      'sourceType': sourceType,
      'sourceName': sourceName,
    };
    if (textContent != null) {
      request['textContent'] = textContent;
    }
    if (bytesBase64 != null) {
      request['bytesBase64'] = bytesBase64;
    }
    if (mimeType != null) {
      request['mimeType'] = mimeType;
    }
    final raw = await _bridge.call(
      'analyzeWrongWordImport',
      _codec.encodeRequest(request),
    );
    return AiWrongWordImportAnalysis.fromJson(_codec.decodeResponse(raw));
  }

  Future<AiWrongWordImportSource?> pickWrongWordImportSource({
    required String sourceType,
  }) async {
    final request = _codec.encodeRequest({'sourceType': sourceType});
    final raw = await _bridge.call('pickWrongWordImportSource', request);
    final decoded = _codec.decodeDynamicResponse(raw);
    if (decoded is! Map<String, dynamic>) return null;
    if (decoded['cancelled'] == true) return null;
    return AiWrongWordImportSource.fromJson(decoded);
  }

  Future<AiWrongWordImportCommitResult> commitWrongWordImport({
    required AiWrongWordImportAnalysis analysis,
    required Set<String> acceptedCandidateIds,
  }) async {
    final request = {
      'batchId': analysis.batchId,
      'sourceType': analysis.sourceType,
      'sourceName': analysis.sourceName,
      'acceptedCandidateIds': acceptedCandidateIds.toList(growable: false),
      'candidates': analysis.candidates
          .map((candidate) => candidate.toJson())
          .toList(growable: false),
    };
    final raw = await _bridge.call(
      'commitWrongWordImport',
      _codec.encodeRequest(request),
    );
    return AiWrongWordImportCommitResult.fromJson(_codec.decodeResponse(raw));
  }

  /// Returns the current AI provider configuration summary.
  /// Shows provider names, models, and redacted auth tokens.
  Future<Map<String, dynamic>> getAiProviderConfig() async {
    final raw = await _bridge.call('getAiProviderConfig');
    return _codec.decodeResponse(raw);
  }

  /// Saves a new AI provider configuration.
  Future<Map<String, dynamic>> saveAiProviderConfig(
    Map<String, dynamic> config,
  ) async {
    final raw = await _bridge.call(
      'saveAiProviderConfig',
      _codec.encodeRequest(config),
    );
    return _codec.decodeResponse(raw);
  }
}

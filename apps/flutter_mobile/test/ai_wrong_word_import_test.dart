import 'package:flutter_mobile/sdk/ai_client.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('parses import candidates and derives high frequency from count', () {
    final analysis = AiWrongWordImportAnalysis.fromJson({
      'batchId': 'batch-1',
      'sourceType': 'image',
      'warnings': ['low confidence handwriting'],
      'candidates': [
        {
          'candidateId': 'abandon',
          'word': ' abandon ',
          'meaning': 'give up',
          'occurrenceCount': 3,
          'confidence': 0.81,
          'evidence': 'appears three times',
        },
        {'candidateId': 'blank', 'word': '   '},
      ],
    });

    expect(analysis.batchId, 'batch-1');
    expect(analysis.sourceType, 'image');
    expect(analysis.warnings, ['low confidence handwriting']);
    expect(analysis.candidates, hasLength(1));
    expect(analysis.candidates.single.word, 'abandon');
    expect(analysis.candidates.single.isHighFrequency, isTrue);
  });

  test('commit result separates added duplicate and high frequency words', () {
    final result = AiWrongWordImportCommitResult.fromJson({
      'persisted': true,
      'added': [
        {
          'candidateId': 'abandon',
          'word': 'abandon',
          'occurrenceCount': 4,
          'confidence': 0.9,
          'evidence': 'import count',
        },
      ],
      'skipped': [
        {
          'candidateId': 'known',
          'word': 'known',
          'occurrenceCount': 1,
          'confidence': 0.9,
          'isDuplicate': true,
          'evidence': 'already exists',
        },
      ],
    });

    expect(result.persisted, isTrue);
    expect(result.added.map((item) => item.word), ['abandon']);
    expect(result.skipped.single.isDuplicate, isTrue);
    expect(result.highFrequency.map((item) => item.word), ['abandon']);
  });

  test('text import analysis does not fall back to local preview extraction', () async {
    final client = AiClient(
      _MissingMethodBridge(),
      const BridgeCodec(),
    );

    expect(
      () => client.analyzeWrongWordImport(
        sourceType: 'text',
        sourceName: 'pasted-wrong-word-record',
        textContent: 'abandon abandon known',
      ),
      throwsA(
        isA<BridgeError>()
            .having((error) => error.code, 'code', 'METHOD_MISSING'),
      ),
    );
  });

  test('wrong-word import commit does not synthesize non-persisted results', () async {
    final client = AiClient(
      _MissingMethodBridge(),
      const BridgeCodec(),
    );
    final analysis = AiWrongWordImportAnalysis.fromJson({
      'batchId': 'batch-1',
      'sourceType': 'text',
      'sourceName': 'paste',
      'candidates': [
        {
          'candidateId': 'abandon',
          'word': 'abandon',
          'occurrenceCount': 2,
          'confidence': 0.9,
          'evidence': 'pasted text',
        },
      ],
    });

    expect(
      () => client.commitWrongWordImport(
        analysis: analysis,
        acceptedCandidateIds: {'abandon'},
      ),
      throwsA(
        isA<BridgeError>()
            .having((error) => error.code, 'code', 'METHOD_MISSING'),
      ),
    );
  });
}

class _MissingMethodBridge implements RustBridge {
  @override
  Future<String> call(String method, [String? argument]) {
    throw BridgeError.platform(
      'METHOD_MISSING',
      'Method $method not found on native side',
    );
  }

  @override
  Future<void> callVoid(String method, [String? argument]) async {
    throw BridgeError.platform(
      'METHOD_MISSING',
      'Method $method not found on native side',
    );
  }

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> isAvailable() async => false;
}

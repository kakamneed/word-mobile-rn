import 'package:flutter_mobile/sdk/ai_client.dart';
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
}

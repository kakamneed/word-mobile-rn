import 'package:flutter_mobile/sdk/ai_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('normalizes wrong words for AI passage generation', () {
    final context = TodayAiPassageContext.fromJson({
      'date': '2026-04-29',
      'tasksComplete': true,
      'wrongWords': [
        {
          'entryId': 1,
          'word': 'alpha',
          'primaryGloss': 'alpha meaning',
          'partOfSpeech': 'n.',
        },
        {
          'entryId': 2,
          'word': '  beta  ',
          'primaryGloss': 'beta meaning',
          'partOfSpeech': 'v.',
        },
        {'entryId': 3, 'word': '   '},
      ],
    });

    expect(context.generationWrongWords, hasLength(2));
    expect(context.generationWrongWords.first['word'], 'alpha');
    expect(context.generationTargetWords, ['alpha', 'beta']);
  });

  test('keeps target words as fallback when only word strings are available', () {
    final context = TodayAiPassageContext.fromJson({
      'date': '2026-04-29',
      'tasksComplete': true,
      'wrongWords': ['alpha', ' beta ', ''],
    });

    expect(context.generationWrongWords, isEmpty);
    expect(context.generationTargetWords, ['alpha', 'beta']);
  });
}

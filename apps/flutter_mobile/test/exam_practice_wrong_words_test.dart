import 'package:flutter_mobile/features/wrong_words_screen.dart';
import 'package:flutter_mobile/sdk/exam_practice_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('exam wrong words weight fuzzy familiar unknown and causal marks', () {
    ExamVocabularyPriority item(
      String word,
      int causal,
      int unknown,
      int familiar,
      int fuzzy,
    ) => ExamVocabularyPriority(
      word: word,
      priorityScore: 999,
      paperCount: 20,
      articleCount: 20,
      occurrenceCount: 20,
      fuzzyMarkCount: fuzzy,
      familiarMarkCount: familiar,
      unknownMarkCount: unknown,
      wrongAssociationCount: causal,
      masteredMarkCount: 0,
      factors: const [],
      sources: const [],
    );

    final sorted = sortExamPracticeWrongWords([
      item('fuzzy-heavy', 0, 0, 0, 4),
      item('familiar-heavy', 0, 0, 2, 0),
      item('unknown-heavy', 0, 2, 0, 0),
      item('causal', 1, 0, 0, 0),
      item('unmarked', 0, 0, 0, 0),
    ]);

    expect(sorted.map((item) => item.word), [
      'unknown-heavy',
      'causal',
      'familiar-heavy',
      'fuzzy-heavy',
    ]);
  });
}

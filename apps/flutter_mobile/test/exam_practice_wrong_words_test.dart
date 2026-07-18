import 'package:flutter_mobile/features/wrong_words_screen.dart';
import 'package:flutter_mobile/sdk/exam_practice_client.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('exam wrong words sort by total red/yellow marks then red marks', () {
    ExamVocabularyPriority item(String word, int red, int yellow) =>
        ExamVocabularyPriority(
          word: word,
          priorityScore: 999,
          paperCount: 20,
          articleCount: 20,
          occurrenceCount: 20,
          unknownMarkCount: yellow,
          wrongAssociationCount: red,
          masteredMarkCount: 0,
          factors: const [],
        );

    final sorted = sortExamPracticeWrongWords([
      item('yellow-heavy', 1, 4),
      item('red-heavy', 3, 2),
      item('fewer', 2, 1),
      item('unmarked', 0, 0),
    ]);

    expect(sorted.map((item) => item.word), [
      'red-heavy',
      'yellow-heavy',
      'fewer',
    ]);
  });
}

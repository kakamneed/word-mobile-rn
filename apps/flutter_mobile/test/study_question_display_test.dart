import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_mobile/features/study_screen.dart';
import 'package:flutter_mobile/sdk/study_client.dart';

void main() {
  test(
    'cnToEn hero displays the Chinese prompt instead of the English answer',
    () {
      const question = StudyQuestion(
        questionId: 'q1',
        questionType: 'cnToEnChoice',
        entrySourceId: 'surgeon',
        word: 'surgeon',
        prompt: '外科医生',
        partOfSpeech: 'n',
        acceptedMeanings: ['外科医生'],
        choices: [
          {'label': 'A', 'text': 'wreck'},
          {'label': 'B', 'text': 'rabbit'},
          {'label': 'C', 'text': 'reproach'},
          {'label': 'D', 'text': 'surgeon'},
        ],
        correctChoiceLabel: 'D',
        questionIndex: 10,
        totalQuestions: 20,
      );

      final display = studyHeroDisplayForTest(question);

      expect(display.title, '外科医生');
      expect(display.subtitle, isNull);
      expect(display.title, isNot('surgeon'));
    },
  );

  test('non cnToEn hero keeps the studied word and part of speech', () {
    const question = StudyQuestion(
      questionId: 'q2',
      questionType: 'enToCnChoice',
      entrySourceId: 'abrupt',
      word: 'abrupt',
      prompt: 'abrupt',
      partOfSpeech: 'adj',
      acceptedMeanings: ['突然的'],
      questionIndex: 3,
      totalQuestions: 20,
    );

    final display = studyHeroDisplayForTest(question);

    expect(display.title, 'abrupt');
    expect(display.subtitle, 'adj');
  });
}

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_mobile/features/croc_bti_model.dart';
import 'package:flutter_mobile/sdk/plan_client.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('maps strong VINT answers to 鳄骑兵', () {
    final answers = _answersForTraits({'V', 'I', 'N', 'T'});

    final result = evaluateCrocBti(answers);

    expect(result.code, 'VINT');
    expect(result.title, '鳄骑兵');
    expect(result.assetPath, 'assets/croc_bti/cavalry_croc.png');
    expect(result.summary, contains('冲锋'));
    expect(result.axisScores['vc']!.strength, 'clear');
    expect(result.weights['newWords']!, greaterThan(result.weights['review']!));
    expect(
      result.weights['mixedTest']!,
      greaterThan(result.weights['contextExamples']!),
    );
  });

  test('maps strong CORA answers to 鳄渊者', () {
    final answers = _answersForTraits({'C', 'O', 'R', 'A'});

    final result = evaluateCrocBti(answers);

    expect(result.code, 'CORA');
    expect(result.title, '鳄渊者');
    expect(result.assetPath, 'assets/croc_bti/stargazer_croc.jpg');
    expect(result.summary, contains('星盘'));
    expect(result.weights['review']!, greaterThan(result.weights['newWords']!));
    expect(
      result.weights['contextExamples']!,
      greaterThan(result.weights['activeRecall']!),
    );
  });

  test('normalizes recommendations to 100 percent', () {
    final weights = calculateCrocBtiWeights(['C', 'O', 'R', 'T']);

    expect(weights.values.fold<int>(0, (sum, value) => sum + value), 100);
  });

  test('uses current personality title as applied plan name', () {
    final result = evaluateCrocBti(_answersForTraits({'V', 'I', 'N', 'T'}));
    const plan = PlanSummary(
      id: 1,
      name: 'Starter Plan',
      newWordsPerDay: 10,
      reviewWordsPerDay: 20,
      mixedTestPerDay: 5,
      wrongWordTestPerDay: 3,
      rootAffixPerDay: 1,
      growthIntervalDays: 7,
      growthIncrement: 5,
      growthRuleMode: 'shared',
    );

    final input = crocBtiPlanInputFor(plan, result);

    expect(input['name'], result.title);
  });

  test('scales plan recommendation by daily learning minutes', () {
    final result = evaluateCrocBti(_answersForTraits({'V', 'I', 'N', 'T'}));

    final tenMinutes = crocBtiPlanInputForDailyMinutes(result.weights, 10);
    final short = crocBtiPlanInputForDailyMinutes(result.weights, 20);
    final long = crocBtiPlanInputForDailyMinutes(result.weights, 80);

    expect(tenMinutes.values.fold<int>(0, (sum, value) => sum + value), 40);
    expect(short.values.fold<int>(0, (sum, value) => sum + value), 80);
    expect(long['newWordsPerDay']!, greaterThan(short['newWordsPerDay']!));
    expect(long['newWordsPerDay']! % 4, 0);
    expect(short['newWordsPerDay']! % 4, 0);
    expect(
      long['reviewWordsPerDay']!,
      greaterThan(short['reviewWordsPerDay']!),
    );
    expect(long['mixedTestPerDay']!, greaterThan(short['mixedTestPerDay']!));
  });

  test('normalizes question type recommendations per mode', () {
    final result = evaluateCrocBti(_answersForTraits({'V', 'I', 'N', 'T'}));

    expect(
      result.questionTypeWeightsByMode.keys,
      containsAll(['review', 'mixedTest', 'wrongWordReinforcement']),
    );
    expect(result.questionTypeWeightsByMode.keys, isNot(contains('newWord')));
    expect(result.questionTypeWeightsByMode.keys, isNot(contains('rootAffix')));
    for (final weights in result.questionTypeWeightsByMode.values) {
      expect(weights.values.fold<int>(0, (sum, value) => sum + value), 100);
    }
  });

  test('personalizes mixed test question types by croc bti profile', () {
    final cavalry = evaluateCrocBti(_answersForTraits({'V', 'I', 'N', 'T'}));
    final abyss = evaluateCrocBti(_answersForTraits({'C', 'O', 'R', 'A'}));

    final cavalryMix = cavalry.questionTypeWeightsByMode['mixedTest']!;
    final abyssMix = abyss.questionTypeWeightsByMode['mixedTest']!;

    expect(cavalryMix, isNot(abyssMix));
    expect(cavalryMix['enToCnChoice']!, greaterThan(abyssMix['enToCnChoice']!));
    expect(
      abyssMix['exampleToCnChoiceNoTranslation']!,
      greaterThan(cavalryMix['exampleToCnChoiceNoTranslation']!),
    );
    expect(
      abyssMix['wordSkeletonInput']!,
      greaterThan(cavalryMix['wordSkeletonInput']!),
    );
  });

  test('normalizes edited question type weights before plan save', () {
    final normalized = normalizeCrocBtiQuestionTypeWeightsByMode({
      'newWord': {
        'enToCnChoice': 25,
        'exampleToCnChoice': 25,
        'cnToEnChoice': 25,
        'enToCnInput': 25,
      },
      'mixedTest': {
        'enToCnChoice': 40,
        'exampleToCnChoice': 30,
        'exampleToCnChoiceNoTranslation': 30,
        'wordSkeletonInput': 30,
      },
      'rootAffix': {'rootToGlossInput': 50, 'glossToRootInput': 50},
    });

    expect(normalized.keys, isNot(contains('newWord')));
    expect(normalized.keys, isNot(contains('rootAffix')));
    expect(
      normalized['mixedTest']!.values.fold<int>(0, (sum, value) => sum + value),
      100,
    );
    expect(normalized['mixedTest']!['enToCnChoice'], greaterThan(0));
  });

  test('persists complete answers for default result display', () async {
    final answers = _answersForTraits({'V', 'I', 'N', 'T'});

    await saveCrocBtiAnswers(answers);
    final loaded = await loadSavedCrocBtiAnswers();

    expect(loaded, answers);
    expect(hasCompleteCrocBtiAnswers(loaded), isTrue);
  });

  test('keeps croc bti answers isolated by account scope', () async {
    final guestAnswers = _answersForTraits({'C', 'O', 'R', 'A'});
    final accountAnswers = _answersForTraits({'V', 'I', 'N', 'T'});

    await saveCrocBtiAnswers(guestAnswers);
    await saveCrocBtiDailyMinutes(20);
    await saveCrocBtiAnswers(accountAnswers, userId: 'user-1');
    await saveCrocBtiDailyMinutes(55, userId: 'user-1');

    expect(await loadSavedCrocBtiAnswers(), guestAnswers);
    expect(await loadSavedCrocBtiDailyMinutes(), 20);
    expect(await loadSavedCrocBtiAnswers(userId: 'user-1'), accountAnswers);
    expect(await loadSavedCrocBtiDailyMinutes(userId: 'user-1'), 55);
  });

  test('ignores legacy unscoped croc bti answers in guest mode', () async {
    SharedPreferences.setMockInitialValues({
      'croc_bti_saved_answers_v1': '{"vc-word-volume":2}',
      'croc_bti_daily_minutes_v1': 80,
    });

    expect(await loadSavedCrocBtiAnswers(), isEmpty);
    expect(await loadSavedCrocBtiDailyMinutes(), 40);
  });

  test('clears saved answers before retake', () async {
    await saveCrocBtiAnswers(_answersForTraits({'C', 'O', 'R', 'A'}));

    await clearSavedCrocBtiAnswers();
    final loaded = await loadSavedCrocBtiAnswers();

    expect(loaded, isEmpty);
    expect(hasCompleteCrocBtiAnswers(loaded), isFalse);
  });
}

Map<String, int> _answersForTraits(Set<String> traits) {
  return {
    for (final question in crocBtiQuestions)
      question.id: traits.contains(question.positiveTrait) ? 2 : -2,
  };
}

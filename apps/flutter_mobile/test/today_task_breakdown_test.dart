import 'package:flutter_mobile/features/today_shell_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  const plan = PlanSummary(
    id: 1,
    name: 'Test Plan',
    newWordsPerDay: 2,
    reviewWordsPerDay: 3,
    mixedTestPerDay: 4,
    wrongWordTestPerDay: 5,
    rootAffixPerDay: 2,
    growthIntervalDays: 7,
    growthIncrement: 5,
    growthRuleMode: 'shared',
  );

  test('task breakdown keeps plan-backed modes even when pools are empty', () {
    final snapshot = <String, dynamic>{
      'newWordsTarget': 8,
      'newWordsCompleted': 2,
      'reviewWordsTarget': 0,
      'reviewWordsCompleted': 0,
      'mixedTestTarget': 4,
      'mixedTestCompleted': 1,
      'wrongWordTestTarget': 0,
      'wrongWordTestCompleted': 0,
      'rootAffixTarget': 2,
      'rootAffixCompleted': 0,
    };

    expect(todayTaskBreakdownModesForTest(snapshot, plan), [
      'newWord',
      'review',
      'mixedTest',
      'wrongWordReinforcement',
      'rootAffix',
    ]);
    expect(todayTaskBreakdownRowsForTest(snapshot, plan), [
      'newWord:2/8',
      'review:0/0',
      'mixedTest:1/4',
      'wrongWordReinforcement:0/0',
      'rootAffix:0/2',
    ]);
  });

  test('completion uses the same display targets as task breakdown', () {
    final snapshot = <String, dynamic>{
      'newWordsTarget': 8,
      'newWordsCompleted': 8,
      'reviewWordsTarget': 0,
      'reviewWordsCompleted': 0,
      'mixedTestTarget': 4,
      'mixedTestCompleted': 4,
      'wrongWordTestTarget': 0,
      'wrongWordTestCompleted': 0,
      'rootAffixTarget': 2,
      'rootAffixCompleted': 0,
    };

    expect(todayCompletionForTest(snapshot, plan), 86);
  });

  test('task rows cap displayed completion at the displayed target', () {
    final snapshot = <String, dynamic>{
      'newWordsTarget': 20,
      'newWordsCompleted': 1,
      'reviewWordsTarget': 12,
      'reviewWordsCompleted': 3,
      'mixedTestTarget': 3,
      'mixedTestCompleted': 4,
      'wrongWordTestTarget': 3,
      'wrongWordTestCompleted': 0,
      'rootAffixTarget': 2,
      'rootAffixCompleted': 0,
    };

    expect(
      todayTaskBreakdownRowsForTest(snapshot, plan),
      contains('mixedTest:3/3'),
    );
  });

  test('review row remains visible from today plan when review pool target is zero', () {
    final snapshot = <String, dynamic>{
      'newWordsTarget': 0,
      'newWordsCompleted': 0,
      'reviewWordsTarget': 0,
      'reviewWordsCompleted': 0,
      'mixedTestTarget': 0,
      'mixedTestCompleted': 0,
      'wrongWordTestTarget': 0,
      'wrongWordTestCompleted': 0,
      'rootAffixTarget': 0,
      'rootAffixCompleted': 0,
    };

    expect(todayTaskBreakdownModesForTest(snapshot, plan), contains('review'));
    expect(todayTaskBreakdownRowsForTest(snapshot, plan), contains('review:0/0'));
  });
}

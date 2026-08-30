import 'package:flutter_mobile/features/plan_screen.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('wrong-word plan target accepts values above thirty', () {
    expect(planWrongWordTargetMax, greaterThan(30));
    expect(parseWrongWordPlanTargetForTest('35', 30), 35);
    expect(
      parseWrongWordPlanTargetForTest('${planWrongWordTargetMax + 1}', 30),
      planWrongWordTargetMax,
    );
  });
}

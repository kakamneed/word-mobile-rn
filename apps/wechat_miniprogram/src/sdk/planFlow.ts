import { PlanSummary } from './types';

export type PlanInput = ReturnType<typeof buildPlanInput>;

export function clampInt(value: number, min: number, max: number) {
  const rounded = Math.round(Number.isFinite(value) ? value : min);
  return Math.max(min, Math.min(max, rounded));
}

export function roundDownToMultipleOfFour(value: number) {
  const clamped = Math.max(0, Math.round(value));
  return clamped - (clamped % 4);
}

export function buildPlanInput(
  plan: PlanSummary,
  draft: Partial<
    Pick<
      PlanSummary,
      | 'name'
      | 'newWordsPerDay'
      | 'reviewWordsPerDay'
      | 'mixedTestPerDay'
      | 'wrongWordTestPerDay'
      | 'rootAffixPerDay'
      | 'growthRuleEnabled'
      | 'growthRuleMode'
      | 'growthIntervalDays'
      | 'growthIncrement'
      | 'sharedGrowthRule'
      | 'growthRulesByMode'
      | 'questionTypeWeightsByMode'
    >
  >,
) {
  const growthIntervalDays = clampInt(draft.growthIntervalDays ?? plan.growthIntervalDays, 1, 365);
  const growthIncrement = clampInt(draft.growthIncrement ?? plan.growthIncrement, 0, 100);
  return {
    name: (draft.name ?? plan.name).trim() || plan.name,
    newWordsPerDay: roundDownToMultipleOfFour(
      clampInt(draft.newWordsPerDay ?? plan.newWordsPerDay, 0, 180),
    ),
    reviewWordsPerDay: clampInt(draft.reviewWordsPerDay ?? plan.reviewWordsPerDay, 0, 200),
    mixedTestPerDay: clampInt(draft.mixedTestPerDay ?? plan.mixedTestPerDay, 0, 50),
    wrongWordTestPerDay: clampInt(
      draft.wrongWordTestPerDay ?? plan.wrongWordTestPerDay,
      0,
      30,
    ),
    rootAffixPerDay: clampInt(draft.rootAffixPerDay ?? plan.rootAffixPerDay ?? 0, 0, 50),
    growthRuleEnabled: draft.growthRuleEnabled ?? plan.growthRuleEnabled,
    growthRuleMode: draft.growthRuleMode ?? plan.growthRuleMode,
    growthIntervalDays,
    growthIncrement,
    sharedGrowthRule: draft.sharedGrowthRule ?? { intervalDays: growthIntervalDays, increment: growthIncrement },
    growthRulesByMode: draft.growthRulesByMode ?? plan.growthRulesByMode ?? {},
    questionTypeWeightsByMode:
      draft.questionTypeWeightsByMode ?? plan.questionTypeWeightsByMode ?? {},
  };
}

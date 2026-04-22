import {
  getActivePlan as bridgeGetActivePlan,
  savePlan as bridgeSavePlan,
  applySavedPlanToToday as bridgeApplySavedPlanToToday,
  type GrowthModeKey,
  type GrowthRule,
  type PlanEditInput,
  type PlanSummary,
} from './mobile-bridge';

export type {PlanSummary, PlanEditInput};

const GROWTH_MODE_KEYS: GrowthModeKey[] = [
  'newWord',
  'review',
  'mixedTest',
  'wrongWordReinforcement',
  'rootAffix',
];

export function normalizeGrowthRule(rule?: Partial<GrowthRule> | null): GrowthRule {
  return {
    intervalDays: Math.max(1, Math.min(365, rule?.intervalDays ?? 7)),
    increment: Math.max(0, Math.min(100, rule?.increment ?? 5)),
  };
}

export function normalizePlanGrowth(plan: PlanSummary): PlanSummary {
  const sharedGrowthRule = normalizeGrowthRule(
    plan.sharedGrowthRule ?? {
      intervalDays: plan.growthIntervalDays,
      increment: plan.growthIncrement,
    },
  );
  const growthRulesByMode = Object.fromEntries(
    GROWTH_MODE_KEYS.map(mode => [
      mode,
      normalizeGrowthRule(plan.growthRulesByMode?.[mode] ?? sharedGrowthRule),
    ]),
  ) as Record<GrowthModeKey, GrowthRule>;

  return {
    ...plan,
    growthRuleMode: plan.growthRuleMode ?? 'shared',
    growthIntervalDays: sharedGrowthRule.intervalDays,
    growthIncrement: sharedGrowthRule.increment,
    sharedGrowthRule,
    growthRulesByMode,
  };
}

export async function fetchActivePlan(): Promise<PlanSummary | null> {
  const plan = await bridgeGetActivePlan();
  return plan ? normalizePlanGrowth(plan) : null;
}

export async function savePlan(
  planId: number,
  input: PlanEditInput,
): Promise<PlanSummary> {
  const saved = await bridgeSavePlan(planId, input);
  return normalizePlanGrowth(saved);
}

export async function applyPlanToToday(): Promise<PlanSummary> {
  const applied = await bridgeApplySavedPlanToToday();
  return normalizePlanGrowth(applied);
}

export function validatePlanInput(input: PlanEditInput): string | null {
  if (
    input.newWordsPerDay !== undefined &&
    (input.newWordsPerDay < 0 || input.newWordsPerDay > 100)
  ) {
    return '每日新词数量需要在 0 到 100 之间';
  }

  if (
    input.reviewWordsPerDay !== undefined &&
    (input.reviewWordsPerDay < 0 || input.reviewWordsPerDay > 200)
  ) {
    return '每日复习数量需要在 0 到 200 之间';
  }

  if (
    input.mixedTestPerDay !== undefined &&
    (input.mixedTestPerDay < 0 || input.mixedTestPerDay > 50)
  ) {
    return '每日混合测试数量需要在 0 到 50 之间';
  }

  if (
    input.wrongWordTestPerDay !== undefined &&
    (input.wrongWordTestPerDay < 0 || input.wrongWordTestPerDay > 30)
  ) {
    return '每日错词强化数量需要在 0 到 30 之间';
  }

  if (
    input.rootAffixPerDay !== undefined &&
    (input.rootAffixPerDay < 0 || input.rootAffixPerDay > 50)
  ) {
    return '每日词根词缀数量需要在 0 到 50 之间';
  }

  if (
    input.growthIntervalDays !== undefined &&
    (input.growthIntervalDays < 1 || input.growthIntervalDays > 365)
  ) {
    return '增长周期需要在 1 到 365 天之间';
  }

  if (
    input.growthIncrement !== undefined &&
    (input.growthIncrement < 0 || input.growthIncrement > 100)
  ) {
    return '增长幅度需要在 0 到 100 之间';
  }

  if (input.sharedGrowthRule) {
    const shared = normalizeGrowthRule(input.sharedGrowthRule);
    if (shared.intervalDays < 1 || shared.intervalDays > 365) {
      return '共享增长周期需要在 1 到 365 天之间';
    }
    if (shared.increment < 0 || shared.increment > 100) {
      return '共享增长幅度需要在 0 到 100 之间';
    }
  }

  if (input.growthRulesByMode) {
    for (const mode of Object.keys(input.growthRulesByMode) as GrowthModeKey[]) {
      const rule = normalizeGrowthRule(input.growthRulesByMode[mode]);
      if (rule.intervalDays < 1 || rule.intervalDays > 365) {
        return `${mode} 的增长周期需要在 1 到 365 天之间`;
      }
      if (rule.increment < 0 || rule.increment > 100) {
        return `${mode} 的增长幅度需要在 0 到 100 之间`;
      }
    }
  }

  return null;
}

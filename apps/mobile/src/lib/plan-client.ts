/**
 * Plan Client
 */

import {
  getActivePlan as bridgeGetActivePlan,
  savePlan as bridgeSavePlan,
  applySavedPlanToToday as bridgeApplySavedPlanToToday,
  type PlanSummary,
  type PlanEditInput,
} from './mobile-bridge';

export type {PlanSummary, PlanEditInput};

export async function fetchActivePlan(): Promise<PlanSummary | null> {
  return bridgeGetActivePlan();
}

export async function savePlan(
  planId: number,
  input: PlanEditInput,
): Promise<PlanSummary> {
  return bridgeSavePlan(planId, input);
}

export async function applyPlanToToday(): Promise<PlanSummary> {
  return bridgeApplySavedPlanToToday();
}

export function validatePlanInput(input: PlanEditInput): string | null {
  if (
    input.newWordsPerDay !== undefined &&
    (input.newWordsPerDay < 5 || input.newWordsPerDay > 100)
  ) {
    return '每日新词数量需要在 5 到 100 之间';
  }

  if (
    input.reviewWordsPerDay !== undefined &&
    (input.reviewWordsPerDay < 10 || input.reviewWordsPerDay > 200)
  ) {
    return '每日复习数量需要在 10 到 200 之间';
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

  return null;
}

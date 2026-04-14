import {getTodayHomeState as bridgeGetTodayHomeState} from './mobile-bridge';
import type {
  TodayHomeState,
  DailySnapshot,
  PlanSummary,
  WordbookSummary,
  DailyProgress,
  SessionMode,
} from './mobile-bridge';

export type {
  TodayHomeState,
  DailySnapshot,
  PlanSummary,
  WordbookSummary,
  DailyProgress,
};

export async function fetchToday(): Promise<TodayHomeState> {
  return bridgeGetTodayHomeState();
}

export function calculateCompletion(snapshot: DailySnapshot): number {
  const total =
    snapshot.newWordsTarget +
    snapshot.reviewWordsTarget +
    snapshot.mixedTestTarget +
    snapshot.wrongWordTestTarget +
    (snapshot.rootAffixTarget ?? 0);

  const completed =
    snapshot.newWordsCompleted +
    snapshot.reviewWordsCompleted +
    snapshot.mixedTestCompleted +
    snapshot.wrongWordTestCompleted +
    (snapshot.rootAffixCompleted ?? 0);

  return total > 0 ? Math.round((completed / total) * 100) : 0;
}

export function getPrimaryAction(snapshot: DailySnapshot | null): {
  label: string;
  count: number;
  type: SessionMode | 'done';
} {
  if (!snapshot) {
    return {label: '开始今日学习', count: 0, type: 'newWord'};
  }

  if (snapshot.newWordsCompleted < snapshot.newWordsTarget) {
    return {
      label: '学习新词',
      count: snapshot.newWordsTarget - snapshot.newWordsCompleted,
      type: 'newWord',
    };
  }

  if (snapshot.reviewWordsCompleted < snapshot.reviewWordsTarget) {
    return {
      label: '继续复习',
      count: snapshot.reviewWordsTarget - snapshot.reviewWordsCompleted,
      type: 'review',
    };
  }

  if (snapshot.mixedTestCompleted < snapshot.mixedTestTarget) {
    return {
      label: '开始混合测试',
      count: snapshot.mixedTestTarget - snapshot.mixedTestCompleted,
      type: 'mixedTest',
    };
  }

  if (snapshot.wrongWordTestCompleted < snapshot.wrongWordTestTarget) {
    return {
      label: '进行错词强化',
      count: snapshot.wrongWordTestTarget - snapshot.wrongWordTestCompleted,
      type: 'wrongWordReinforcement',
    };
  }

  if ((snapshot.rootAffixCompleted ?? 0) < (snapshot.rootAffixTarget ?? 0)) {
    return {
      label: '学习词根词缀',
      count: (snapshot.rootAffixTarget ?? 0) - (snapshot.rootAffixCompleted ?? 0),
      type: 'rootAffix',
    };
  }

  return {label: '今日已完成', count: 0, type: 'done'};
}

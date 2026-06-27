import { PlanSummary, TodayHomeState, TodayRewardState } from './types';
import { scopedStorageKey } from './storageNamespace';

const todayStateKey = 'word_miniprogram_today_state';
const rewardStateKey = 'word_miniprogram_reward_state';

export const defaultPlanForToday: PlanSummary = {
  id: 1,
  name: '鳄甲卫',
  newWordsPerDay: 20,
  reviewWordsPerDay: 28,
  mixedTestPerDay: 34,
  wrongWordTestPerDay: 26,
  rootAffixPerDay: 4,
  growthRuleEnabled: true,
  growthRuleMode: 'shared',
  growthIntervalDays: 7,
  growthIncrement: 5,
};

export const defaultTodayState: TodayHomeState = {
  todayDate: '2026-05-21',
  activePlan: defaultPlanForToday,
  wordbooks: [
    { id: 3, code: 'kaoyan', name: 'KaoYan', category: 'exam', totalEntries: 5500, isActive: true },
  ],
  dailyProgress: {
    completedTasks: 0,
    totalTasks: 5,
    accuracyPercent: 0,
    streakDays: 0,
  },
  modeProgress: {},
  resumeHint: { hasResume: false },
};

export const defaultRewardState: TodayRewardState = {
  todayDate: '2026-05-21',
  rewardId: undefined,
  claimedAt: undefined,
  canClaim: false,
  asset: undefined,
};

declare const wx:
  | {
      getStorageSync?: <T = unknown>(key: string) => T;
      setStorageSync?: (key: string, value: unknown) => void;
    }
  | undefined;

let memoryTodayState: TodayHomeState | undefined;
let memoryRewardState: TodayRewardState | undefined;

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function numberOrDefault(value: unknown, fallback: number) {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback;
}

function normalizePlan(value: unknown): PlanSummary {
  const source = isRecord(value) ? value : {};
  return {
    ...defaultPlanForToday,
    name: typeof source.name === 'string' && source.name ? source.name : defaultPlanForToday.name,
    id: numberOrDefault(source.id, defaultPlanForToday.id),
    newWordsPerDay: numberOrDefault(source.newWordsPerDay, defaultPlanForToday.newWordsPerDay),
    reviewWordsPerDay: numberOrDefault(source.reviewWordsPerDay, defaultPlanForToday.reviewWordsPerDay),
    mixedTestPerDay: numberOrDefault(source.mixedTestPerDay, defaultPlanForToday.mixedTestPerDay),
    wrongWordTestPerDay: numberOrDefault(source.wrongWordTestPerDay, defaultPlanForToday.wrongWordTestPerDay),
    rootAffixPerDay: numberOrDefault(source.rootAffixPerDay, defaultPlanForToday.rootAffixPerDay ?? 4),
    growthRuleEnabled: typeof source.growthRuleEnabled === 'boolean'
      ? source.growthRuleEnabled
      : defaultPlanForToday.growthRuleEnabled,
    growthRuleMode: typeof source.growthRuleMode === 'string'
      ? source.growthRuleMode
      : defaultPlanForToday.growthRuleMode,
    growthIntervalDays: numberOrDefault(source.growthIntervalDays, defaultPlanForToday.growthIntervalDays),
    growthIncrement: numberOrDefault(source.growthIncrement, defaultPlanForToday.growthIncrement),
    questionTypeWeightsByMode: isRecord(source.questionTypeWeightsByMode)
      ? source.questionTypeWeightsByMode as PlanSummary['questionTypeWeightsByMode']
      : defaultPlanForToday.questionTypeWeightsByMode,
  };
}

function normalizeTodayState(value: unknown): TodayHomeState {
  const source = isRecord(value) ? value : {};
  const progress = isRecord(source.dailyProgress) ? source.dailyProgress : {};
  const fallback = defaultTodayState;

  return {
    ...fallback,
    ...source,
    activePlan: normalizePlan(source.activePlan ?? fallback.activePlan),
    wordbooks: Array.isArray(source.wordbooks) ? source.wordbooks as TodayHomeState['wordbooks'] : fallback.wordbooks,
    dailyProgress: {
      completedTasks: numberOrDefault(progress.completedTasks, fallback.dailyProgress.completedTasks),
      totalTasks: numberOrDefault(progress.totalTasks, fallback.dailyProgress.totalTasks),
      accuracyPercent: numberOrDefault(progress.accuracyPercent, fallback.dailyProgress.accuracyPercent),
      streakDays: numberOrDefault(progress.streakDays, fallback.dailyProgress.streakDays),
    },
    modeProgress: isRecord(source.modeProgress)
      ? source.modeProgress as TodayHomeState['modeProgress']
      : fallback.modeProgress,
    resumeHint: isRecord(source.resumeHint)
      ? { ...fallback.resumeHint, ...source.resumeHint } as TodayHomeState['resumeHint']
      : fallback.resumeHint,
  };
}

function normalizeRewardState(value: unknown): TodayRewardState {
  const source = isRecord(value) ? value : {};
  const asset = isRecord(source.asset)
    ? source.asset as TodayRewardState['asset']
    : undefined;
  return {
    ...defaultRewardState,
    ...source,
    canClaim: typeof source.canClaim === 'boolean' ? source.canClaim : false,
    asset,
  };
}

export function getStoredTodayState(): TodayHomeState {
  const stored = typeof wx !== 'undefined' && wx.getStorageSync
    ? wx.getStorageSync<unknown>(scopedStorageKey(todayStateKey))
    : undefined;
  const normalized = normalizeTodayState(stored ?? memoryTodayState ?? defaultTodayState);
  saveStoredTodayState(normalized);
  return clone(normalized);
}

export function saveStoredTodayState(today: TodayHomeState) {
  memoryTodayState = normalizeTodayState(today);
  if (typeof wx !== 'undefined' && wx.setStorageSync) {
    wx.setStorageSync(scopedStorageKey(todayStateKey), memoryTodayState);
  }
}

export function getStoredRewardState(): TodayRewardState {
  const stored = typeof wx !== 'undefined' && wx.getStorageSync
    ? wx.getStorageSync<unknown>(scopedStorageKey(rewardStateKey))
    : undefined;
  const normalized = normalizeRewardState(stored ?? memoryRewardState ?? defaultRewardState);
  saveStoredRewardState(normalized);
  return clone(normalized);
}

export function saveStoredRewardState(reward: TodayRewardState) {
  memoryRewardState = normalizeRewardState(reward);
  if (typeof wx !== 'undefined' && wx.setStorageSync) {
    wx.setStorageSync(scopedStorageKey(rewardStateKey), memoryRewardState);
  }
}

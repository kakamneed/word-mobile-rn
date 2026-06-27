import {
  defaultTodayState,
  getStoredTodayState,
  saveStoredTodayState,
} from '../src/sdk/todayStore';
import { restoreActiveStorageNamespace } from '../src/sdk/storageNamespace';

type WxStorage = {
  storage: Record<string, unknown>;
  getStorageSync<T = unknown>(key: string): T;
  setStorageSync(key: string, value: unknown): void;
};

const namespace = 'wx_test_user';
const todayKey = `word_miniprogram:${namespace}:word_miniprogram_today_state`;

const wxStorage: WxStorage = {
  storage: {
    word_miniprogram_active_storage_namespace: namespace,
    [todayKey]: {
      todayDate: '2026-05-23',
      activePlan: { name: '旧缓存计划' },
      resumeHint: { hasResume: true, mode: 'newWord', current: 3, total: 20 },
    },
  },
  getStorageSync<T = unknown>(key: string): T {
    return this.storage[key] as T;
  },
  setStorageSync(key: string, value: unknown) {
    this.storage[key] = value;
  },
};

(globalThis as typeof globalThis & { wx: WxStorage }).wx = wxStorage;
restoreActiveStorageNamespace();

const migrated = getStoredTodayState();

if (migrated.dailyProgress.totalTasks !== defaultTodayState.dailyProgress.totalTasks) {
  throw new Error('Expected missing dailyProgress.totalTasks to be restored from defaults');
}

if (migrated.dailyProgress.completedTasks !== 0) {
  throw new Error('Expected missing completedTasks to be restored from defaults');
}

if (migrated.activePlan?.name !== '旧缓存计划') {
  throw new Error('Expected valid stored plan fields to be preserved during migration');
}

if (migrated.activePlan?.newWordsPerDay !== defaultTodayState.activePlan?.newWordsPerDay) {
  throw new Error('Expected missing activePlan fields to be restored from defaults');
}

if (!migrated.resumeHint?.hasResume || migrated.resumeHint.current !== 3) {
  throw new Error('Expected resume hint to survive migration');
}

const written = wxStorage.storage[todayKey] as typeof migrated;
if (!written.dailyProgress || written.dailyProgress.totalTasks !== defaultTodayState.dailyProgress.totalTasks) {
  throw new Error('Expected migrated Today state to be written back to namespaced storage');
}

saveStoredTodayState({ ...migrated, dailyProgress: undefined as never });
const saved = wxStorage.storage[todayKey] as typeof migrated;
if (!saved.dailyProgress || saved.dailyProgress.totalTasks !== defaultTodayState.dailyProgress.totalTasks) {
  throw new Error('Expected saveStoredTodayState to normalize invalid callers too');
}

console.log(JSON.stringify({
  totalTasks: migrated.dailyProgress.totalTasks,
  planName: migrated.activePlan?.name,
  resumeCurrent: migrated.resumeHint?.current,
  storageKey: todayKey,
}, null, 2));

import { scopedStorageKey } from './storageNamespace';
import { ReportsOverview, WrongWordEntry } from './types';

const reportsCacheKey = 'cloud_reports_overview';
const wrongWordsCacheKey = 'cloud_wrong_words';

declare const wx:
  | {
      getStorageSync?: <T = unknown>(key: string) => T;
      setStorageSync?: (key: string, value: unknown) => void;
    }
  | undefined;

function readScoped<T>(key: string): T | undefined {
  try {
    return wx?.getStorageSync?.<T>(scopedStorageKey(key)) || undefined;
  } catch {
    return undefined;
  }
}

function writeScoped<T>(key: string, value: T) {
  try {
    wx?.setStorageSync?.(scopedStorageKey(key), value);
  } catch {
    // Cache misses are safe; the network response remains authoritative.
  }
}

export function getCachedReportsOverview() {
  return readScoped<ReportsOverview>(reportsCacheKey);
}

export function saveCachedReportsOverview(value: ReportsOverview) {
  writeScoped(reportsCacheKey, value);
}

export function getCachedWrongWords() {
  return readScoped<WrongWordEntry[]>(wrongWordsCacheKey);
}

export function saveCachedWrongWords(value: WrongWordEntry[]) {
  writeScoped(wrongWordsCacheKey, value);
}

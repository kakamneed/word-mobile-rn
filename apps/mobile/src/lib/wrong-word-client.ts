import {
  getWrongWordDetail as bridgeGetWrongWordDetail,
  getWrongWords as bridgeGetWrongWords,
  type WrongWordDetail,
  type WrongWordEntry,
} from './mobile-bridge';

export type {WrongWordDetail, WrongWordEntry};

export type WrongWordFilter = 'all' | 'highPriority' | 'recent' | 'frequent';

export async function fetchWrongWords(
  filter: WrongWordFilter,
): Promise<WrongWordEntry[]> {
  return bridgeGetWrongWords(filter);
}

export async function fetchWrongWordDetail(
  entryId: number,
): Promise<WrongWordDetail> {
  return bridgeGetWrongWordDetail(entryId);
}

export async function removeWrongWord(entryId: number): Promise<void> {
  void entryId;
}

export async function startWrongWordReview(count: number): Promise<void> {
  void count;
}

import { scopedStorageKey } from './storageNamespace';
import { StoredStudySessionSnapshot } from './types';

const accessTokenKey = 'word_miniprogram_access_token';
const refreshTokenKey = 'word_miniprogram_refresh_token';
const activeStudySessionKey = 'word_miniprogram_active_study_session';

let memoryAccessToken = '';
let memoryRefreshToken = '';
let memoryActiveStudySession: StoredStudySessionSnapshot | undefined;

declare const wx:
  | {
      setStorageSync?: (key: string, data: unknown) => void;
      getStorageSync?: <T = unknown>(key: string) => T;
      removeStorageSync?: (key: string) => void;
    }
  | undefined;

export function saveSessionTokens(input: {
  accessToken: string;
  refreshToken: string;
}) {
  memoryAccessToken = input.accessToken;
  memoryRefreshToken = input.refreshToken;
}

export function getAccessToken() {
  return memoryAccessToken;
}

export function getRefreshToken() {
  return memoryRefreshToken;
}

export function clearSessionTokens() {
  memoryAccessToken = '';
  memoryRefreshToken = '';
}

export function saveActiveStudySession(snapshot: StoredStudySessionSnapshot) {
  memoryActiveStudySession = snapshot;
  if (typeof wx !== 'undefined' && wx.setStorageSync) {
    wx.setStorageSync(scopedStorageKey(activeStudySessionKey), snapshot);
  }
}

export function getActiveStudySession() {
  if (typeof wx !== 'undefined' && wx.getStorageSync) {
    const snapshot = wx.getStorageSync<StoredStudySessionSnapshot>(
      scopedStorageKey(activeStudySessionKey),
    );
    return snapshot || memoryActiveStudySession;
  }
  return memoryActiveStudySession;
}

export function clearActiveStudySession() {
  memoryActiveStudySession = undefined;
  if (typeof wx !== 'undefined' && wx.removeStorageSync) {
    wx.removeStorageSync(scopedStorageKey(activeStudySessionKey));
  }
}

export const sessionStorageKeys = {
  accessTokenKey,
  refreshTokenKey,
  activeStudySessionKey,
};

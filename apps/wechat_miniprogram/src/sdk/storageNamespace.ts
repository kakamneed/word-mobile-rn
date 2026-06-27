import { AuthSessionState } from './types';

const namespaceStorageKey = 'word_miniprogram_active_storage_namespace';
const pendingNamespace = 'pending_wechat_login';
export const storageNamespaceUpdatedEvent = 'storage:namespace-updated';

let activeNamespace = pendingNamespace;

declare const wx:
  | {
      setStorageSync?: (key: string, value: unknown) => void;
      getStorageSync?: <T = unknown>(key: string) => T;
    }
  | undefined;

declare const Taro:
  | {
      eventCenter?: {
        trigger?: (eventName: string, payload?: unknown) => void;
      };
    }
  | undefined;

function sanitizeNamespace(value: string) {
  return value.replace(/[^a-zA-Z0-9_-]/g, '_').slice(0, 80) || pendingNamespace;
}

export function scopedStorageKey(key: string) {
  return `word_miniprogram:${activeNamespace}:${key}`;
}

export function getActiveStorageNamespace() {
  return activeNamespace;
}

export function setActiveStorageNamespaceFromAuth(auth?: AuthSessionState | null) {
  activeNamespace = sanitizeNamespace(auth?.user.internalUserId || pendingNamespace);
  try {
    wx?.setStorageSync?.(namespaceStorageKey, activeNamespace);
  } catch {
    // Memory namespace is still updated for the current runtime.
  }
  try {
    Taro?.eventCenter?.trigger?.(storageNamespaceUpdatedEvent, activeNamespace);
  } catch {
    // The event is best-effort; storage isolation still works without it.
  }
  return activeNamespace;
}

export function restoreActiveStorageNamespace() {
  try {
    const stored = wx?.getStorageSync?.<string>(namespaceStorageKey);
    activeNamespace = sanitizeNamespace(stored || pendingNamespace);
  } catch {
    activeNamespace = pendingNamespace;
  }
  return activeNamespace;
}

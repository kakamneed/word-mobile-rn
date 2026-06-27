import Taro from '@tarojs/taro';

import { isHttpSdk, sdk } from '@/sdk';
import { mockAuthState } from '@/sdk/mockData';
import { setActiveStorageNamespaceFromAuth } from '@/sdk/storageNamespace';
import { AuthSessionState } from '@/sdk/types';

export function displayNameFromAuth(auth?: AuthSessionState | null) {
  if (!auth) return '账号';
  const id = auth.user.internalUserId;
  if (id.startsWith('wx_')) return `微信用户 ${id.slice(3, 9)}`;
  return id || '账号';
}

export function avatarTextFromAuth(auth?: AuthSessionState | null) {
  if (!auth) return '账';
  if (auth.user.primaryIdentity === 'wechat_mp') return '微';
  return displayNameFromAuth(auth).slice(0, 1).toUpperCase();
}

export function phaseLabel(auth?: AuthSessionState | null) {
  if (!auth) return isHttpSdk() ? '未连接' : '本地演示';
  const labels: Record<string, string> = {
    guest_local_only: '本地访客',
    signed_in_active: '已登录',
    merge_decision_required: '需要合并确认',
  };
  return labels[auth.accountState.phase] ?? auth.accountState.phase;
}

export async function loadAuthState() {
  const state = !isHttpSdk() ? mockAuthState : await sdk.auth.getMe();
  setActiveStorageNamespaceFromAuth(state);
  return state;
}

export async function loginWithWechat() {
  if (!isHttpSdk() || !('loginWithWechatCode' in sdk.auth)) {
    setActiveStorageNamespaceFromAuth(mockAuthState);
    return mockAuthState;
  }
  const result = await Taro.login();
  if (!result.code) throw new Error('微信登录没有返回 code');
  const state = await sdk.auth.loginWithWechatCode(result.code);
  setActiveStorageNamespaceFromAuth(state);
  return state;
}

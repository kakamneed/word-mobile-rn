import { HttpMiniProgramSdk } from './httpClient';
import { MiniProgramSdk } from './client';
import {
  restoreActiveStorageNamespace,
  setActiveStorageNamespaceFromAuth,
} from './storageNamespace';

const apiBaseUrl = process.env.TARO_APP_API_BASE_URL;

function normalizeApiBaseUrl(value: string) {
  const trimmed = value.trim().replace(/\/+$/, '');
  if (/\/rest\/v1$/i.test(trimmed)) {
    throw new Error('TARO_APP_API_BASE_URL must point to the API gateway or Supabase Functions root, not /rest/v1');
  }
  return trimmed;
}

function createSdk() {
  if (process.env.TARO_APP_SDK_MODE === 'mock') {
    console.warn('Mini Program SDK running in explicit mock mode');
    return new MiniProgramSdk();
  }

  if (process.env.TARO_APP_SDK_MODE === 'http') {
    if (typeof apiBaseUrl !== 'string' || !/^https?:\/\//.test(apiBaseUrl)) {
      throw new Error('TARO_APP_API_BASE_URL is required when TARO_APP_SDK_MODE=http');
    }
    return new HttpMiniProgramSdk(normalizeApiBaseUrl(apiBaseUrl));
  }

  throw new Error('Set TARO_APP_SDK_MODE to http or mock');
}

export const sdk = createSdk();
restoreActiveStorageNamespace();

export type AppSdk = typeof sdk;

export function isHttpSdk() {
  return sdk instanceof HttpMiniProgramSdk;
}

export function bootstrapWechatLogin() {
  if (sdk instanceof HttpMiniProgramSdk) {
    return sdk
      .bootstrapWechatLogin()
      .then((state) => {
        setActiveStorageNamespaceFromAuth(state);
        return state;
      })
      .catch((error) => {
        console.error('WeChat login bootstrap failed', error);
        throw error;
      });
  }

  return sdk.auth.getMe().then((state) => {
    setActiveStorageNamespaceFromAuth(state);
    return state;
  });
}

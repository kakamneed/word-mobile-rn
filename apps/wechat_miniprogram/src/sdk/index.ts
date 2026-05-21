import { HttpMiniProgramSdk } from './httpClient';
import { MiniProgramSdk } from './client';

const sdkMode = process.env.TARO_APP_SDK_MODE;
const apiBaseUrl = process.env.TARO_APP_API_BASE_URL;

function createSdk() {
  if (process.env.TARO_APP_SDK_MODE === 'mock') {
    console.warn('Mini Program SDK running in explicit mock mode');
    return new MiniProgramSdk();
  }

  if (process.env.TARO_APP_SDK_MODE === 'http') {
    if (typeof apiBaseUrl !== 'string' || !/^https?:\/\//.test(apiBaseUrl)) {
      throw new Error('TARO_APP_API_BASE_URL is required when TARO_APP_SDK_MODE=http');
    }
    return new HttpMiniProgramSdk(apiBaseUrl);
  }

  throw new Error('Set TARO_APP_SDK_MODE to http or mock');
}

export const sdk = createSdk();

export type AppSdk = typeof sdk;

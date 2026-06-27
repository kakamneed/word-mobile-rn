import path from 'path';
import { UserConfigExport } from '@tarojs/cli';

const sdkMode = process.env.TARO_APP_SDK_MODE === 'http' ? 'http' : 'mock';
const defaultSupabaseFunctionsBaseUrl =
  'https://pmevdtgogsudnfgxjgiz.supabase.co/functions/v1';
const apiBaseUrl =
  process.env.TARO_APP_API_BASE_URL ??
  (sdkMode === 'http' ? defaultSupabaseFunctionsBaseUrl : '');

const config: UserConfigExport = {
  projectName: 'word-mobile-wechat-miniprogram',
  date: '2026-05-20',
  designWidth: 375,
  deviceRatio: {
    640: 2.34 / 2,
    750: 1,
    828: 1.81 / 2,
    375: 2,
  },
  sourceRoot: 'src',
  outputRoot: 'dist',
  framework: 'react',
  compiler: 'webpack5',
  env: {
    TARO_APP_SDK_MODE: JSON.stringify(sdkMode),
    TARO_APP_API_BASE_URL: JSON.stringify(apiBaseUrl),
  },
  alias: {
    '@': path.resolve(__dirname, '..', 'src'),
  },
  copy: {
    patterns: [
      {
        from: 'src/assets/crocodile_refresh',
        to: 'dist/assets/crocodile_refresh',
      },
      {
        from: 'src/assets/croc_bti_compressed',
        to: 'dist/assets/croc_bti_compressed',
      },
    ],
    options: {},
  },
  mini: {},
  h5: {},
};

export default config;

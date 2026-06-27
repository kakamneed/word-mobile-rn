import Taro from '@tarojs/taro';

export type ThemeKey = 'purple' | 'green' | 'blue';

export interface ThemeStyle {
  key: ThemeKey;
  label: string;
  caption: string;
  primary: string;
  hero: string;
  soft: string;
}

export const themeStyles: ThemeStyle[] = [
  {
    key: 'purple',
    label: '鳄甲紫',
    caption: '接近 Flutter 当前默认主色',
    primary: '#6b5aa0',
    hero: '#68579a',
    soft: '#eee6ff',
  },
  {
    key: 'green',
    label: '沼泽绿',
    caption: '更接近旧版小程序绿色卡片',
    primary: '#2f7d65',
    hero: '#2f7d65',
    soft: '#e8f4ef',
  },
  {
    key: 'blue',
    label: '湖面蓝',
    caption: '低饱和、偏学习工具风格',
    primary: '#3b83a5',
    hero: '#3b83a5',
    soft: '#e7f1f6',
  },
];

const storageKey = 'app_color_style';

export function themeFromKey(value?: string | null) {
  return themeStyles.find((style) => style.key === value) ?? themeStyles[0];
}

export function getStoredTheme() {
  try {
    return themeFromKey(Taro.getStorageSync<string>(storageKey));
  } catch {
    return themeStyles[0];
  }
}

export function saveStoredTheme(key: ThemeKey) {
  Taro.setStorageSync(storageKey, key);
  return themeFromKey(key);
}

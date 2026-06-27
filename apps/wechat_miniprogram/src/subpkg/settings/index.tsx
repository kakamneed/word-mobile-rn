import { CSSProperties, useEffect, useState } from 'react';
import { Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { Screen } from '@/components/Screen';
import { getStoredTheme, saveStoredTheme, ThemeKey, themeStyles } from '@/sdk/themeStore';

import './index.scss';

export default function SettingsPage() {
  const [selected, setSelected] = useState<ThemeKey>(getStoredTheme().key);

  useEffect(() => {
    setSelected(getStoredTheme().key);
  }, []);

  function selectTheme(key: ThemeKey) {
    const theme = saveStoredTheme(key);
    setSelected(theme.key);
    Taro.showToast({ title: '主题已保存', icon: 'success' });
  }

  const currentTheme = themeStyles.find((style) => style.key === selected) ?? themeStyles[0];

  return (
    <Screen title="设置" activeTab="today" showTabBar={false} themeOverride={currentTheme}>
      <View className="settings-card">
        <Text className="settings-card__title">主题和偏好</Text>
        <Text className="settings-card__caption">
          对应 Flutter 设置页里的颜色风格。选择后会保存到本机，并立即影响当前页面。
        </Text>
      </View>

      <View className="settings-list">
        {themeStyles.map((style) => (
          <View
            key={style.key}
            className={`settings-row ${selected === style.key ? 'settings-row--active' : ''}`}
            style={
              selected === style.key
                ? ({
                    '--flutter-primary': style.primary,
                    '--flutter-soft': style.soft,
                  } as CSSProperties)
                : undefined
            }
            onClick={() => selectTheme(style.key)}
          >
            <View className="settings-row__color" style={{ background: style.primary }} />
            <View className="settings-row__copy">
              <Text className="settings-row__title">{style.label}</Text>
              <Text className="settings-row__caption">{style.caption}</Text>
            </View>
            {selected === style.key && <Text className="settings-row__check">✓</Text>}
          </View>
        ))}
      </View>
    </Screen>
  );
}

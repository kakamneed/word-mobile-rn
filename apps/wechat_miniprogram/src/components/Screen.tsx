import { CSSProperties, ReactNode, useEffect, useState } from 'react';
import { Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import {
  avatarTextFromAuth,
  displayNameFromAuth,
  loadAuthState,
  loginWithWechat,
  phaseLabel,
} from '@/sdk/authState';
import { AuthSessionState } from '@/sdk/types';
import { getStoredTheme, ThemeStyle } from '@/sdk/themeStore';

import './Screen.scss';

export type MainTab = 'today' | 'plan' | 'wrong' | 'reports';

const tabs: Array<{ key: MainTab; label: string; icon: string; url: string }> = [
  { key: 'today', label: '今日', icon: 'today', url: '/pages/today/index' },
  { key: 'plan', label: '计划', icon: 'tune', url: '/subpkg/plan/index' },
  { key: 'wrong', label: '错词', icon: 'menu_book', url: '/subpkg/wrong-words/index' },
  { key: 'reports', label: '报告', icon: 'query_stats', url: '/subpkg/reports/index' },
];

const drawerItems = [
  {
    icon: 'emoji_objects',
    title: '新手引导',
    caption: '重新设置词数并回顾功能',
    url: '/subpkg/onboarding/index',
  },
  {
    icon: 'psychology',
    title: '鳄 bti 学习人格',
    caption: '测试适合你的新词、复习、混测和错题权重',
    url: '/subpkg/croc-bti/index',
  },
  {
    icon: 'account_circle',
    title: '个人信息',
    caption: '昵称、头像和账号绑定',
    url: '/subpkg/profile/index',
  },
  {
    icon: 'leaderboard',
    title: '排行榜',
    caption: '查看周榜、月榜、总榜和正确率排行',
    url: '/subpkg/leaderboard/index',
  },
  {
    icon: 'settings',
    title: '设置',
    caption: '主题和偏好',
    url: '/subpkg/settings/index',
  },
  {
    icon: 'system_update_alt',
    title: '账号',
    caption: '查看登录状态和绑定信息',
    url: '/subpkg/account/index',
  },
];

export function AppIcon({ name, className = '' }: { name: string; className?: string }) {
  return <Text className={`app-icon app-icon--${name} ${className}`} />;
}

function navigate(url: string) {
  Taro.navigateTo({ url }).catch(() => {
    Taro.redirectTo({ url }).catch(() => undefined);
  });
}

function navigateMain(url: string) {
  Taro.redirectTo({ url }).catch(() => {
    Taro.reLaunch({ url }).catch(() => undefined);
  });
}

function BottomNav({ activeTab }: { activeTab: MainTab }) {
  return (
    <View className="bottom-nav">
      {tabs.map((tab) => (
        <View
          key={tab.key}
          className={`bottom-nav__item ${activeTab === tab.key ? 'bottom-nav__item--active' : ''}`}
          onClick={() => {
            if (activeTab !== tab.key) navigateMain(tab.url);
          }}
        >
          <View className="bottom-nav__pill">
            <AppIcon name={tab.icon} className="bottom-nav__icon" />
          </View>
          <Text className="bottom-nav__label">{tab.label}</Text>
        </View>
      ))}
    </View>
  );
}

function AccountDrawer({
  auth,
  onClose,
  onLogin,
}: {
  auth: AuthSessionState | null;
  onClose: () => void;
  onLogin: () => void;
}) {
  return (
    <View className="account-drawer" catchMove>
      <View className="account-drawer__scrim" onClick={onClose} catchMove />
      <View className="account-drawer__panel" catchMove>
        <View className="account-drawer__profile">
          <View className="account-drawer__avatar">{avatarTextFromAuth(auth)}</View>
          <Text className="account-drawer__name">{displayNameFromAuth(auth)}</Text>
          <Text className="account-drawer__state">{phaseLabel(auth)}</Text>
          <View className="account-drawer__login" onClick={onLogin}>
            <Text>{auth ? '刷新微信登录' : '微信登录'}</Text>
          </View>
        </View>
        <View className="account-drawer__list">
          {drawerItems.map((item) => (
            <View
              key={item.title}
              className="account-drawer__item"
              onClick={() => {
                onClose();
                navigate(item.url);
              }}
            >
              <View className="account-drawer__icon">
                <AppIcon name={item.icon} />
              </View>
              <View className="account-drawer__copy">
                <Text className="account-drawer__title">{item.title}</Text>
                <Text className="account-drawer__caption">{item.caption}</Text>
              </View>
            </View>
          ))}
        </View>
        <View className="account-drawer__logout">
          <View className="account-drawer__icon">
            <AppIcon name="logout" />
          </View>
          <View className="account-drawer__copy">
            <Text className="account-drawer__title">退出登录</Text>
            <Text className="account-drawer__caption">保留本机学习数据</Text>
          </View>
        </View>
      </View>
    </View>
  );
}

export function Screen({
  title = '',
  activeTab = 'today',
  children,
  showTabBar = true,
  themeOverride,
}: {
  title?: string;
  activeTab?: MainTab;
  children: ReactNode;
  showTabBar?: boolean;
  themeOverride?: ThemeStyle;
}) {
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [storedTheme, setStoredTheme] = useState(getStoredTheme);
  const [auth, setAuth] = useState<AuthSessionState | null>(null);

  useEffect(() => {
    setStoredTheme(getStoredTheme());
    loadAuthState().then(setAuth).catch(() => setAuth(null));

    const onSessionUpdated = (state: AuthSessionState) => setAuth(state);
    Taro.eventCenter.on('auth:session-updated', onSessionUpdated);
    return () => {
      Taro.eventCenter.off('auth:session-updated', onSessionUpdated);
    };
  }, []);

  async function handleLogin() {
    try {
      const state = await loginWithWechat();
      setAuth(state);
      Taro.showToast({ title: '微信登录成功', icon: 'success' });
    } catch (error) {
      Taro.showToast({
        title: error instanceof Error ? error.message.slice(0, 14) : '登录失败',
        icon: 'none',
      });
    }
  }

  const showBack = !showTabBar;
  const theme = themeOverride ?? storedTheme;

  return (
    <View
      className={`screen ${drawerOpen ? 'screen--drawer-open' : ''}`}
      style={
        {
          '--flutter-primary': theme.primary,
          '--flutter-hero-purple': theme.hero,
          '--flutter-soft': theme.soft,
        } as CSSProperties
      }
    >
      <View className="screen__header">
        <View className="screen__title-wrap">
          {showBack && (
            <View className="screen__back" onClick={() => Taro.navigateBack()}>
              <Text>{'<'}</Text>
            </View>
          )}
          <Text className="screen__title">{title}</Text>
        </View>
        <View className="screen__avatar" onClick={() => setDrawerOpen(true)}>
          {avatarTextFromAuth(auth)}
        </View>
      </View>
      <View className="screen__body">{children}</View>
      {showTabBar && <BottomNav activeTab={activeTab} />}
      {drawerOpen && (
        <AccountDrawer
          auth={auth}
          onClose={() => setDrawerOpen(false)}
          onLogin={handleLogin}
        />
      )}
    </View>
  );
}

export function Section({
  title,
  action,
  children,
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <View className="section">
      <View className="section__header">
        <Text className="section__title">{title}</Text>
        {action}
      </View>
      {children}
    </View>
  );
}

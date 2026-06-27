import { useEffect, useState } from 'react';
import { Input, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { PrimaryButton } from '@/components/PrimaryButton';
import { Screen, Section } from '@/components/Screen';
import { isHttpSdk, sdk } from '@/sdk';
import { loginWithWechat } from '@/sdk/authState';
import { mockAuthState } from '@/sdk/mockData';
import { AuthSessionState } from '@/sdk/types';

import './index.scss';

const phaseLabels: Record<string, string> = {
  guest_local_only: '本地访客',
  signed_in_active: '已登录',
  merge_decision_required: '需要合并确认',
};

function formatUserId(value: string) {
  if (!value) return '未知账号';
  if (value.startsWith('wx_')) return `微信账号 ${value.slice(3, 11)}`;
  return value;
}

export default function AccountPage() {
  const [auth, setAuth] = useState<AuthSessionState | null>(isHttpSdk() ? null : mockAuthState);
  const [errorMessage, setErrorMessage] = useState<string>();
  const [email, setEmail] = useState('');
  const [bindMessage, setBindMessage] = useState<string>();

  useEffect(() => {
    let mounted = true;

    const loadAuth = () => {
      sdk.auth
        .getMe()
        .then((state) => {
          if (!mounted) return;
          setAuth(state);
          setErrorMessage(undefined);
        })
        .catch((error) => {
          if (!mounted) return;
          setErrorMessage(error instanceof Error ? error.message : String(error));
          if (!isHttpSdk()) setAuth(mockAuthState);
        });
    };

    const onSessionUpdated = (state: AuthSessionState) => {
      if (!mounted) return;
      setAuth(state);
      setErrorMessage(undefined);
    };

    Taro.eventCenter.on('auth:session-updated', onSessionUpdated);
    loadAuth();

    return () => {
      mounted = false;
      Taro.eventCenter.off('auth:session-updated', onSessionUpdated);
    };
  }, []);

  async function handleWechatLogin() {
    try {
      const state = await loginWithWechat();
      setAuth(state);
      setErrorMessage(undefined);
      Taro.showToast({ title: '微信登录成功', icon: 'success' });
    } catch (error) {
      Taro.showToast({
        title: error instanceof Error ? error.message.slice(0, 14) : '登录失败',
        icon: 'none',
      });
    }
  }

  async function handleStartEmailBind() {
    try {
      const normalized = email.trim();
      if (!normalized) {
        Taro.showToast({ title: '先输入邮箱', icon: 'none' });
        return;
      }
      const result = await sdk.auth.startEmailBind(normalized);
      setBindMessage(result.message);
      Taro.showModal({
        title: result.requiresConfirmation ? '找到邮箱账号' : '邮箱绑定',
        content: result.message,
        showCancel: false,
      });
    } catch (error) {
      Taro.showToast({
        title: error instanceof Error ? error.message.slice(0, 14) : '绑定失败',
        icon: 'none',
      });
    }
  }

  if (errorMessage) {
    return (
      <Screen title="账号" activeTab="today">
        <View className="account-error">
          <Text className="account-error__title">账号加载失败</Text>
          <Text className="account-error__message">{errorMessage}</Text>
        </View>
      </Screen>
    );
  }

  if (!auth) {
    return (
      <Screen title="账号" activeTab="today">
        <View className="account-error account-error--loading">
          <Text className="account-error__title">正在连接微信账号</Text>
          <Text className="account-error__message">正在通过 Supabase 后端换取登录状态。</Text>
        </View>
      </Screen>
    );
  }

  return (
    <Screen title="账号" activeTab="today">
      <View className="drawer-header">
        <View className="drawer-header__avatar">
          <Text>{auth.user.primaryIdentity === 'wechat_mp' ? '微' : '邮'}</Text>
        </View>
        <Text className="drawer-header__label">账号</Text>
        <Text className="account-card__id">{formatUserId(auth.user.internalUserId)}</Text>
        <Text className="account-card__state">
          {phaseLabels[auth.accountState.phase] ?? auth.accountState.phase}
        </Text>
      </View>

      <Section title="绑定状态">
        <View className="binding-row">
          <Text>微信</Text>
          <Text className="binding-row__status">
            {auth.user.hasWechatBinding ? '已绑定' : '未绑定'}
          </Text>
        </View>
        <View className="binding-row">
          <Text>邮箱</Text>
          <Text className="binding-row__status">
            {auth.user.hasEmailBinding ? '已绑定' : '未绑定'}
          </Text>
        </View>
      </Section>

      <Section title="侧边栏功能">
        <View className="drawer-list">
          <View className="drawer-tile" onClick={() => Taro.navigateTo({ url: '/subpkg/croc-bti/index' })}>
            <Text className="drawer-tile__icon">BTI</Text>
            <View>
              <Text className="drawer-tile__title">鳄 bti 学习人格</Text>
              <Text className="drawer-tile__caption">调整新词、复习、混测和错题权重。</Text>
            </View>
          </View>
          <View className="drawer-tile" onClick={() => Taro.navigateTo({ url: '/subpkg/leaderboard/index' })}>
            <Text className="drawer-tile__icon">榜</Text>
            <View>
              <Text className="drawer-tile__title">排行榜</Text>
              <Text className="drawer-tile__caption">查看周榜、月榜、总榜和正确率排名。</Text>
            </View>
          </View>
          <View className="drawer-tile" onClick={() => Taro.navigateTo({ url: '/subpkg/plan/index' })}>
            <Text className="drawer-tile__icon">计</Text>
            <View>
              <Text className="drawer-tile__title">计划</Text>
              <Text className="drawer-tile__caption">调整当前学习计划和每日负载。</Text>
            </View>
          </View>
        </View>
      </Section>

      <Section title="个性化">
        <View className="action-card">
          <View>
            <Text className="action-card__title">个人资料与偏好</Text>
            <Text className="action-card__caption">头像、昵称、主题和本机设置。</Text>
          </View>
        </View>
      </Section>

      <Section title="绑定邮箱账号">
        <View className="action-card account-email-bind">
          <View className="account-email-bind__copy">
            <Text className="action-card__title">测试建议使用专用测试邮箱</Text>
            <Text className="action-card__caption">
              不建议用真实主账号邮箱反复测试，避免后续合并映射污染真实数据。
            </Text>
          </View>
          <Input
            className="account-email-bind__input"
            type="text"
            value={email}
            placeholder="yourname+mp-test@example.com"
            onInput={(event) => setEmail(String(event.detail.value))}
          />
          {bindMessage && <Text className="account-email-bind__message">{bindMessage}</Text>}
        </View>
      </Section>

      <PrimaryButton
        onClick={handleWechatLogin}
      >
        {auth.user.hasWechatBinding ? '刷新微信登录' : '微信登录'}
      </PrimaryButton>

      <PrimaryButton
        variant="secondary"
        onClick={handleStartEmailBind}
      >
        绑定邮箱账号
      </PrimaryButton>
    </Screen>
  );
}

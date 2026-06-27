import { useEffect, useState } from 'react';
import { Button, Input, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { Screen } from '@/components/Screen';
import {
  avatarTextFromAuth,
  displayNameFromAuth,
  loadAuthState,
  loginWithWechat,
} from '@/sdk/authState';
import { sdk } from '@/sdk';
import { scopedStorageKey } from '@/sdk/storageNamespace';
import { AuthSessionState } from '@/sdk/types';

import './index.scss';

const avatarColors = ['#a7f3d0', '#bfdbfe', '#fde68a', '#fbcfe8', '#c4b5fd', '#fed7aa'];
const profileStorageKey = 'word_miniprogram_profile';

interface StoredProfile {
  displayName?: string;
  avatarIndex?: number;
}

function readStoredProfile(): StoredProfile {
  try {
    return Taro.getStorageSync(scopedStorageKey(profileStorageKey)) || {};
  } catch {
    return {};
  }
}

export default function ProfilePage() {
  const [auth, setAuth] = useState<AuthSessionState | null>(null);
  const [displayName, setDisplayName] = useState('');
  const [avatarIndex, setAvatarIndex] = useState(0);
  const [email, setEmail] = useState('');
  const [bindMessage, setBindMessage] = useState('');

  useEffect(() => {
    const stored = readStoredProfile();
    if (typeof stored.avatarIndex === 'number') setAvatarIndex(stored.avatarIndex);

    loadAuthState()
      .then((state) => {
        setAuth(state);
        setDisplayName(stored.displayName || displayNameFromAuth(state));
      })
      .catch(() => {
        setDisplayName(stored.displayName || '未登录用户');
      });

    const onSessionUpdated = (state: AuthSessionState) => {
      setAuth(state);
      if (!readStoredProfile().displayName) setDisplayName(displayNameFromAuth(state));
    };
    Taro.eventCenter.on('auth:session-updated', onSessionUpdated);
    return () => {
      Taro.eventCenter.off('auth:session-updated', onSessionUpdated);
    };
  }, []);

  async function refreshWechatLogin() {
    try {
      const state = await loginWithWechat();
      setAuth(state);
      if (!readStoredProfile().displayName) setDisplayName(displayNameFromAuth(state));
      Taro.showToast({ title: '微信登录成功', icon: 'success' });
    } catch (error) {
      Taro.showToast({
        title: error instanceof Error ? error.message.slice(0, 14) : '登录失败',
        icon: 'none',
      });
    }
  }

  async function bindEmail() {
    const normalized = email.trim();
    if (!normalized.includes('@')) {
      Taro.showToast({ title: '请输入邮箱', icon: 'none' });
      return;
    }

    try {
      const result = await sdk.auth.startEmailBind(normalized);
      const message = result.message || (result.requiresConfirmation ? '已提交绑定请求' : '邮箱账号已绑定');
      setBindMessage(message);
      await loadAuthState().then(setAuth).catch(() => undefined);
      Taro.showModal({
        title: '邮箱绑定',
        content: message,
        showCancel: false,
      });
    } catch (error) {
      setBindMessage(error instanceof Error ? error.message : '绑定失败');
      Taro.showToast({
        title: error instanceof Error ? error.message.slice(0, 14) : '绑定失败',
        icon: 'none',
      });
    }
  }

  function saveProfile() {
    Taro.setStorageSync(scopedStorageKey(profileStorageKey), {
      displayName: displayName.trim(),
      avatarIndex,
    });
    Taro.showToast({ title: '个人信息已保存', icon: 'success' });
  }

  const name = displayName.trim() || displayNameFromAuth(auth);

  return (
    <Screen title="个人信息" activeTab="today" showTabBar={false}>
      <View className="profile-preview">
        <View className="profile-avatar" style={{ background: avatarColors[avatarIndex] }}>
          <Text>{name.slice(0, 1).toUpperCase() || avatarTextFromAuth(auth)}</Text>
        </View>
        <View className="profile-preview__copy">
          <Text className="profile-preview__name">{name}</Text>
          <Text className="profile-preview__caption">
            昵称和头像会用于抽屉、排行榜和个人展示。
          </Text>
          <Text className="profile-preview__state">
            {auth ? `已连接：${displayNameFromAuth(auth)}` : '尚未连接微信账号'}
          </Text>
        </View>
      </View>

      <View className="profile-card">
        <Text className="profile-card__title">微信账号</Text>
        <Text className="profile-card__caption">
          用当前小程序身份刷新登录状态，成功后会同步到侧边栏和账号页。
        </Text>
        <Button className="profile-login" onClick={refreshWechatLogin}>
          {auth ? '刷新微信登录' : '微信登录'}
        </Button>
      </View>

      <View className="profile-card">
        <Text className="profile-card__title">绑定邮箱账号</Text>
        <Text className="profile-card__caption">
          测试阶段建议使用专门的测试邮箱。绑定后会用该邮箱对应的 Flutter 云端学习数据作为读取来源。
        </Text>
        <Input
          className="profile-input"
          value={email}
          placeholder="输入 Flutter 账号邮箱"
          onInput={(event) => setEmail(String(event.detail.value))}
        />
        {bindMessage && <Text className="profile-bind-message">{bindMessage}</Text>}
        <Button className="profile-login" onClick={bindEmail}>
          绑定邮箱账号
        </Button>
      </View>

      <View className="profile-card">
        <Text className="profile-card__title">昵称</Text>
        <Text className="profile-card__caption">输入你想展示在排行榜上的名字。</Text>
        <Input
          className="profile-input"
          maxlength={24}
          value={displayName}
          onInput={(event) => setDisplayName(String(event.detail.value))}
        />
      </View>

      <View className="profile-card">
        <Text className="profile-card__title">头像颜色</Text>
        <Text className="profile-card__caption">
          小程序首版先使用本地颜色头像，后续再接入相册头像。
        </Text>
        <View className="avatar-color-row">
          {avatarColors.map((color, index) => (
            <View
              key={color}
              className={`avatar-color ${index === avatarIndex ? 'avatar-color--active' : ''}`}
              style={{ background: color }}
              onClick={() => setAvatarIndex(index)}
            >
              {index === avatarIndex && <Text>✓</Text>}
            </View>
          ))}
        </View>
      </View>

      <Button className="profile-save" onClick={saveProfile}>
        保存
      </Button>
    </Screen>
  );
}

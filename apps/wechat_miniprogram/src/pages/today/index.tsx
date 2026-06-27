import { useCallback, useEffect, useState } from 'react';
import { Image, Text, View } from '@tarojs/components';
import Taro, { useDidShow, usePullDownRefresh } from '@tarojs/taro';

import { CrocodileRefreshPopup } from '@/components/CrocodileFrameAnimation';
import { Screen } from '@/components/Screen';
import { sdk } from '@/sdk';
import {
  defaultRewardState,
  defaultTodayState,
  getStoredRewardState,
  getStoredTodayState,
  saveStoredRewardState,
  saveStoredTodayState,
} from '@/sdk/todayStore';
import { storageNamespaceUpdatedEvent } from '@/sdk/storageNamespace';
import { StudyMode, TodayHomeState, TodayRewardState } from '@/sdk/types';

import './index.scss';

const taskRows: Array<{
  mode: StudyMode;
  tone: 'new' | 'review' | 'mixed' | 'wrong' | 'root';
  title: string;
  subtitle: string;
  countKey: 'newWordsPerDay' | 'reviewWordsPerDay' | 'mixedTestPerDay' | 'wrongWordTestPerDay' | 'rootAffixPerDay';
}> = [
  { mode: 'newWord', tone: 'new', title: '新词学习', subtitle: '建立今日新词基础', countKey: 'newWordsPerDay' },
  { mode: 'review', tone: 'review', title: '复习', subtitle: '回顾旧词，巩固记忆', countKey: 'reviewWordsPerDay' },
  { mode: 'mixedTest', tone: 'mixed', title: '混合测试', subtitle: '综合检验今日状态', countKey: 'mixedTestPerDay' },
  { mode: 'wrongWordReinforcement', tone: 'wrong', title: '错词强化', subtitle: '回收今天的薄弱点', countKey: 'wrongWordTestPerDay' },
  { mode: 'rootAffix', tone: 'root', title: '词根词缀', subtitle: '词根、前缀和后缀题', countKey: 'rootAffixPerDay' },
];

export function calculateTodayCompletion(today: TodayHomeState) {
  const progress = today.dailyProgress ?? defaultTodayState.dailyProgress;
  return progress.totalTasks
    ? Math.min(100, Math.round((progress.completedTasks / progress.totalTasks) * 100))
    : 0;
}

function taskProgress(today: TodayHomeState, task: (typeof taskRows)[number]) {
  const plan = today.activePlan ?? defaultTodayState.activePlan;
  const total = Number(today.modeProgress?.[task.mode]?.total ?? plan?.[task.countKey] ?? 0);
  const completed = Number(today.modeProgress?.[task.mode]?.completed ?? 0);
  return {
    completed: Math.min(completed, total),
    total,
    percent: total ? Math.min(100, Math.round((completed / total) * 100)) : 0,
  };
}

function studyUrlForMode(today: TodayHomeState, mode: StudyMode) {
  const hint = today.resumeHint;
  if (hint?.hasResume && hint.mode === mode) {
    return '/pages/study/index?resume=1';
  }
  return `/pages/study/index?mode=${mode}`;
}

export default function TodayPage() {
  const [today, setToday] = useState<TodayHomeState>(defaultTodayState);
  const [reward, setReward] = useState<TodayRewardState>(defaultRewardState);
  const [refreshing, setRefreshing] = useState(false);
  const [claimingReward, setClaimingReward] = useState(false);

  const refreshToday = useCallback(async (showIndicator = false) => {
    if (showIndicator) setRefreshing(true);
    try {
      const cachedToday = getStoredTodayState();
      const cachedReward = getStoredRewardState();
      setToday(cachedToday);
      setReward(cachedReward);
      const [cloudToday, cloudReward] = await Promise.all([
        sdk.today.getTodayHomeState(),
        sdk.rewards.getTodayRewardState().catch(() => cachedReward),
      ]);
      saveStoredTodayState(cloudToday);
      saveStoredRewardState(cloudReward);
      setToday(cloudToday);
      setReward(cloudReward);
      if (showIndicator) {
        await new Promise((resolve) => setTimeout(resolve, 720));
      }
    } catch (error) {
      if (process.env.TARO_APP_SDK_MODE === 'http') throw error;
      setToday(defaultTodayState);
      setReward(defaultRewardState);
    } finally {
      if (showIndicator) setRefreshing(false);
    }
  }, []);

  useEffect(() => {
    refreshToday();
    const refreshForNamespace = () => refreshToday();
    Taro.eventCenter.on(storageNamespaceUpdatedEvent, refreshForNamespace);
    Taro.eventCenter.on('auth:session-updated', refreshForNamespace);
    return () => {
      Taro.eventCenter.off(storageNamespaceUpdatedEvent, refreshForNamespace);
      Taro.eventCenter.off('auth:session-updated', refreshForNamespace);
    };
  }, [refreshToday]);

  useDidShow(() => {
    refreshToday();
  });

  usePullDownRefresh(() => {
    refreshToday(true).finally(() => Taro.stopPullDownRefresh());
  });

  const completion = calculateTodayCompletion(today);
  const dailyProgress = today.dailyProgress ?? defaultTodayState.dailyProgress;
  const remaining = Math.max(0, dailyProgress.totalTasks - dailyProgress.completedTasks);
  const hasReward = Boolean(reward.rewardId);
  const canClaimReward = Boolean(reward.canClaim && !hasReward && !claimingReward);
  const rewardTitle = reward.asset?.title ?? reward.rewardId ?? '今日奖励';

  const claimReward = async () => {
    if (!canClaimReward) return;
    setClaimingReward(true);
    try {
      const claimed = await sdk.rewards.claimTodayReward();
      saveStoredRewardState(claimed);
      setReward(claimed);
      Taro.showToast({ title: '已领取奖励', icon: 'success' });
    } catch (error) {
      const message = error instanceof Error ? error.message : '领取失败';
      Taro.showToast({ title: message.slice(0, 14), icon: 'none' });
    } finally {
      setClaimingReward(false);
    }
  };

  return (
    <Screen title="今日" activeTab="today">
      <View className={`today-refresh-overlay ${refreshing ? 'today-refresh-overlay--active' : ''}`}>
        <CrocodileRefreshPopup active={refreshing} label="刷新今日进度" />
      </View>

      <View className="today-hero">
        <Text className="today-hero__date">{today.todayDate}</Text>
        <Text className="today-hero__title">学习新词</Text>
        <Text className="today-hero__subtitle">先把今天的新词任务推进起来。</Text>
        <View className="today-hero__rail">
          <View className="today-hero__fill" style={{ width: `${completion}%` }} />
        </View>
        <Text className="today-hero__meta">今日完成度 {completion}%</Text>
        <Text className="today-hero__meta">剩余 {remaining} 个任务单位</Text>
        <View className="today-hero__button" onClick={() => Taro.navigateTo({ url: studyUrlForMode(today, 'newWord') })}>
          <Text>进入新词学习</Text>
        </View>
      </View>

      <View className="flutter-card task-breakdown">
        <View className="task-breakdown__header">
          <Text className="flutter-card__title">今日任务拆解</Text>
          <View className="task-breakdown__edit" onClick={() => Taro.navigateTo({ url: '/subpkg/plan/index' })}>
            <Text className="task-breakdown__edit-icon">✎</Text>
            <Text>修改</Text>
          </View>
        </View>

        {taskRows.map((task) => {
          const progress = taskProgress(today, task);
          return (
            <View className="task-line" key={task.mode} onClick={() => Taro.navigateTo({ url: studyUrlForMode(today, task.mode) })}>
              <View className={`task-line__dot task-line__dot--${task.tone}`} />
              <View className="task-line__body">
                <View className="task-line__top">
                  <View>
                    <Text className="task-line__title">{task.title}</Text>
                    <Text className="task-line__subtitle">{task.subtitle}</Text>
                  </View>
                  <View className="task-line__count">
                    <Text>{progress.completed}/{progress.total}</Text>
                    <Text className={`task-line__chevron task-line__chevron--${task.tone}`}>›</Text>
                  </View>
                </View>
                <View className="task-line__rail">
                  <View className={`task-line__fill task-line__fill--${task.tone}`} style={{ width: `${progress.percent}%` }} />
                </View>
              </View>
            </View>
          );
        })}
      </View>

      <View className="flutter-card reward-card">
        <Text className="flutter-card__title">今日奖励</Text>
        <Text className="flutter-card__subtitle">
          {hasReward ? '今日奖励已领取。' : reward.canClaim ? '完成任务，拉杆领取今日奖励。' : '完成所有启用任务后领取奖励。'}
        </Text>
        <View className="reward-card__stage">
          <View className={`reward-machine ${canClaimReward ? 'reward-machine--ready' : ''}`} onClick={claimReward}>
            <View className="reward-machine__knob" />
            <View className="reward-machine__body">
              <View className="reward-machine__slot" />
              <View className="reward-machine__window" />
              <View className="reward-machine__feet"><View /><View /></View>
            </View>
          </View>
          <View className={`reward-lock ${hasReward ? 'reward-lock--claimed' : ''}`}>
            {hasReward && reward.asset?.imageUrl ? (
              <Image className="reward-lock__image" mode="aspectFill" src={reward.asset.imageUrl} />
            ) : (
              <Text className="reward-lock__icon">{reward.canClaim ? '!' : '锁'}</Text>
            )}
            <Text className="reward-lock__caption">{hasReward ? rewardTitle : reward.canClaim ? '点击拉杆领取' : '完成后解锁'}</Text>
          </View>
        </View>
        <View className={`reward-card__button ${canClaimReward ? 'reward-card__button--enabled' : ''}`} onClick={claimReward}>
          <Text>{hasReward ? '今日已领取' : claimingReward ? '领取中...' : reward.canClaim ? '领取今日奖励' : '任务未完成'}</Text>
        </View>
      </View>

      {today.resumeHint?.hasResume && (
        <View className="resume-strip" onClick={() => Taro.navigateTo({ url: '/pages/study/index?resume=1' })}>
          <Text className="resume-strip__title">继续上次学习</Text>
          <Text className="resume-strip__text">{today.resumeHint.word} {today.resumeHint.current}/{today.resumeHint.total}</Text>
        </View>
      )}
    </Screen>
  );
}

import { useEffect, useState } from 'react';
import { Button, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { Screen } from '../../components/Screen';
import { buildPlanInput, clampInt } from '../../sdk/planFlow';
import { mockPlan } from '../../sdk/mockData';
import { sdk } from '../../sdk/client';
import { PlanSummary } from '../../sdk/types';

import './index.scss';

type CountKey = 'newWordsPerDay' | 'reviewWordsPerDay' | 'mixedTestPerDay' | 'wrongWordTestPerDay';

const countRows: Array<{ key: CountKey; title: string; caption: string; min: number; max: number }> = [
  { key: 'newWordsPerDay', title: '新词学习', caption: '建立今日新词基础', min: 0, max: 180 },
  { key: 'reviewWordsPerDay', title: '复习', caption: '回顾旧词，巩固记忆', min: 0, max: 200 },
  { key: 'mixedTestPerDay', title: '混合测试', caption: '综合检验今日状态', min: 0, max: 50 },
  { key: 'wrongWordTestPerDay', title: '错词强化', caption: '回收今天的薄弱点', min: 0, max: 30 },
];

export default function OnboardingPage() {
  const [step, setStep] = useState(0);
  const [plan, setPlan] = useState<PlanSummary>(mockPlan);
  const [draft, setDraft] = useState<Partial<PlanSummary>>({});
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    sdk.plan.getActivePlan().then(setPlan).catch(() => undefined);
  }, []);

  const planInput = buildPlanInput(plan, draft);

  function countValue(key: CountKey) {
    return Number(draft[key] ?? plan[key] ?? 0);
  }

  function updateCount(key: CountKey, delta: number, min: number, max: number) {
    setDraft((current) => ({
      ...current,
      [key]: clampInt(Number(current[key] ?? plan[key] ?? 0) + delta, min, max),
    }));
  }

  async function saveAndApply() {
    setSaving(true);
    try {
      await sdk.plan.savePlan({ planId: plan.id, input: planInput });
      await sdk.plan.applySavedPlanToToday();
      Taro.showToast({ title: '已完成引导', icon: 'success' });
      Taro.redirectTo({ url: '/pages/today/index?refresh=1' });
    } finally {
      setSaving(false);
    }
  }

  return (
    <Screen title="新手引导" activeTab="today" showTabBar={false}>
      <View className="onboarding-hero">
        <Text className="onboarding-hero__eyebrow">重新回顾</Text>
        <Text className="onboarding-hero__title">把今日学习节奏重新校准一下。</Text>
        <Text className="onboarding-hero__caption">
          这里对应 Flutter 抽屉里的新手引导入口，用于回顾主功能并重新设置每日题量。
        </Text>
      </View>

      <View className="onboarding-progress">
        {[0, 1, 2].map((item) => (
          <View
            key={item}
            className={`onboarding-progress__dot ${step >= item ? 'onboarding-progress__dot--active' : ''}`}
          />
        ))}
      </View>

      {step === 0 && (
        <View className="onboarding-card">
          <Text className="onboarding-card__title">功能回顾</Text>
          <Text className="onboarding-card__body">今日页负责承接计划进度，学习页负责逐题作答，错词本会沉淀薄弱点，报告页展示趋势。</Text>
          <Text className="onboarding-card__body">鳄bti 可以继续微调不同模式的权重；小程序首版先保留入口，不添加 AI 功能。</Text>
        </View>
      )}

      {step === 1 && (
        <View className="onboarding-card">
          <Text className="onboarding-card__title">每日目标</Text>
          <Text className="onboarding-card__body">按 Flutter 逻辑，计划保存后再同步到今日，首页和作答页会使用同一份计划口径。</Text>
          {countRows.map((row) => (
            <View key={row.key} className="onboarding-row">
              <View>
                <Text className="onboarding-row__title">{row.title}</Text>
                <Text className="onboarding-row__caption">{row.caption}</Text>
              </View>
              <View className="onboarding-stepper">
                <Button
                  className="onboarding-stepper__button"
                  onClick={() => updateCount(row.key, -1, row.min, row.max)}
                >
                  -
                </Button>
                <Text className="onboarding-stepper__value">{countValue(row.key)}</Text>
                <Button
                  className="onboarding-stepper__button"
                  onClick={() => updateCount(row.key, 1, row.min, row.max)}
                >
                  +
                </Button>
              </View>
            </View>
          ))}
        </View>
      )}

      {step === 2 && (
        <View className="onboarding-card">
          <Text className="onboarding-card__title">准备完成</Text>
          <Text className="onboarding-card__body">
            当前计划：新词 {planInput.newWordsPerDay}，复习 {planInput.reviewWordsPerDay}，混测{' '}
            {planInput.mixedTestPerDay}，错词 {planInput.wrongWordTestPerDay}。
          </Text>
          <Text className="onboarding-card__body">保存后会立即同步到今日页。</Text>
        </View>
      )}

      <View className="onboarding-actions">
        {step > 0 && (
          <Button className="onboarding-action" onClick={() => setStep((current) => current - 1)}>
            上一步
          </Button>
        )}
        {step < 2 ? (
          <Button
            className="onboarding-action onboarding-action--primary"
            onClick={() => setStep((current) => current + 1)}
          >
            下一步
          </Button>
        ) : (
          <Button
            className="onboarding-action onboarding-action--primary"
            loading={saving}
            onClick={saveAndApply}
          >
            保存并进入今日
          </Button>
        )}
      </View>
    </Screen>
  );
}

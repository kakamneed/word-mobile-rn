import { useEffect, useMemo, useState } from 'react';
import { Button, Input, Switch, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { Screen } from '../../components/Screen';
import { buildPlanInput, clampInt } from '../../sdk/planFlow';
import { mockPlan, mockWordbooks } from '../../sdk/mockData';
import { sdk } from '../../sdk';
import { PlanSummary, StudyMode, WordbookSummary } from '../../sdk/types';

import './index.scss';

type PlanNumberKey =
  | 'newWordsPerDay'
  | 'reviewWordsPerDay'
  | 'mixedTestPerDay'
  | 'wrongWordTestPerDay'
  | 'rootAffixPerDay';

type GrowthRule = { intervalDays: number; increment: number };
type Draft = Partial<PlanSummary>;

const targetMeta: Array<{
  key: PlanNumberKey;
  title: string;
  caption: string;
  min: number;
  max: number;
}> = [
  { key: 'newWordsPerDay', title: '新词学习', caption: '单位：题；需为 4 的倍数，对应四种固定题型', min: 0, max: 180 },
  { key: 'reviewWordsPerDay', title: '复习', caption: '单位：题；按人格推荐题型占比分配', min: 0, max: 200 },
  { key: 'mixedTestPerDay', title: '混合测试', caption: '按题推进', min: 0, max: 50 },
  { key: 'wrongWordTestPerDay', title: '错词强化', caption: '按题推进', min: 0, max: 30 },
  { key: 'rootAffixPerDay', title: '词根词缀', caption: '词根、前缀和后缀题', min: 0, max: 50 },
];

const growthModes: Array<{ key: StudyMode; title: string }> = [
  { key: 'newWord', title: '新词' },
  { key: 'review', title: '复习' },
  { key: 'mixedTest', title: '混测' },
  { key: 'wrongWordReinforcement', title: '错词' },
  { key: 'rootAffix', title: '词根' },
];

function getPlanNumber(plan: PlanSummary, draft: Draft, key: PlanNumberKey) {
  return Number(draft[key] ?? plan[key] ?? 0);
}

function getSharedRule(plan: PlanSummary, draft: Draft): GrowthRule {
  const planInput = buildPlanInput(plan, draft);
  return (
    planInput.sharedGrowthRule ?? {
      intervalDays: planInput.growthIntervalDays,
      increment: planInput.growthIncrement,
    }
  );
}

function getModeRule(plan: PlanSummary, draft: Draft, mode: StudyMode): GrowthRule {
  const shared = getSharedRule(plan, draft);
  const planInput = buildPlanInput(plan, draft);
  return planInput.growthRulesByMode?.[mode] ?? shared;
}

export default function PlanPage() {
  const [plan, setPlan] = useState<PlanSummary>(mockPlan);
  const [draft, setDraft] = useState<Draft>({});
  const [wordbooks, setWordbooks] = useState<WordbookSummary[]>(mockWordbooks);
  const [pendingWordbookId, setPendingWordbookId] = useState<number | undefined>(
    mockWordbooks.find((wordbook) => wordbook.isActive)?.id,
  );
  const [saving, setSaving] = useState(false);
  const [applying, setApplying] = useState(false);

  useEffect(() => {
    let mounted = true;
    Promise.all([sdk.plan.getActivePlan(), sdk.plan.getWordbooks()])
      .then(([nextPlan, nextWordbooks]) => {
        if (!mounted) return;
        setPlan(nextPlan);
        setWordbooks(nextWordbooks);
        setPendingWordbookId(nextWordbooks.find((wordbook) => wordbook.isActive)?.id);
      })
      .catch(() => undefined);
    return () => {
      mounted = false;
    };
  }, []);

  const planInput = useMemo(() => buildPlanInput(plan, draft), [draft, plan]);
  const growthRuleMode = planInput.growthRuleMode === 'perMode' ? 'perMode' : 'shared';

  function updateNumber(key: PlanNumberKey, delta: number, min: number, max: number) {
    setDraft((current) => ({
      ...current,
      [key]: clampInt(getPlanNumber(plan, current, key) + delta, min, max),
    }));
  }

  function updateSharedRule(partial: Partial<GrowthRule>) {
    const next = { ...getSharedRule(plan, draft), ...partial };
    setDraft((current) => ({
      ...current,
      growthIntervalDays: next.intervalDays,
      growthIncrement: next.increment,
      sharedGrowthRule: next,
    }));
  }

  function updateModeRule(mode: StudyMode, partial: Partial<GrowthRule>) {
    const currentRule = getModeRule(plan, draft, mode);
    const nextRule = { ...currentRule, ...partial };
    setDraft((current) => ({
      ...current,
      growthRuleMode: 'perMode',
      growthRulesByMode: {
        ...(buildPlanInput(plan, current).growthRulesByMode ?? {}),
        [mode]: nextRule,
      },
    }));
  }

  async function persistPlanAndWordbook() {
    const nextPlan = await sdk.plan.savePlan({ planId: plan.id, input: planInput });
    setPlan(nextPlan);
    setDraft({});
    if (pendingWordbookId) {
      await sdk.plan.toggleWordbook({ wordbookId: pendingWordbookId, isActive: true });
      setWordbooks((current) =>
        current.map((wordbook) => ({
          ...wordbook,
          isActive: wordbook.id === pendingWordbookId,
        })),
      );
    }
    return nextPlan;
  }

  async function savePlan() {
    setSaving(true);
    try {
      await persistPlanAndWordbook();
      Taro.showToast({ title: '计划已保存', icon: 'success' });
    } finally {
      setSaving(false);
    }
  }

  async function applySavedPlanToToday() {
    setApplying(true);
    try {
      await persistPlanAndWordbook();
      await sdk.plan.applySavedPlanToToday();
      Taro.showToast({ title: '已同步到今日', icon: 'success' });
      Taro.redirectTo({ url: '/pages/today/index?refresh=1' });
    } finally {
      setApplying(false);
    }
  }

  return (
    <Screen title="计划" activeTab="plan">
      <View className="plan-hero">
        <Text className="plan-hero__title">{planInput.name}</Text>
        <Text className="plan-hero__caption">计划编辑、词书选择、增长规则会直接影响今日与学习页的主流程。</Text>
        <View className="plan-hero__metrics">
          <View className="plan-hero__metric">
            <Text>{planInput.newWordsPerDay}</Text>
            <Text>新词/天</Text>
          </View>
          <View className="plan-hero__metric">
            <Text>{planInput.reviewWordsPerDay}</Text>
            <Text>复习/天</Text>
          </View>
          <View className="plan-hero__metric">
            <Text>{planInput.rootAffixPerDay}</Text>
            <Text>词根</Text>
          </View>
        </View>
        <Text className="plan-hero__foot">
          已选词书：{wordbooks.find((wordbook) => wordbook.id === pendingWordbookId)?.name ?? '未选择'}
        </Text>
      </View>

      <View className="plan-card">
        <Text className="plan-card__title">计划名称</Text>
        <Text className="plan-card__caption">计划会同时影响今日、学习和报告的展示口径。</Text>
        <Input
          className="plan-input"
          maxlength={50}
          value={planInput.name}
          onInput={(event) => setDraft((current) => ({ ...current, name: String(event.detail.value) }))}
        />
        <Text className="plan-input__count">{planInput.name.length}/50</Text>
      </View>

      <View className="plan-card">
        <Text className="plan-card__title">每日目标</Text>
        <Text className="plan-card__caption">和 RN 一样，首页与学习流程统一按题量展示进度。新词、复习按每词 4 题换算。</Text>
        {targetMeta.map((target) => (
          <View key={target.key} className="target-row">
            <View className="target-row__copy">
              <Text className="target-row__title">{target.title}</Text>
              <Text className="target-row__caption">{target.caption}</Text>
            </View>
            <View className="stepper">
              <Button className="stepper__button" onClick={() => updateNumber(target.key, -1, target.min, target.max)}>
                -
              </Button>
              <Text className="stepper__value">{getPlanNumber(plan, draft, target.key)}</Text>
              <Button className="stepper__button" onClick={() => updateNumber(target.key, 1, target.min, target.max)}>
                +
              </Button>
            </View>
          </View>
        ))}
      </View>

      <View className="plan-card">
        <Text className="plan-card__title">增长规则</Text>
        <Text className="plan-card__caption">支持共享规则和分模式规则。保存后可以决定今天是否立刻采用新节奏。</Text>
        <View className="growth-toggle-row">
          <View>
            <Text className="growth-toggle-row__title">启用增长推荐</Text>
            <Text className="growth-toggle-row__caption">开启后会按间隔天数逐步增加每日计划量</Text>
          </View>
          <Switch
            checked={planInput.growthRuleEnabled}
            color="#02c755"
            onChange={(event) => setDraft((current) => ({ ...current, growthRuleEnabled: event.detail.value }))}
          />
        </View>
        <View className="segment-row">
          <View
            className={`segment ${growthRuleMode === 'shared' ? 'segment--active' : ''}`}
            onClick={() => setDraft((current) => ({ ...current, growthRuleMode: 'shared' }))}
          >
            全部共享
          </View>
          <View
            className={`segment ${growthRuleMode === 'perMode' ? 'segment--active' : ''}`}
            onClick={() => setDraft((current) => ({ ...current, growthRuleMode: 'perMode' }))}
          >
            分别设置
          </View>
        </View>

        {growthRuleMode === 'shared' ? (
          <RuleEditor title="统一增长规则" rule={getSharedRule(plan, draft)} onChange={updateSharedRule} />
        ) : (
          <View className="growth-mode-list">
            {growthModes.map((mode) => (
              <RuleEditor
                key={mode.key}
                title={`${mode.title}增长规则`}
                rule={getModeRule(plan, draft, mode.key)}
                onChange={(partial) => updateModeRule(mode.key, partial)}
              />
            ))}
          </View>
        )}
      </View>

      <View className="plan-card">
        <Text className="plan-card__title">词书管理</Text>
        <Text className="plan-card__caption">保持和 RN 一样：词书选择先保存，再决定是否把变化应用到今天。</Text>
        {wordbooks.map((wordbook) => (
          <View
            key={wordbook.id}
            className={`wordbook-row ${pendingWordbookId === wordbook.id ? 'wordbook-row--active' : ''}`}
            onClick={() => setPendingWordbookId(wordbook.id)}
          >
            <View className={`radio-dot ${pendingWordbookId === wordbook.id ? 'radio-dot--active' : ''}`} />
            <View className="wordbook-row__copy">
              <Text className="wordbook-row__title">{wordbook.name}</Text>
              <Text className="wordbook-row__caption">
                {wordbook.totalEntries} 词 · {wordbook.category}
              </Text>
            </View>
          </View>
        ))}
      </View>

      <View className="plan-actions">
        <Button className="plan-action plan-action--primary" loading={saving} onClick={savePlan}>
          保存计划
        </Button>
        <Button className="plan-action" loading={applying} onClick={applySavedPlanToToday}>
          同步到今日
        </Button>
      </View>
    </Screen>
  );
}

function RuleEditor({
  title,
  rule,
  onChange,
}: {
  title: string;
  rule: GrowthRule;
  onChange: (partial: Partial<GrowthRule>) => void;
}) {
  return (
    <View className="growth-box">
      <Text className="growth-box__title">{title}</Text>
      <View className="target-row target-row--compact">
        <View className="target-row__copy">
          <Text className="target-row__title">增长间隔</Text>
          <Text className="target-row__caption">多少天后提醒</Text>
        </View>
        <View className="stepper">
          <Button className="stepper__button" onClick={() => onChange({ intervalDays: clampInt(rule.intervalDays - 1, 1, 365) })}>
            -
          </Button>
          <Text className="stepper__value">{rule.intervalDays}</Text>
          <Button className="stepper__button" onClick={() => onChange({ intervalDays: clampInt(rule.intervalDays + 1, 1, 365) })}>
            +
          </Button>
        </View>
      </View>
      <View className="target-row target-row--compact">
        <View className="target-row__copy">
          <Text className="target-row__title">增长增量</Text>
          <Text className="target-row__caption">每次增加多少</Text>
        </View>
        <View className="stepper">
          <Button className="stepper__button" onClick={() => onChange({ increment: clampInt(rule.increment - 1, 0, 100) })}>
            -
          </Button>
          <Text className="stepper__value">{rule.increment}</Text>
          <Button className="stepper__button" onClick={() => onChange({ increment: clampInt(rule.increment + 1, 0, 100) })}>
            +
          </Button>
        </View>
      </View>
    </View>
  );
}

import { useEffect, useMemo, useState } from 'react';
import { Button, Image, Slider, Switch, Text, View } from '@tarojs/components';
import Taro from '@tarojs/taro';

import { PrimaryButton } from '@/components/PrimaryButton';
import { Screen, Section } from '@/components/Screen';
import {
  clampCrocBtiAnswer,
  crocBtiPlanInputFor,
  crocBtiPlanInputForDailyMinutes,
  crocBtiQuestions,
  evaluateCrocBti,
  hasCompleteCrocBtiAnswers,
  rebalanceCrocBtiQuestionTypeWeights,
} from '@/sdk/crocBti';
import { sdk } from '@/sdk';
import { scopedStorageKey } from '@/sdk/storageNamespace';
import { PlanSummary } from '@/sdk/types';

import './index.scss';

const ANSWERS_KEY = 'croc-bti-answers';
const MINUTES_KEY = 'croc-bti-minutes';
const QUESTIONS_VERSION_KEY = 'croc-bti-question-version';
const QUESTIONS_VERSION = `v2:${crocBtiQuestions.map((question) => question.id).join('|')}`;

function restoreCompleteAnswers(
  profileAnswers?: Record<string, number>,
  savedAnswers?: Record<string, number>,
  savedVersion = '',
) {
  const source = profileAnswers ?? (savedVersion === QUESTIONS_VERSION ? savedAnswers : undefined);
  if (!source) return {};
  return Object.fromEntries(
    crocBtiQuestions
      .map((question) => [question.id, source[question.id]] as const)
      .filter(([, value]) => Number.isFinite(value) && value >= -2 && value <= 2),
  );
}

const planLabels: Record<string, string> = {
  newWordsPerDay: '新词/天',
  reviewWordsPerDay: '复习/天',
  mixedTestPerDay: '混测/天',
  wrongWordTestPerDay: '错词/天',
  rootAffixPerDay: '词根词缀/天',
};

export default function CrocBtiPage() {
  const [answers, setAnswers] = useState<Record<string, number>>({});
  const [dailyLearningMinutes, setDailyLearningMinutes] = useState(40);
  const [showResult, setShowResult] = useState(false);
  const [plan, setPlan] = useState<PlanSummary>();
  const [editedPlanInput, setEditedPlanInput] = useState<Record<string, number>>();
  const [questionTypeWeightsByMode, setQuestionTypeWeightsByMode] = useState<Record<string, Record<string, number>>>();
  const [growthRuleEnabled, setGrowthRuleEnabled] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const load = async () => {
      const savedAnswers = Taro.getStorageSync<Record<string, number>>(scopedStorageKey(ANSWERS_KEY)) || {};
      const savedVersion = String(Taro.getStorageSync<string>(scopedStorageKey(QUESTIONS_VERSION_KEY)) || '');
      const savedMinutes = Number(Taro.getStorageSync<number>(scopedStorageKey(MINUTES_KEY)) || 40);
      const profile = await sdk.crocBti.getProfile().catch(() => null);
      const restoredAnswers = restoreCompleteAnswers(profile?.answers, savedAnswers, savedVersion);
      setAnswers(restoredAnswers);
      setDailyLearningMinutes(Math.max(10, Math.min(240, Math.round(profile?.dailyLearningMinutes ?? savedMinutes))));
      setShowResult(false);
      const activePlan = await sdk.plan.getActivePlan();
      setPlan(activePlan);
      setGrowthRuleEnabled(activePlan.growthRuleEnabled);
    };
    load().catch((error) => Taro.showToast({ title: String(error), icon: 'none' }));
  }, []);

  const result = useMemo(() => evaluateCrocBti(answers), [answers]);
  const complete = hasCompleteCrocBtiAnswers(answers);
  const planInput = editedPlanInput ?? crocBtiPlanInputForDailyMinutes(result.weights, dailyLearningMinutes);
  const weights = questionTypeWeightsByMode ?? result.questionTypeWeightsByMode;

  useEffect(() => {
    if (!showResult) return;
    setEditedPlanInput(crocBtiPlanInputForDailyMinutes(result.weights, dailyLearningMinutes));
    setQuestionTypeWeightsByMode(result.questionTypeWeightsByMode);
  }, [dailyLearningMinutes, result.questionTypeWeightsByMode, result.weights, showResult]);

  const updateAnswer = (id: string, value: number) => {
    setAnswers((current) => ({ ...current, [id]: clampCrocBtiAnswer(value) }));
  };

  const showResultOrHint = () => {
    if (!complete) {
      Taro.showToast({ title: `${Object.keys(answers).length}/${crocBtiQuestions.length} answered`, icon: 'none' });
      setShowResult(false);
      return;
    }
    Taro.setStorageSync(scopedStorageKey(ANSWERS_KEY), answers);
    Taro.setStorageSync(scopedStorageKey(MINUTES_KEY), dailyLearningMinutes);
    Taro.setStorageSync(scopedStorageKey(QUESTIONS_VERSION_KEY), QUESTIONS_VERSION);
    setShowResult(true);
  };

  const updatePlanInput = (key: string, value: number) => {
    setEditedPlanInput((current) => ({ ...(current ?? planInput), [key]: Math.max(0, Math.min(240, Math.round(value))) }));
  };

  const applyResult = async () => {
    if (!plan || !complete) return;
    setSaving(true);
    try {
      Taro.setStorageSync(scopedStorageKey(ANSWERS_KEY), answers);
      Taro.setStorageSync(scopedStorageKey(MINUTES_KEY), dailyLearningMinutes);
      Taro.setStorageSync(scopedStorageKey(QUESTIONS_VERSION_KEY), QUESTIONS_VERSION);
      const normalizedWeights = weights;
      await sdk.crocBti.saveProfile({
        profile: {
          resultCode: result.code,
          title: result.title,
          summary: result.summary,
          advice: result.advice,
          assetPath: result.assetPath,
          flavor: result.flavor,
          answers,
          axisScores: result.axisScores,
          weights: result.weights,
          planInput,
          questionTypeWeightsByMode: normalizedWeights,
          dailyLearningMinutes,
          growthRuleEnabled,
          source: 'croc_bti',
          version: Date.now(),
          evaluatedAt: new Date().toISOString(),
        },
      });
      await sdk.plan.savePlan({ planId: plan.id, input: crocBtiPlanInputFor(plan, result, { planInput, questionTypeWeightsByMode: normalizedWeights, growthRuleEnabled }) });
      await sdk.plan.applySavedPlanToToday();
      await sdk.sync.flushPendingToCloud();
      Taro.redirectTo({ url: '/pages/today/index?refresh=1' });
    } finally {
      setSaving(false);
    }
  };

  if (showResult && complete) {
    return (
      <Screen title="鳄bti" activeTab="plan">
        <View className="result-card result-card--large">
          <Image className="result-card__image" mode="aspectFill" src={`/${result.assetPath}`} />
          <Text className="result-card__code">{result.code}</Text>
          <Text className="result-card__title">{result.title}</Text>
          <Text className="result-card__caption">{result.summary}</Text>
          <Text className="result-card__caption">{result.advice}</Text>
        </View>

        <Section title="每日分钟数">
          <View className="minutes-card">
            <Text className="minutes-card__value">{dailyLearningMinutes} min</Text>
            <Slider min={10} max={120} step={5} value={dailyLearningMinutes} activeColor="#6b5aa0" onChange={(event) => setDailyLearningMinutes(event.detail.value)} />
          </View>
        </Section>

        <Section title="计划题量">
          <View className="tuning-list">
            {Object.entries(planInput).map(([key, value]) => (
              <View className="tuning-row" key={key}>
                <Text className="tuning-row__title">{planLabels[key] ?? key}</Text>
                <View className="tuning-row__controls">
                  <Button onClick={() => updatePlanInput(key, value - 1)}>-</Button>
                  <Text>{value}</Text>
                  <Button onClick={() => updatePlanInput(key, value + 1)}>+</Button>
                </View>
              </View>
            ))}
          </View>
        </Section>

        <Section title="增长规则">
          <View className="growth-toggle-row">
            <Text>Growth rule</Text>
            <Switch checked={growthRuleEnabled} onChange={(event) => setGrowthRuleEnabled(event.detail.value)} />
          </View>
        </Section>

        <Section title="模式题型权重">
          {Object.entries(weights).map(([mode, modeWeights]) => (
            <View className="weight-card" key={mode}>
              <Text className="weight-card__title">{mode}</Text>
              {Object.entries(modeWeights).map(([questionType, value]) => (
                <View className="weight-row" key={questionType}>
                  <Text>{questionType}</Text>
                  <Slider min={0} max={60} step={5} value={value} activeColor="#6b5aa0" onChange={(event) => setQuestionTypeWeightsByMode((current) => ({ ...(current ?? weights), [mode]: rebalanceCrocBtiQuestionTypeWeights(modeWeights, questionType, event.detail.value) }))} />
                  <Text>{value}%</Text>
                </View>
              ))}
            </View>
          ))}
        </Section>

        <View className="croc-actions">
          <PrimaryButton variant="secondary" onClick={() => setShowResult(false)}>重新测试</PrimaryButton>
          <PrimaryButton disabled={saving} onClick={applyResult}>{saving ? '保存中...' : '应用结果'}</PrimaryButton>
        </View>
      </Screen>
    );
  }

  return (
    <Screen title="鳄bti" activeTab="plan">
      <View className="croc-hero">
        <Text className="croc-hero__title">Croc BTI</Text>
        <Text className="croc-hero__caption">Answer every question before viewing the result.</Text>
      </View>

      <Section title="每日分钟数">
        <View className="minutes-card">
          <Text className="minutes-card__value">{dailyLearningMinutes} min</Text>
          <Slider min={10} max={120} step={5} value={dailyLearningMinutes} activeColor="#2f6f5e" onChange={(event) => setDailyLearningMinutes(event.detail.value)} />
        </View>
      </Section>

      <Section title="问题">
        <View className="question-list">
          {crocBtiQuestions.map((question) => (
            <View className="question-card" key={question.id}>
              <Text className="question-card__text">{question.text}</Text>
              <View className="answer-row answer-row--five">
                {[-2, -1, 0, 1, 2].map((value) => (
                  <Button key={value} className={`answer-pill ${answers[question.id] === value ? 'answer-pill--selected' : ''}`} onClick={() => updateAnswer(question.id, value)}>
                    <Text>{value + 3}</Text>
                  </Button>
                ))}
              </View>
            </View>
          ))}
        </View>
      </Section>

      <PrimaryButton onClick={showResultOrHint}>{complete ? '查看结果' : `${Object.keys(answers).length}/${crocBtiQuestions.length}`}</PrimaryButton>
    </Screen>
  );
}

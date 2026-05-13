import React, {useMemo, useState} from 'react';
import {
  Alert,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import {
  CROC_BTI_QUESTIONS,
  evaluateCrocBti,
  type CrocBtiAnswer,
  type CrocBtiAnswerValue,
  type CrocBtiResult,
  type CrocBtiWeights,
} from '../../lib/croc-bti';
import {applyPlanToToday, savePlan, type PlanSummary} from '../../lib/plan-client';

interface CrocBtiScreenProps {
  plan: PlanSummary;
  onBack: () => void;
  onApplied: (plan: PlanSummary) => void;
}

const ANSWER_OPTIONS: {label: string; value: CrocBtiAnswerValue}[] = [
  {label: '很不同意', value: -2},
  {label: '不同意', value: -1},
  {label: '不确定', value: 0},
  {label: '同意', value: 1},
  {label: '很同意', value: 2},
];

const WEIGHT_LABELS: {key: keyof CrocBtiWeights; label: string}[] = [
  {key: 'newWords', label: '新词'},
  {key: 'review', label: '复习'},
  {key: 'mixedTest', label: '混测'},
  {key: 'wrongWordReview', label: '错题'},
  {key: 'contextExamples', label: '语境'},
  {key: 'activeRecall', label: '输出'},
];

export function CrocBtiScreen({
  plan,
  onBack,
  onApplied,
}: CrocBtiScreenProps): React.JSX.Element {
  const [answerMap, setAnswerMap] = useState<Record<string, CrocBtiAnswerValue>>({});
  const [showResult, setShowResult] = useState(false);
  const [saving, setSaving] = useState(false);

  const answers: CrocBtiAnswer[] = useMemo(
    () =>
      Object.entries(answerMap).map(([questionId, value]) => ({
        questionId,
        value,
      })),
    [answerMap],
  );
  const result = useMemo(() => evaluateCrocBti(answers), [answers]);
  const answeredCount = Object.keys(answerMap).length;
  const allAnswered = answeredCount === CROC_BTI_QUESTIONS.length;

  const handleApply = async () => {
    setSaving(true);
    try {
      const saved = await savePlan(plan.id, {
        name: `${plan.name} - ${result.title}`,
        ...result.planInput,
      });
      const applied = await applyPlanToToday();
      onApplied(applied ?? saved);
      Alert.alert('已应用鳄bti', `${result.title} 的学习权重已经写入今日计划。`);
    } catch (error) {
      Alert.alert(
        '应用失败',
        error instanceof Error ? error.message : '鳄bti 推荐暂时没有写入成功。',
      );
    } finally {
      setSaving(false);
    }
  };

  if (showResult) {
    return (
      <ResultView
        result={result}
        saving={saving}
        onBack={() => setShowResult(false)}
        onClose={onBack}
        onApply={() => void handleApply()}
      />
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <TouchableOpacity onPress={onBack}>
          <Text style={styles.headerButton}>返回</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>鳄bti</Text>
        <TouchableOpacity
          onPress={() => setShowResult(true)}
          disabled={!allAnswered}
          accessibilityState={{disabled: !allAnswered}}>
          <Text style={[styles.headerButton, !allAnswered && styles.disabledText]}>
            结果
          </Text>
        </TouchableOpacity>
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.intro}>
          <Text style={styles.kicker}>学习人格测试</Text>
          <Text style={styles.title}>测出你的鳄系学习方式</Text>
          <Text style={styles.description}>
            回答 24 道偏好题，系统会匹配鳄骑兵、鳄渊者等学习称号，并推荐新词、复习、混测和错题的每日权重。
          </Text>
          <View style={styles.progressTrack}>
            <View
              style={[
                styles.progressFill,
                {width: `${(answeredCount / CROC_BTI_QUESTIONS.length) * 100}%`},
              ]}
            />
          </View>
          <Text style={styles.progressText}>
            {answeredCount}/{CROC_BTI_QUESTIONS.length} 已回答
          </Text>
        </View>

        {CROC_BTI_QUESTIONS.map((question, index) => (
          <View key={question.id} style={styles.questionCard}>
            <Text style={styles.questionIndex}>{index + 1}</Text>
            <Text style={styles.questionText}>{question.text}</Text>
            <View style={styles.optionRow}>
              {ANSWER_OPTIONS.map(option => {
                const selected = answerMap[question.id] === option.value;
                return (
                  <TouchableOpacity
                    key={option.value}
                    style={[styles.optionButton, selected && styles.optionButtonSelected]}
                    onPress={() =>
                      setAnswerMap(current => ({
                        ...current,
                        [question.id]: option.value,
                      }))
                    }>
                    <Text
                      style={[
                        styles.optionText,
                        selected && styles.optionTextSelected,
                      ]}>
                      {option.label}
                    </Text>
                  </TouchableOpacity>
                );
              })}
            </View>
          </View>
        ))}

        <TouchableOpacity
          style={[styles.primaryButton, !allAnswered && styles.primaryButtonDisabled]}
          onPress={() => setShowResult(true)}
          disabled={!allAnswered}>
          <Text style={styles.primaryButtonText}>
            {allAnswered ? '查看我的鳄bti' : '答完后查看结果'}
          </Text>
        </TouchableOpacity>
      </ScrollView>
    </SafeAreaView>
  );
}

function ResultView({
  result,
  saving,
  onBack,
  onClose,
  onApply,
}: {
  result: CrocBtiResult;
  saving: boolean;
  onBack: () => void;
  onClose: () => void;
  onApply: () => void;
}): React.JSX.Element {
  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <TouchableOpacity onPress={onBack}>
          <Text style={styles.headerButton}>重看题目</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>测试结果</Text>
        <TouchableOpacity onPress={onClose}>
          <Text style={styles.headerButton}>完成</Text>
        </TouchableOpacity>
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.resultHero}>
          <Text style={styles.resultCode}>{result.code}</Text>
          <Text style={styles.resultTitle}>{result.title}</Text>
          <Text style={styles.resultSummary}>{result.summary}</Text>
        </View>

        <View style={styles.panel}>
          <Text style={styles.panelTitle}>推荐学习权重</Text>
          {WEIGHT_LABELS.map(item => (
            <View key={item.key} style={styles.weightRow}>
              <Text style={styles.weightLabel}>{item.label}</Text>
              <View style={styles.weightBarTrack}>
                <View
                  style={[
                    styles.weightBarFill,
                    {width: `${result.weights[item.key]}%`},
                  ]}
                />
              </View>
              <Text style={styles.weightValue}>{result.weights[item.key]}%</Text>
            </View>
          ))}
        </View>

        <View style={styles.panel}>
          <Text style={styles.panelTitle}>写入计划后</Text>
          <View style={styles.planGrid}>
            <PlanTarget label="新词" value={result.planInput.newWordsPerDay} />
            <PlanTarget label="复习" value={result.planInput.reviewWordsPerDay} />
            <PlanTarget label="混测" value={result.planInput.mixedTestPerDay} />
            <PlanTarget label="错题" value={result.planInput.wrongWordTestPerDay} />
            <PlanTarget label="词根" value={result.planInput.rootAffixPerDay} />
          </View>
          <Text style={styles.resultAdvice}>{result.advice}</Text>
        </View>

        <TouchableOpacity
          style={[styles.primaryButton, saving && styles.primaryButtonDisabled]}
          onPress={onApply}
          disabled={saving}>
          <Text style={styles.primaryButtonText}>
            {saving ? '正在应用...' : '应用到学习计划'}
          </Text>
        </TouchableOpacity>
      </ScrollView>
    </SafeAreaView>
  );
}

function PlanTarget({
  label,
  value,
}: {
  label: string;
  value: number;
}): React.JSX.Element {
  return (
    <View style={styles.planTarget}>
      <Text style={styles.planTargetValue}>{value}</Text>
      <Text style={styles.planTargetLabel}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  headerButton: {fontSize: 16, color: '#007AFF', fontWeight: '600'},
  headerTitle: {fontSize: 18, fontWeight: '700', color: '#222'},
  disabledText: {color: '#aaa'},
  scroll: {padding: 16, gap: 12},
  intro: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  kicker: {fontSize: 13, color: '#6B7F32', fontWeight: '700', marginBottom: 6},
  title: {fontSize: 24, fontWeight: '800', color: '#222', marginBottom: 8},
  description: {fontSize: 14, color: '#555', lineHeight: 22},
  progressTrack: {
    height: 8,
    borderRadius: 4,
    backgroundColor: '#edf0e5',
    marginTop: 16,
    overflow: 'hidden',
  },
  progressFill: {height: '100%', backgroundColor: '#8DAA3D'},
  progressText: {fontSize: 12, color: '#666', marginTop: 8},
  questionCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  questionIndex: {fontSize: 12, color: '#8DAA3D', fontWeight: '800', marginBottom: 6},
  questionText: {fontSize: 16, color: '#222', lineHeight: 24, marginBottom: 14},
  optionRow: {gap: 8},
  optionButton: {
    borderWidth: 1,
    borderColor: '#e0e0e0',
    borderRadius: 8,
    paddingVertical: 10,
    paddingHorizontal: 12,
    backgroundColor: '#fff',
  },
  optionButtonSelected: {borderColor: '#8DAA3D', backgroundColor: '#F2F7E8'},
  optionText: {fontSize: 14, color: '#444', textAlign: 'center'},
  optionTextSelected: {color: '#50651F', fontWeight: '700'},
  primaryButton: {
    backgroundColor: '#6B7F32',
    borderRadius: 12,
    paddingVertical: 15,
    alignItems: 'center',
    marginTop: 4,
  },
  primaryButtonDisabled: {opacity: 0.5},
  primaryButtonText: {fontSize: 16, color: '#fff', fontWeight: '700'},
  resultHero: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 20,
    alignItems: 'center',
  },
  resultCode: {fontSize: 14, color: '#8DAA3D', fontWeight: '800', marginBottom: 8},
  resultTitle: {fontSize: 32, color: '#222', fontWeight: '900', marginBottom: 10},
  resultSummary: {fontSize: 15, color: '#555', lineHeight: 23, textAlign: 'center'},
  panel: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  panelTitle: {fontSize: 18, fontWeight: '800', color: '#222', marginBottom: 12},
  weightRow: {flexDirection: 'row', alignItems: 'center', marginBottom: 10, gap: 10},
  weightLabel: {width: 38, fontSize: 13, color: '#555', fontWeight: '600'},
  weightBarTrack: {
    flex: 1,
    height: 10,
    borderRadius: 5,
    backgroundColor: '#edf0e5',
    overflow: 'hidden',
  },
  weightBarFill: {height: '100%', backgroundColor: '#8DAA3D'},
  weightValue: {width: 38, fontSize: 13, color: '#333', textAlign: 'right'},
  planGrid: {flexDirection: 'row', flexWrap: 'wrap', gap: 8, marginBottom: 12},
  planTarget: {
    width: '31%',
    backgroundColor: '#F7F8F3',
    borderRadius: 8,
    paddingVertical: 12,
    alignItems: 'center',
  },
  planTargetValue: {fontSize: 20, fontWeight: '800', color: '#6B7F32'},
  planTargetLabel: {fontSize: 12, color: '#666', marginTop: 4},
  resultAdvice: {fontSize: 14, color: '#555', lineHeight: 22},
});

import React, {useState} from 'react';
import {
  View,
  Text,
  StyleSheet,
  ScrollView,
  TextInput,
  TouchableOpacity,
  SafeAreaView,
  Alert,
} from 'react-native';
import type {PlanSummary} from '../../lib/plan-client';
import {GrowthRuleSection} from './growth-rule-section';
import {WordbookSelectorSection} from './wordbook-selector-section';

interface PlanEditorScreenProps {
  plan: PlanSummary;
  onSave: (plan: PlanSummary) => void;
  onCancel: () => void;
}

export function PlanEditorScreen({
  plan,
  onSave,
  onCancel,
}: PlanEditorScreenProps): React.JSX.Element {
  const [name, setName] = useState(plan.name);
  const [newWordsPerDay, setNewWordsPerDay] = useState(plan.newWordsPerDay.toString());
  const [reviewWordsPerDay, setReviewWordsPerDay] = useState(plan.reviewWordsPerDay.toString());
  const [mixedTestPerDay, setMixedTestPerDay] = useState(plan.mixedTestPerDay.toString());
  const [wrongWordTestPerDay, setWrongWordTestPerDay] = useState(
    plan.wrongWordTestPerDay.toString(),
  );
  const [growthIntervalDays, setGrowthIntervalDays] = useState(
    plan.growthIntervalDays.toString(),
  );
  const [growthIncrement, setGrowthIncrement] = useState(
    plan.growthIncrement.toString(),
  );
  const [selectedWordbookIds, setSelectedWordbookIds] = useState<number[]>([]);
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    const updatedPlan: PlanSummary = {
      ...plan,
      name: name.trim(),
      newWordsPerDay: parseInt(newWordsPerDay, 10) || 0,
      reviewWordsPerDay: parseInt(reviewWordsPerDay, 10) || 0,
      mixedTestPerDay: parseInt(mixedTestPerDay, 10) || 0,
      wrongWordTestPerDay: parseInt(wrongWordTestPerDay, 10) || 0,
      growthIntervalDays: parseInt(growthIntervalDays, 10) || 7,
      growthIncrement: parseInt(growthIncrement, 10) || 5,
    };

    if (!updatedPlan.name) {
      Alert.alert('错误', '计划名称不能为空');
      return;
    }

    if (updatedPlan.newWordsPerDay < 5 || updatedPlan.newWordsPerDay > 100) {
      Alert.alert('错误', '新词目标需要在 5 到 100 之间');
      return;
    }

    setSaving(true);
    try {
      await new Promise(resolve => setTimeout(resolve, 300));
      onSave(updatedPlan);
    } finally {
      setSaving(false);
    }
  };

  const adjustValue = (
    value: string,
    setter: (v: string) => void,
    delta: number,
    min: number,
    max: number,
  ) => {
    const num = parseInt(value, 10) || 0;
    const newValue = Math.max(min, Math.min(max, num + delta));
    setter(newValue.toString());
  };

  const newWordQuestions = (parseInt(newWordsPerDay, 10) || 0) * 4;
  const reviewQuestions = (parseInt(reviewWordsPerDay, 10) || 0) * 4;
  const mixedQuestions = parseInt(mixedTestPerDay, 10) || 0;
  const wrongQuestions = parseInt(wrongWordTestPerDay, 10) || 0;
  const totalQuestions =
    newWordQuestions + reviewQuestions + mixedQuestions + wrongQuestions;

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <TouchableOpacity onPress={onCancel} disabled={saving}>
          <Text style={styles.cancelButton}>取消</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>编辑计划</Text>
        <TouchableOpacity onPress={handleSave} disabled={saving}>
          <Text style={[styles.saveButton, saving && styles.saveButtonDisabled]}>
            {saving ? '保存中...' : '保存'}
          </Text>
        </TouchableOpacity>
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        <Section title="计划名称">
          <TextInput
            style={styles.textInput}
            value={name}
            onChangeText={setName}
            placeholder="请输入计划名称"
            maxLength={50}
          />
        </Section>

        <Section title="每日学习目标">
          <Text style={styles.sectionHint}>
            主页和答题页统一按题显示进度。新词、复习按每词 4 题换算；混合测试和错词强化直接按题配置。
          </Text>

          <NumberField
            label="新词（个）"
            value={newWordsPerDay}
            description={`每词 4 题，共 ${newWordQuestions} 题`}
            step={5}
            onAdjust={delta =>
              adjustValue(newWordsPerDay, setNewWordsPerDay, delta, 5, 100)
            }
            onChange={setNewWordsPerDay}
          />

          <NumberField
            label="复习（个）"
            value={reviewWordsPerDay}
            description={`每词 4 题，共 ${reviewQuestions} 题`}
            step={10}
            onAdjust={delta =>
              adjustValue(reviewWordsPerDay, setReviewWordsPerDay, delta, 10, 200)
            }
            onChange={setReviewWordsPerDay}
          />

          <NumberField
            label="混合测试（题）"
            value={mixedTestPerDay}
            description="按题推进"
            step={5}
            onAdjust={delta =>
              adjustValue(mixedTestPerDay, setMixedTestPerDay, delta, 0, 50)
            }
            onChange={setMixedTestPerDay}
          />

          <NumberField
            label="错词强化（题）"
            value={wrongWordTestPerDay}
            description="按题推进"
            step={5}
            onAdjust={delta =>
              adjustValue(
                wrongWordTestPerDay,
                setWrongWordTestPerDay,
                delta,
                0,
                30,
              )
            }
            onChange={setWrongWordTestPerDay}
          />
        </Section>

        <GrowthRuleSection
          intervalDays={parseInt(growthIntervalDays, 10) || 7}
          increment={parseInt(growthIncrement, 10) || 5}
          onChangeInterval={days => setGrowthIntervalDays(days.toString())}
          onChangeIncrement={inc => setGrowthIncrement(inc.toString())}
        />

        <WordbookSelectorSection
          selectedIds={selectedWordbookIds}
          onChange={setSelectedWordbookIds}
        />

        <View style={styles.summaryCard}>
          <Text style={styles.summaryTitle}>每日总题量</Text>
          <Text style={styles.summaryValue}>{totalQuestions} 题</Text>
          <Text style={styles.summaryHint}>
            新词 {newWordQuestions} 题 / 复习 {reviewQuestions} 题 / 混合 {mixedQuestions}{' '}
            题 / 错词 {wrongQuestions} 题
          </Text>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}): React.JSX.Element {
  return (
    <View style={styles.section}>
      <Text style={styles.sectionTitle}>{title}</Text>
      {children}
    </View>
  );
}

function NumberField({
  label,
  value,
  description,
  step,
  onChange,
  onAdjust,
}: {
  label: string;
  value: string;
  description: string;
  step: number;
  onChange: (value: string) => void;
  onAdjust: (delta: number) => void;
}): React.JSX.Element {
  return (
    <View style={styles.numberField}>
      <View style={styles.numberFieldHeader}>
        <View style={styles.numberCopy}>
          <Text style={styles.numberFieldLabel}>{label}</Text>
          <Text style={styles.numberFieldDescription}>{description}</Text>
        </View>
        <View style={styles.numberControls}>
          <TouchableOpacity style={styles.adjustButton} onPress={() => onAdjust(-step)}>
            <Text style={styles.adjustButtonText}>-</Text>
          </TouchableOpacity>
          <TextInput
            style={styles.numberInput}
            value={value}
            onChangeText={onChange}
            keyboardType="number-pad"
            maxLength={3}
          />
          <TouchableOpacity style={styles.adjustButton} onPress={() => onAdjust(step)}>
            <Text style={styles.adjustButtonText}>+</Text>
          </TouchableOpacity>
        </View>
      </View>
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
  cancelButton: {fontSize: 16, color: '#666'},
  headerTitle: {fontSize: 18, fontWeight: '600'},
  saveButton: {fontSize: 16, color: '#007AFF', fontWeight: '600'},
  saveButtonDisabled: {color: '#999'},
  scroll: {padding: 16},
  section: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  sectionTitle: {fontSize: 18, fontWeight: '600', marginBottom: 12, color: '#333'},
  sectionHint: {fontSize: 13, color: '#666', lineHeight: 18, marginBottom: 12},
  textInput: {
    fontSize: 16,
    borderWidth: 1,
    borderColor: '#e0e0e0',
    borderRadius: 8,
    padding: 12,
    backgroundColor: '#f8f8f8',
  },
  numberField: {marginBottom: 16},
  numberFieldHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  numberCopy: {flex: 1, paddingRight: 12},
  numberFieldLabel: {fontSize: 15, fontWeight: '500', color: '#333'},
  numberFieldDescription: {fontSize: 13, color: '#666', marginTop: 2},
  numberControls: {flexDirection: 'row', alignItems: 'center', gap: 8},
  adjustButton: {
    width: 32,
    height: 32,
    borderRadius: 16,
    backgroundColor: '#f0f0f0',
    justifyContent: 'center',
    alignItems: 'center',
  },
  adjustButtonText: {fontSize: 18, fontWeight: 'bold', color: '#007AFF'},
  numberInput: {
    width: 50,
    fontSize: 18,
    fontWeight: '600',
    textAlign: 'center',
    color: '#007AFF',
  },
  summaryCard: {
    backgroundColor: '#E8F4FD',
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
  },
  summaryTitle: {fontSize: 14, color: '#666', marginBottom: 4},
  summaryValue: {fontSize: 32, fontWeight: 'bold', color: '#007AFF'},
  summaryHint: {
    fontSize: 13,
    color: '#666',
    marginTop: 6,
    textAlign: 'center',
    lineHeight: 18,
  },
});

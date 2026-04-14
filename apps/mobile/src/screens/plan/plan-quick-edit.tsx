import React, {useState} from 'react';
import {
  View,
  Text,
  StyleSheet,
  TextInput,
  TouchableOpacity,
  ScrollView,
  SafeAreaView,
  Alert,
} from 'react-native';
import {
  type PlanSummary,
  savePlan,
  applyPlanToToday,
  validatePlanInput,
} from '../../lib/plan-client';

interface PlanQuickEditProps {
  plan: PlanSummary;
  onSave: (plan: PlanSummary) => void;
  onCancel: () => void;
}

export function PlanQuickEdit({
  plan,
  onSave,
  onCancel,
}: PlanQuickEditProps): React.JSX.Element {
  const [name, setName] = useState(plan.name);
  const [newWords, setNewWords] = useState(plan.newWordsPerDay.toString());
  const [reviewWords, setReviewWords] = useState(plan.reviewWordsPerDay.toString());
  const [mixedTest, setMixedTest] = useState(plan.mixedTestPerDay.toString());
  const [wrongWords, setWrongWords] = useState(plan.wrongWordTestPerDay.toString());
  const [rootAffix, setRootAffix] = useState((plan.rootAffixPerDay ?? 0).toString());
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    const input = {
      name: name.trim(),
      newWordsPerDay: parseInt(newWords, 10) || 0,
      reviewWordsPerDay: parseInt(reviewWords, 10) || 0,
      mixedTestPerDay: parseInt(mixedTest, 10) || 0,
      wrongWordTestPerDay: parseInt(wrongWords, 10) || 0,
      rootAffixPerDay: parseInt(rootAffix, 10) || 0,
    };

    const error = validatePlanInput(input);
    if (error) {
      Alert.alert('输入无效', error);
      return;
    }

    setSaving(true);
    try {
      const savedPlan = await savePlan(plan.id, input);
      Alert.alert(
        '计划已保存',
        '这份计划要立刻应用到今天，还是从明天开始？',
        [
          {
            text: '明天开始',
            onPress: () => onSave(savedPlan),
          },
          {
            text: '应用到今天',
            onPress: async () => {
              try {
                const appliedPlan = await applyPlanToToday();
                onSave(appliedPlan);
              } catch {
                Alert.alert(
                  '应用失败',
                  '计划已保存，但应用到今天失败，请稍后重试。',
                );
              }
            },
          },
        ],
      );
    } catch {
      Alert.alert('保存失败', '请稍后重试。');
    } finally {
      setSaving(false);
    }
  };

  const adjustValue = (value: string, delta: number, min: number, max: number): string => {
    const num = parseInt(value, 10) || 0;
    return Math.max(min, Math.min(max, num + delta)).toString();
  };

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <TouchableOpacity onPress={onCancel} disabled={saving}>
          <Text style={styles.cancelButton}>取消</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>快速编辑计划</Text>
        <TouchableOpacity onPress={handleSave} disabled={saving}>
          <Text style={[styles.saveButton, saving && styles.saveButtonDisabled]}>
            {saving ? '保存中...' : '保存'}
          </Text>
        </TouchableOpacity>
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.section}>
          <Text style={styles.label}>计划名称</Text>
          <TextInput
            style={styles.textInput}
            value={name}
            onChangeText={setName}
            placeholder="请输入计划名称"
            maxLength={50}
          />
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>每日目标</Text>
          <Text style={styles.sectionHint}>
            主页统一按题显示进度。新词、复习按每词 4 题换算；词根词缀按每项 2 题换算。
          </Text>

          <NumberField
            label="新词（个，每词 4 题）"
            helper={`${(parseInt(newWords, 10) || 0) * 4} 题`}
            value={newWords}
            onChange={setNewWords}
            step={5}
            onAdjust={delta => setNewWords(adjustValue(newWords, delta, 5, 100))}
          />
          <NumberField
            label="复习（个，每词 4 题）"
            helper={`${(parseInt(reviewWords, 10) || 0) * 4} 题`}
            value={reviewWords}
            onChange={setReviewWords}
            step={10}
            onAdjust={delta =>
              setReviewWords(adjustValue(reviewWords, delta, 10, 200))
            }
          />
          <NumberField
            label="混合测试（题）"
            helper="按题推进"
            value={mixedTest}
            onChange={setMixedTest}
            step={5}
            onAdjust={delta => setMixedTest(adjustValue(mixedTest, delta, 0, 50))}
          />
          <NumberField
            label="错词强化（题）"
            helper="按题推进"
            value={wrongWords}
            onChange={setWrongWords}
            step={5}
            onAdjust={delta => setWrongWords(adjustValue(wrongWords, delta, 0, 30))}
          />
          <NumberField
            label="词根词缀（项，每项 2 题）"
            helper={`${(parseInt(rootAffix, 10) || 0) * 2} 题`}
            value={rootAffix}
            onChange={setRootAffix}
            step={2}
            onAdjust={delta => setRootAffix(adjustValue(rootAffix, delta, 0, 50))}
          />
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

function NumberField({
  label,
  helper,
  value,
  onChange,
  step,
  onAdjust,
}: {
  label: string;
  helper: string;
  value: string;
  onChange: (value: string) => void;
  step: number;
  onAdjust: (delta: number) => void;
}): React.JSX.Element {
  return (
    <View style={styles.numberField}>
      <View style={styles.numberCopy}>
        <Text style={styles.numberLabel}>{label}</Text>
        <Text style={styles.numberHelper}>{helper}</Text>
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
  section: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 12},
  label: {fontSize: 12, color: '#999', marginBottom: 8},
  sectionTitle: {fontSize: 18, fontWeight: '600', marginBottom: 8, color: '#333'},
  sectionHint: {fontSize: 13, color: '#666', lineHeight: 18, marginBottom: 12},
  textInput: {
    fontSize: 16,
    borderWidth: 1,
    borderColor: '#e0e0e0',
    borderRadius: 8,
    padding: 12,
    backgroundColor: '#f8f8f8',
  },
  numberField: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  numberCopy: {flex: 1, paddingRight: 12},
  numberLabel: {fontSize: 15, color: '#333'},
  numberHelper: {fontSize: 12, color: '#888', marginTop: 3},
  numberControls: {flexDirection: 'row', alignItems: 'center', gap: 12},
  adjustButton: {
    width: 36,
    height: 36,
    borderRadius: 18,
    backgroundColor: '#f0f0f0',
    justifyContent: 'center',
    alignItems: 'center',
  },
  adjustButtonText: {fontSize: 20, fontWeight: 'bold', color: '#007AFF'},
  numberInput: {
    width: 60,
    fontSize: 18,
    fontWeight: '600',
    textAlign: 'center',
    color: '#007AFF',
  },
});

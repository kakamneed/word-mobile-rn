import React, {useEffect, useState} from 'react';
import {
  Alert,
  View,
  Text,
  StyleSheet,
  ScrollView,
  TouchableOpacity,
  SafeAreaView,
  ActivityIndicator,
} from 'react-native';
import {
  applyPlanToToday,
  fetchActivePlan,
  normalizePlanGrowth,
  type PlanSummary,
} from '../../lib/plan-client';
import {
  fetchWordbooks,
  toggleWordbook,
  type Wordbook,
} from '../../lib/vocabulary-client';
import {PlanQuickEdit} from './plan-quick-edit';
import {WordbookList} from '../library/wordbook-list';

interface PlanScreenProps {
  onBack?: () => void;
}

export function PlanScreen({onBack}: PlanScreenProps): React.JSX.Element {
  const [plan, setPlan] = useState<PlanSummary | null>(null);
  const [wordbooks, setWordbooks] = useState<Wordbook[]>([]);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    void loadData();
  }, []);

  const loadData = async () => {
    try {
      const [planData, wordbookData] = await Promise.all([
        fetchActivePlan(),
        fetchWordbooks(),
      ]);
      setPlan(planData);
      setWordbooks(wordbookData);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async (updatedPlan: PlanSummary) => {
    setPlan(updatedPlan);
    setEditing(false);
    await loadData();
  };

  const handleToggleWordbook = async (wordbookId: number, isActive: boolean) => {
    await toggleWordbook(wordbookId, isActive);
    Alert.alert(
      '词书选择已保存',
      '要把新的词书选择应用到今天吗？应用到今天会清空今天已有学习进度并重新开始。',
      [
        {
          text: '明天生效',
          onPress: () => {
            void loadData();
          },
        },
        {
          text: '应用到今天',
          onPress: async () => {
            await applyPlanToToday();
            await loadData();
          },
        },
      ],
    );
  };

  if (loading) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <ActivityIndicator size="large" color="#007AFF" />
        </View>
      </SafeAreaView>
    );
  }

  if (!plan) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <Text style={styles.emptyText}>当前没有可用计划</Text>
          <Text style={styles.emptySubtext}>请先创建一个学习计划</Text>
        </View>
      </SafeAreaView>
    );
  }

  const normalizedPlan = normalizePlanGrowth(plan);

  if (editing) {
    return (
      <PlanQuickEdit
        plan={normalizedPlan}
        onSave={handleSave}
        onCancel={() => setEditing(false)}
      />
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        <TouchableOpacity onPress={onBack}>
          <Text style={styles.backButton}>返回</Text>
        </TouchableOpacity>
        <Text style={styles.headerTitle}>学习计划</Text>
        <TouchableOpacity onPress={() => setEditing(true)}>
          <Text style={styles.editButton}>编辑</Text>
        </TouchableOpacity>
      </View>

      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.card}>
          <Text style={styles.label}>计划名称</Text>
          <Text style={styles.planName}>{plan.name}</Text>
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>每日目标</Text>
          <Text style={styles.sectionHint}>
            主页和答题页统一按题显示进度。新词、复习按每词 4 题换算；词根词缀按每项 1 题计算。
          </Text>

          <View style={styles.targetRow}>
            <TargetItem
              primaryValue={`${plan.newWordsPerDay} 个`}
              secondaryValue={`${plan.newWordsPerDay * 4} 题`}
              label="新词学习"
            />
            <TargetItem
              primaryValue={`${plan.reviewWordsPerDay} 个`}
              secondaryValue={`${plan.reviewWordsPerDay * 4} 题`}
              label="复习"
            />
          </View>

          <View style={styles.targetRow}>
            <TargetItem
              primaryValue={`${plan.mixedTestPerDay} 题`}
              secondaryValue="按题推进"
              label="混合测试"
            />
            <TargetItem
              primaryValue={`${plan.wrongWordTestPerDay} 题`}
              secondaryValue="按题推进"
              label="错词强化"
            />
          </View>

          <View style={styles.targetRow}>
            <TargetItem
              primaryValue={`${plan.rootAffixPerDay ?? 0} 项`}
              secondaryValue={`${plan.rootAffixPerDay ?? 0} 题`}
              label="词根词缀"
              fullWidth
            />
          </View>
        </View>

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>自动增长</Text>
          {normalizedPlan.growthRuleMode === 'perMode' ? (
            <View style={styles.growthList}>
              {(
                [
                  ['newWord', '新词'],
                  ['review', '复习'],
                  ['mixedTest', '混合'],
                  ['wrongWordReinforcement', '错词'],
                  ['rootAffix', '词根词缀'],
                ] as const
              ).map(([mode, label]) => {
                const rule = normalizedPlan.growthRulesByMode?.[mode];
                if (!rule) {
                  return null;
                }
                return (
                  <Text key={mode} style={styles.growthText}>
                    {label}：每 <Text style={styles.growthValue}>{rule.intervalDays}</Text> 天增加{' '}
                    <Text style={styles.growthValue}>{rule.increment}</Text> 个
                  </Text>
                );
              })}
            </View>
          ) : (
            <Text style={styles.growthText}>
              所有模式共用：每{' '}
              <Text style={styles.growthValue}>
                {normalizedPlan.sharedGrowthRule?.intervalDays ?? normalizedPlan.growthIntervalDays}
              </Text>{' '}
              天增加{' '}
              <Text style={styles.growthValue}>
                {normalizedPlan.sharedGrowthRule?.increment ?? normalizedPlan.growthIncrement}
              </Text>{' '}
              个计划单位。
            </Text>
          )}
        </View>

        <WordbookList wordbooks={wordbooks} onToggle={handleToggleWordbook} />

        <View style={styles.card}>
          <Text style={styles.sectionTitle}>说明</Text>
          <Text style={styles.hintText}>
            新词和复习虽然仍以“个”配置，但实际会扩展成完整题组；Today
            页、任务卡和答题进度统一用“题”显示，避免同一模式出现不同口径。
          </Text>
        </View>
      </ScrollView>
    </SafeAreaView>
  );
}

function TargetItem({
  primaryValue,
  secondaryValue,
  label,
  fullWidth,
}: {
  primaryValue: string;
  secondaryValue: string;
  label: string;
  fullWidth?: boolean;
}): React.JSX.Element {
  return (
    <View style={[styles.targetItem, fullWidth && styles.targetItemFull]}>
      <Text style={styles.targetValue}>{primaryValue}</Text>
      <Text style={styles.targetSecondary}>{secondaryValue}</Text>
      <Text style={styles.targetLabel}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  center: {flex: 1, justifyContent: 'center', alignItems: 'center'},
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
  backButton: {fontSize: 16, color: '#007AFF'},
  headerTitle: {fontSize: 18, fontWeight: '600'},
  editButton: {fontSize: 16, color: '#007AFF', fontWeight: '500'},
  scroll: {padding: 16, gap: 12},
  card: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  label: {fontSize: 12, color: '#999', marginBottom: 8},
  planName: {fontSize: 20, fontWeight: '600', color: '#333'},
  sectionTitle: {fontSize: 18, fontWeight: '600', marginBottom: 8, color: '#333'},
  sectionHint: {fontSize: 13, color: '#666', lineHeight: 18, marginBottom: 16},
  targetRow: {flexDirection: 'row', marginBottom: 16, gap: 16},
  targetItem: {
    flex: 1,
    backgroundColor: '#f8f8f8',
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
  },
  targetItemFull: {flex: 0, width: '100%'},
  targetValue: {fontSize: 22, fontWeight: 'bold', color: '#007AFF', marginBottom: 4},
  targetSecondary: {fontSize: 13, color: '#666', marginBottom: 6},
  targetLabel: {fontSize: 13, color: '#666'},
  growthList: {gap: 8},
  growthText: {fontSize: 15, color: '#333', lineHeight: 22},
  growthValue: {fontWeight: '600', color: '#007AFF'},
  hintText: {fontSize: 14, color: '#666', lineHeight: 22},
  emptyText: {fontSize: 18, fontWeight: '600', marginBottom: 8},
  emptySubtext: {fontSize: 14, color: '#666'},
});

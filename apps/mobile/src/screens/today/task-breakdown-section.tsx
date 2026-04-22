import React, {useState} from 'react';
import {StyleSheet, Text, TouchableOpacity, View} from 'react-native';
import type {DailySnapshot, PlanSummary} from '../../lib/today-client';
import type {SessionMode} from '../../lib/mobile-bridge';

interface TaskBreakdownSectionProps {
  snapshot: DailySnapshot;
  activePlan: PlanSummary | null;
  onStartStudy?: (mode: SessionMode) => void;
}

function TaskItem({
  icon,
  label,
  helper,
  completed,
  target,
  denominatorLabel,
  color,
  onPress,
}: {
  icon: string;
  label: string;
  helper?: string;
  completed: number;
  target: number;
  denominatorLabel?: string;
  color: string;
  onPress?: () => void;
}): React.JSX.Element {
  const progress = target > 0 ? completed / target : 1;
  const isComplete = completed >= target;

  return (
    <TouchableOpacity
      style={[styles.taskItem, isComplete && styles.taskItemComplete]}
      onPress={onPress}
      disabled={isComplete || !onPress}>
      <View style={[styles.iconContainer, {backgroundColor: `${color}20`}]}>
        <Text style={[styles.icon, {color}]}>{icon}</Text>
      </View>
      <View style={styles.taskInfo}>
        <Text style={styles.taskLabel}>{label}</Text>
        {helper ? <Text style={styles.taskHelper}>{helper}</Text> : null}
        <View style={styles.progressBar}>
          <View
            style={[
              styles.progressFill,
              {
                width: `${Math.min(progress * 100, 100)}%`,
                backgroundColor: color,
              },
            ]}
          />
        </View>
      </View>
      <View style={styles.taskCount}>
        <Text style={[styles.countText, {color}]}>
          {completed}/{denominatorLabel ?? target}
        </Text>
      </View>
    </TouchableOpacity>
  );
}

export function TaskBreakdownSection({
  snapshot,
  activePlan,
  onStartStudy,
}: TaskBreakdownSectionProps): React.JSX.Element {
  const [expanded, setExpanded] = useState(false);

  const tasks: Array<{
    key: SessionMode;
    icon: string;
    label: string;
    helper?: string;
    denominatorLabel?: string;
    completed: number;
    target: number;
    color: string;
  }> = [
    {
      key: 'newWord' as SessionMode,
      icon: 'N',
      label: '新词学习',
      helper: buildHelperText('newWord', snapshot, activePlan),
      denominatorLabel: buildDenominatorLabel('newWord', snapshot, activePlan),
      completed: snapshot.newWordsCompleted,
      target: snapshot.newWordsTarget,
      color: '#34C759',
    },
    {
      key: 'review' as SessionMode,
      icon: 'R',
      label: '复习',
      helper: buildHelperText('review', snapshot, activePlan),
      denominatorLabel: buildDenominatorLabel('review', snapshot, activePlan),
      completed: snapshot.reviewWordsCompleted,
      target: snapshot.reviewWordsTarget,
      color: '#007AFF',
    },
    {
      key: 'mixedTest' as SessionMode,
      icon: 'M',
      label: '混合测试',
      helper: buildHelperText('mixedTest', snapshot, activePlan),
      denominatorLabel: buildDenominatorLabel('mixedTest', snapshot, activePlan),
      completed: snapshot.mixedTestCompleted,
      target: snapshot.mixedTestTarget,
      color: '#FF9500',
    },
    {
      key: 'wrongWordReinforcement' as SessionMode,
      icon: 'W',
      label: '错词强化',
      helper: buildHelperText('wrongWordReinforcement', snapshot, activePlan),
      denominatorLabel: buildDenominatorLabel('wrongWordReinforcement', snapshot, activePlan),
      completed: snapshot.wrongWordTestCompleted,
      target: snapshot.wrongWordTestTarget,
      color: '#FF3B30',
    },
    {
      key: 'rootAffix' as SessionMode,
      icon: 'RA',
      label: '词根词缀',
      helper: buildHelperText('rootAffix', snapshot, activePlan),
      denominatorLabel: buildDenominatorLabel('rootAffix', snapshot, activePlan),
      completed: snapshot.rootAffixCompleted ?? 0,
      target: snapshot.rootAffixTarget ?? 0,
      color: '#8E44AD',
    },
  ].filter(task => task.target > 0);

  const visibleTasks = expanded ? tasks : tasks.slice(0, 3);

  return (
    <View style={styles.container}>
      <Text style={styles.sectionTitle}>今日任务</Text>
      <Text style={styles.sectionHint}>
        新词和复习按每词 4 题换算；任务说明里会拆出基础计划量和近几天分摊量。
      </Text>

      {visibleTasks.map(task => (
        <TaskItem
          key={task.key}
          icon={task.icon}
          label={task.label}
          helper={task.helper}
          completed={task.completed}
          target={task.target}
          denominatorLabel={task.denominatorLabel}
          color={task.color}
          onPress={() => onStartStudy?.(task.key)}
        />
      ))}

      {tasks.length > 3 ? (
        <TouchableOpacity
          style={styles.expandButton}
          onPress={() => setExpanded(!expanded)}>
          <Text style={styles.expandText}>
            {expanded ? '收起' : `展开其余 ${tasks.length - 3} 项`}
          </Text>
        </TouchableOpacity>
      ) : null}
    </View>
  );
}

function buildHelperText(
  mode: SessionMode,
  snapshot: DailySnapshot,
  activePlan: PlanSummary | null,
): string | undefined {
  const {base, carryover} = taskBreakdown(mode, snapshot, activePlan);
  return carryover > 0 ? `计划 ${base} + 分摊 ${carryover}` : `计划 ${base}`;
}

function buildDenominatorLabel(
  mode: SessionMode,
  snapshot: DailySnapshot,
  activePlan: PlanSummary | null,
): string {
  const {base, carryover} = taskBreakdown(mode, snapshot, activePlan);
  return carryover > 0 ? `${base}+${carryover}` : `${base}`;
}

function taskBreakdown(
  mode: SessionMode,
  snapshot: DailySnapshot,
  activePlan: PlanSummary | null,
): {base: number; carryover: number} {
  switch (mode) {
    case 'newWord': {
      const base = snapshot.newWordsBaseTarget ?? ((activePlan?.newWordsPerDay ?? 0) * 4);
      return {base, carryover: Math.max(snapshot.newWordsTarget - base, 0)};
    }
    case 'review': {
      const base = snapshot.reviewWordsBaseTarget ?? ((activePlan?.reviewWordsPerDay ?? 0) * 4);
      return {base, carryover: Math.max(snapshot.reviewWordsTarget - base, 0)};
    }
    case 'mixedTest': {
      const base = snapshot.mixedTestBaseTarget ?? (activePlan?.mixedTestPerDay ?? 0);
      return {base, carryover: Math.max(snapshot.mixedTestTarget - base, 0)};
    }
    case 'wrongWordReinforcement': {
      const base =
        snapshot.wrongWordTestBaseTarget ?? (activePlan?.wrongWordTestPerDay ?? 0);
      return {base, carryover: Math.max(snapshot.wrongWordTestTarget - base, 0)};
    }
    case 'rootAffix': {
      const total = snapshot.rootAffixTarget ?? 0;
      const base = snapshot.rootAffixBaseTarget ?? (activePlan?.rootAffixPerDay ?? total);
      return {base, carryover: Math.max(total - base, 0)};
    }
    default:
      return {base: 0, carryover: 0};
  }
}

const styles = StyleSheet.create({
  container: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  sectionTitle: {fontSize: 18, fontWeight: '600', marginBottom: 6, color: '#333'},
  sectionHint: {fontSize: 13, color: '#666', lineHeight: 18, marginBottom: 12},
  taskItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  taskItemComplete: {opacity: 0.6},
  iconContainer: {
    minWidth: 40,
    height: 40,
    borderRadius: 20,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: 12,
    paddingHorizontal: 6,
  },
  icon: {fontSize: 12, fontWeight: '700'},
  taskInfo: {flex: 1},
  taskLabel: {fontSize: 15, fontWeight: '500', marginBottom: 2, color: '#333'},
  taskHelper: {fontSize: 12, color: '#888', marginBottom: 6},
  progressBar: {height: 4, backgroundColor: '#f0f0f0', borderRadius: 2, overflow: 'hidden'},
  progressFill: {height: '100%', borderRadius: 2},
  taskCount: {marginLeft: 12},
  countText: {fontSize: 14, fontWeight: '600'},
  expandButton: {alignItems: 'center', paddingVertical: 12, marginTop: 4},
  expandText: {color: '#007AFF', fontSize: 14, fontWeight: '500'},
});

import React, {useState} from 'react';
import {View, Text, StyleSheet, TouchableOpacity} from 'react-native';
import type {DailySnapshot} from '../../lib/today-client';
import type {SessionMode} from '../../lib/mobile-bridge';

interface TaskBreakdownSectionProps {
  snapshot: DailySnapshot;
  onStartStudy?: (mode: SessionMode) => void;
}

function TaskItem({
  icon,
  label,
  helper,
  completed,
  target,
  color,
  onPress,
}: {
  icon: string;
  label: string;
  helper?: string;
  completed: number;
  target: number;
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
              {width: `${Math.min(progress * 100, 100)}%`, backgroundColor: color},
            ]}
          />
        </View>
      </View>
      <View style={styles.taskCount}>
        <Text style={[styles.countText, {color}]}>
          {completed}/{target} 题
        </Text>
      </View>
    </TouchableOpacity>
  );
}

export function TaskBreakdownSection({
  snapshot,
  onStartStudy,
}: TaskBreakdownSectionProps): React.JSX.Element {
  const [expanded, setExpanded] = useState(false);

  const tasks: Array<{
    key: SessionMode;
    icon: string;
    label: string;
    helper?: string;
    completed: number;
    target: number;
    color: string;
  }> = [
    {
      key: 'newWord' as SessionMode,
      icon: '新',
      label: '新词学习',
      helper: buildHelperText('newWord', snapshot.newWordsTarget),
      completed: snapshot.newWordsCompleted,
      target: snapshot.newWordsTarget,
      color: '#34C759',
    },
    {
      key: 'review' as SessionMode,
      icon: '复',
      label: '复习',
      helper: buildHelperText('review', snapshot.reviewWordsTarget),
      completed: snapshot.reviewWordsCompleted,
      target: snapshot.reviewWordsTarget,
      color: '#007AFF',
    },
    {
      key: 'mixedTest' as SessionMode,
      icon: '测',
      label: '混合测试',
      helper: '按题推进',
      completed: snapshot.mixedTestCompleted,
      target: snapshot.mixedTestTarget,
      color: '#FF9500',
    },
    {
      key: 'wrongWordReinforcement' as SessionMode,
      icon: '错',
      label: '错词强化',
      helper: '按题推进',
      completed: snapshot.wrongWordTestCompleted,
      target: snapshot.wrongWordTestTarget,
      color: '#FF3B30',
    },
    {
      key: 'rootAffix' as SessionMode,
      icon: '根',
      label: '词根词缀',
      helper: buildHelperText('rootAffix', snapshot.rootAffixTarget ?? 0),
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
        新词/复习会按每词 4 题换算；词根词缀按每项 2 题换算；主页统一按题显示进度。
      </Text>

      {visibleTasks.map(task => (
        <TaskItem
          key={task.key}
          icon={task.icon}
          label={task.label}
          helper={task.helper}
          completed={task.completed}
          target={task.target}
          color={task.color}
          onPress={() => onStartStudy?.(task.key)}
        />
      ))}

      {tasks.length > 3 ? (
        <TouchableOpacity style={styles.expandButton} onPress={() => setExpanded(!expanded)}>
          <Text style={styles.expandText}>
            {expanded ? '收起' : `展开其余 ${tasks.length - 3} 项`}
          </Text>
        </TouchableOpacity>
      ) : null}
    </View>
  );
}

function buildHelperText(mode: SessionMode, targetQuestions: number): string | undefined {
  if (mode === 'newWord' || mode === 'review') {
    return `${Math.floor(targetQuestions / 4)} 个计划，共 ${targetQuestions} 题`;
  }
  if (mode === 'rootAffix') {
    return `${Math.floor(targetQuestions / 2)} 项计划，共 ${targetQuestions} 题`;
  }
  return undefined;
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
    width: 40,
    height: 40,
    borderRadius: 20,
    justifyContent: 'center',
    alignItems: 'center',
    marginRight: 12,
  },
  icon: {fontSize: 14, fontWeight: '700'},
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

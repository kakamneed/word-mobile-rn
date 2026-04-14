import React from 'react';
import {View, Text, StyleSheet, TouchableOpacity} from 'react-native';
import type {SessionMode} from '../../lib/mobile-bridge';

interface NextActionCardProps {
  action: {
    label: string;
    count: number;
    type: SessionMode | 'done';
  };
  onStart: () => void;
}

export function NextActionCard({
  action,
  onStart,
}: NextActionCardProps): React.JSX.Element {
  const isDone = action.type === 'done';

  return (
    <View style={[styles.card, isDone && styles.cardDone]}>
      <View style={styles.content}>
        <Text style={styles.label}>{isDone ? '今日状态' : '下一步'}</Text>
        <Text style={styles.actionText}>{action.label}</Text>
        {!isDone && action.count > 0 ? (
          <Text style={styles.count}>剩余 {action.count} 题</Text>
        ) : null}
      </View>

      {!isDone ? (
        <TouchableOpacity style={styles.button} onPress={onStart}>
          <Text style={styles.buttonText}>开始</Text>
        </TouchableOpacity>
      ) : (
        <View style={styles.doneBadge}>
          <Text style={styles.doneText}>今日任务已完成</Text>
        </View>
      )}
    </View>
  );
}

const styles = StyleSheet.create({
  card: {backgroundColor: '#007AFF', borderRadius: 16, padding: 24, marginBottom: 16},
  cardDone: {backgroundColor: '#34C759'},
  content: {marginBottom: 16},
  label: {fontSize: 14, color: 'rgba(255,255,255,0.85)', marginBottom: 8},
  actionText: {fontSize: 28, fontWeight: 'bold', color: '#fff', marginBottom: 4},
  count: {fontSize: 16, color: 'rgba(255,255,255,0.9)'},
  button: {backgroundColor: '#fff', borderRadius: 12, paddingVertical: 14, alignItems: 'center'},
  buttonText: {color: '#007AFF', fontSize: 18, fontWeight: '600'},
  doneBadge: {
    backgroundColor: 'rgba(255,255,255,0.2)',
    borderRadius: 8,
    paddingVertical: 10,
    paddingHorizontal: 16,
    alignSelf: 'flex-start',
  },
  doneText: {color: '#fff', fontSize: 16, fontWeight: '600'},
});

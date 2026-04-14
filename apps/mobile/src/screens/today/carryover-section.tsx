import React from 'react';
import {View, Text, StyleSheet, TouchableOpacity} from 'react-native';
import type {PlanSummary, WordbookSummary} from '../../lib/today-client';

interface CarryoverSectionProps {
  activePlan: PlanSummary | null;
  wordbooks: WordbookSummary[];
  onNavigateToPlan?: () => void;
}

export function CarryoverSection({
  activePlan,
  wordbooks,
  onNavigateToPlan,
}: CarryoverSectionProps): React.JSX.Element {
  const activeWordbooks = wordbooks.filter(wb => wb.isActive);

  return (
    <View style={styles.container}>
      {activePlan ? (
        <View style={styles.card}>
          <View style={styles.cardHeader}>
            <Text style={styles.cardTitle}>当前计划</Text>
            <TouchableOpacity onPress={onNavigateToPlan}>
              <Text style={styles.linkText}>编辑 {'>'}</Text>
            </TouchableOpacity>
          </View>

          <Text style={styles.planName}>{activePlan.name}</Text>

          <View style={styles.planStats}>
            <View style={styles.stat}>
              <Text style={styles.statValue}>{activePlan.newWordsPerDay}</Text>
              <Text style={styles.statLabel}>新词/天</Text>
            </View>
            <View style={styles.stat}>
              <Text style={styles.statValue}>{activePlan.reviewWordsPerDay}</Text>
              <Text style={styles.statLabel}>复习/天</Text>
            </View>
            <View style={styles.stat}>
              <Text style={styles.statValue}>{activeWordbooks.length}</Text>
              <Text style={styles.statLabel}>已选词书</Text>
            </View>
          </View>
        </View>
      ) : null}

      <TouchableOpacity style={styles.card} onPress={onNavigateToPlan}>
        <View style={styles.cardHeader}>
          <Text style={styles.cardTitle}>词书设置</Text>
          <Text style={styles.linkText}>去计划页管理 {'>'}</Text>
        </View>

        {activeWordbooks.length > 0 ? (
          <View style={styles.wordbookList}>
            {activeWordbooks.slice(0, 3).map(wb => (
              <View key={wb.id} style={styles.wordbookItem}>
                <Text style={styles.wordbookName}>{wb.name}</Text>
                <Text style={styles.wordbookCount}>{wb.totalEntries} 词</Text>
              </View>
            ))}
            {activeWordbooks.length > 3 ? (
              <Text style={styles.moreText}>另有 {activeWordbooks.length - 3} 本</Text>
            ) : null}
          </View>
        ) : (
          <Text style={styles.emptyText}>当前还没有选中的词书，请到计划页配置。</Text>
        )}
      </TouchableOpacity>

      <View style={[styles.card, styles.tipCard]}>
        <Text style={styles.tipIcon}>提</Text>
        <Text style={styles.tipText}>
          词书入口已并入计划页，错词本保留独立入口，结构会继续朝桌面端的任务语义对齐。
        </Text>
      </View>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {gap: 12},
  card: {backgroundColor: '#fff', borderRadius: 12, padding: 16},
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  cardTitle: {fontSize: 16, fontWeight: '600', color: '#333'},
  linkText: {color: '#007AFF', fontSize: 14},
  planName: {fontSize: 18, fontWeight: '600', marginBottom: 12, color: '#333'},
  planStats: {flexDirection: 'row', gap: 24},
  stat: {alignItems: 'center'},
  statValue: {fontSize: 20, fontWeight: 'bold', color: '#007AFF'},
  statLabel: {fontSize: 12, color: '#666', marginTop: 2},
  wordbookList: {gap: 8},
  wordbookItem: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingVertical: 8,
    borderBottomWidth: 1,
    borderBottomColor: '#f0f0f0',
  },
  wordbookName: {fontSize: 15, color: '#333'},
  wordbookCount: {fontSize: 13, color: '#666'},
  moreText: {fontSize: 13, color: '#999', textAlign: 'center', marginTop: 8},
  emptyText: {fontSize: 14, color: '#666'},
  tipCard: {backgroundColor: '#FFF9E6', flexDirection: 'row', alignItems: 'center', gap: 12},
  tipIcon: {fontSize: 20, fontWeight: '700', color: '#B35900'},
  tipText: {flex: 1, fontSize: 14, color: '#666', lineHeight: 20},
});

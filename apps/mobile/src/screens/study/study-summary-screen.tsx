import React from 'react';
import {View, Text, StyleSheet, TouchableOpacity, ScrollView, SafeAreaView} from 'react-native';
import type {SessionSummary} from '../../lib/study-client';

interface StudySummaryScreenProps {
  summary: SessionSummary;
  nextAction: string;
  onReturnToToday: () => void;
  onContinueNextRound?: () => void;
}

export function StudySummaryScreen({
  summary,
  nextAction,
  onReturnToToday,
  onContinueNextRound,
}: StudySummaryScreenProps): React.JSX.Element {
  const accuracy = Math.round(summary.accuracyPercent);
  const isGoodPerformance = accuracy >= 80;

  return (
    <SafeAreaView style={styles.container}>
      <ScrollView contentContainerStyle={styles.scroll}>
        <View style={styles.header}>
          <Text style={styles.icon}>学</Text>
          <Text style={styles.title}>本项学习完成</Text>
          <Text style={styles.subtitle}>本次共完成 {summary.totalWords} 个学习项</Text>
        </View>

        <View style={styles.statsGrid}>
          <StatCard label="正确率" value={`${accuracy}%`} color={isGoodPerformance ? '#34C759' : '#FF9500'} />
          <StatCard label="正确" value={summary.correctCount.toString()} color="#34C759" />
          <StatCard label="总题数" value={summary.totalQuestions.toString()} color="#007AFF" />
          <StatCard label="用时" value={formatTime(summary.totalTimeMs)} color="#666" />
        </View>

        <View style={styles.breakdownCard}>
          <Text style={styles.breakdownTitle}>结果拆分</Text>
          <BreakdownRow label="正确" value={summary.correctCount} total={summary.totalQuestions} color="#34C759" />
          <BreakdownRow label="基本正确" value={summary.fuzzyCorrectCount} total={summary.totalQuestions} color="#FF9500" />
          <BreakdownRow label="错误" value={summary.incorrectCount} total={summary.totalQuestions} color="#FF3B30" />
          <BreakdownRow label="跳过" value={summary.skippedCount} total={summary.totalQuestions} color="#999" />

          {summary.wrongWordCount > 0 ? (
            <View style={styles.wrongWordsBox}>
              <Text style={styles.wrongWordsText}>本轮有 {summary.wrongWordCount} 个词进入错词复盘。</Text>
            </View>
          ) : null}
        </View>

        <View style={styles.recommendationCard}>
          <Text style={styles.recommendationLabel}>下一步建议</Text>
          <Text style={styles.recommendationText}>{nextAction}</Text>
        </View>
      </ScrollView>

      <View style={styles.actionButtons}>
        {onContinueNextRound ? (
          <TouchableOpacity style={styles.continueButton} onPress={onContinueNextRound}>
            <Text style={styles.continueButtonText}>下一项</Text>
          </TouchableOpacity>
        ) : null}

        <TouchableOpacity style={styles.returnButton} onPress={onReturnToToday}>
          <Text style={styles.returnButtonText}>返回今日页</Text>
        </TouchableOpacity>
      </View>
    </SafeAreaView>
  );
}

function StatCard({
  label,
  value,
  color,
}: {
  label: string;
  value: string;
  color: string;
}): React.JSX.Element {
  return (
    <View style={styles.statCard}>
      <Text style={[styles.statValue, {color}]}>{value}</Text>
      <Text style={styles.statLabel}>{label}</Text>
    </View>
  );
}

function BreakdownRow({
  label,
  value,
  total,
  color,
}: {
  label: string;
  value: number;
  total: number;
  color: string;
}): React.JSX.Element {
  const percentage = total > 0 ? (value / total) * 100 : 0;

  return (
    <View style={styles.breakdownRow}>
      <View style={styles.breakdownLabel}>
        <View style={[styles.dot, {backgroundColor: color}]} />
        <Text style={styles.breakdownLabelText}>{label}</Text>
      </View>
      <View style={styles.breakdownBar}>
        <View style={[styles.breakdownFill, {width: `${percentage}%`, backgroundColor: color}]} />
      </View>
      <Text style={styles.breakdownValue}>{value}</Text>
    </View>
  );
}

function formatTime(ms: number): string {
  const minutes = Math.floor(ms / 60000);
  const seconds = Math.floor((ms % 60000) / 1000);
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  scroll: {padding: 20},
  header: {alignItems: 'center', marginBottom: 24},
  icon: {fontSize: 48, marginBottom: 16, fontWeight: '700', color: '#007AFF'},
  title: {fontSize: 28, fontWeight: 'bold', color: '#333', marginBottom: 8},
  subtitle: {fontSize: 16, color: '#666'},
  statsGrid: {flexDirection: 'row', flexWrap: 'wrap', gap: 12, marginBottom: 20},
  statCard: {flex: 1, minWidth: '45%', backgroundColor: '#fff', borderRadius: 12, padding: 16, alignItems: 'center'},
  statValue: {fontSize: 28, fontWeight: 'bold', marginBottom: 4},
  statLabel: {fontSize: 14, color: '#666'},
  breakdownCard: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  breakdownTitle: {fontSize: 18, fontWeight: '600', marginBottom: 16, color: '#333'},
  breakdownRow: {flexDirection: 'row', alignItems: 'center', marginBottom: 12},
  breakdownLabel: {flexDirection: 'row', alignItems: 'center', width: 100},
  dot: {width: 8, height: 8, borderRadius: 4, marginRight: 8},
  breakdownLabelText: {fontSize: 14, color: '#333'},
  breakdownBar: {flex: 1, height: 8, backgroundColor: '#f0f0f0', borderRadius: 4, marginHorizontal: 12},
  breakdownFill: {height: '100%', borderRadius: 4},
  breakdownValue: {width: 30, textAlign: 'right', fontSize: 14, fontWeight: '600', color: '#333'},
  wrongWordsBox: {backgroundColor: '#FFF3F3', borderRadius: 8, padding: 12, marginTop: 12},
  wrongWordsText: {color: '#D32F2F', fontSize: 14},
  recommendationCard: {backgroundColor: '#E8F4FD', borderRadius: 12, padding: 16, marginBottom: 20},
  recommendationLabel: {fontSize: 12, color: '#666', marginBottom: 8},
  recommendationText: {fontSize: 16, color: '#007AFF', fontWeight: '500'},
  actionButtons: {padding: 20, backgroundColor: '#fff', borderTopWidth: 1, borderTopColor: '#e0e0e0', gap: 12},
  continueButton: {backgroundColor: '#007AFF', borderRadius: 12, paddingVertical: 16, alignItems: 'center'},
  continueButtonText: {color: '#fff', fontSize: 18, fontWeight: '600'},
  returnButton: {backgroundColor: '#f0f0f0', borderRadius: 12, paddingVertical: 16, alignItems: 'center'},
  returnButtonText: {color: '#333', fontSize: 18, fontWeight: '600'},
});

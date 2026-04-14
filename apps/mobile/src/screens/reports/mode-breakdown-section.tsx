import React from 'react';
import {StyleSheet, Text, TouchableOpacity, View} from 'react-native';
import type {
  DailyStat,
  ModeBreakdown,
  ReportsOverview,
} from '../../lib/reports-client';
import {DailyLineChart} from './daily-line-chart';

interface ModeBreakdownSectionProps {
  breakdown: ModeBreakdown[];
  selectedMode: string | null;
  onSelectMode: (mode: string | null) => void;
  seriesByMode: ReportsOverview['modeSeries'];
}

const MODE_CONFIG: Record<string, {label: string; color: string}> = {
  newWord: {label: '新词学习', color: '#34C759'},
  review: {label: '复习', color: '#007AFF'},
  mixedTest: {label: '混合测试', color: '#FF9500'},
  wrongWordReinforcement: {label: '错词强化', color: '#FF3B30'},
  rootAffix: {label: '词根词缀', color: '#8E44AD'},
};

export function ModeBreakdownSection({
  breakdown,
  selectedMode,
  onSelectMode,
  seriesByMode,
}: ModeBreakdownSectionProps): React.JSX.Element {
  return (
    <View style={styles.container}>
      <Text style={styles.title}>按模式查看</Text>
      <Text style={styles.subtitle}>点击模式可展开建议和该模式独立趋势图</Text>

      {breakdown.map(item => {
        const config = MODE_CONFIG[item.mode];
        const missed = item.totalQuestions - item.correctCount;
        const selected = selectedMode === item.mode;
        const modeSeries = (seriesByMode[item.mode] ?? []) as DailyStat[];

        return (
          <TouchableOpacity
            key={item.mode}
            style={[styles.card, selected && styles.cardSelected]}
            onPress={() => onSelectMode(selected ? null : item.mode)}>
            <View style={styles.cardHeader}>
              <Text style={styles.cardTitle}>{config.label}</Text>
              <Text style={[styles.cardAccuracy, {color: config.color}]}>
                {Math.round(item.accuracyPercent)}%
              </Text>
            </View>

            <View style={styles.statsRow}>
              <Text style={styles.statsText}>题目 {item.totalQuestions}</Text>
              <Text style={styles.statsText}>正确 {item.correctCount}</Text>
              <Text style={styles.statsText}>错题 {missed}</Text>
            </View>

            <View style={styles.progressTrack}>
              <View
                style={[
                  styles.progressFill,
                  {width: `${item.accuracyPercent}%`, backgroundColor: config.color},
                ]}
              />
            </View>

            {selected ? (
              <>
                <Text style={styles.tipText}>
                  {buildAdvice(config.label, item.accuracyPercent)}
                </Text>
                <DailyLineChart
                  data={modeSeries}
                  title={`${config.label}趋势`}
                  subtitle="可左右滑动，点击点查看当天正确率"
                  lineColor={config.color}
                  compact
                />
              </>
            ) : null}
          </TouchableOpacity>
        );
      })}
    </View>
  );
}

function buildAdvice(label: string, accuracy: number): string {
  if (accuracy >= 85) {
    return `${label}表现稳定，可以继续保持当前节奏。`;
  }
  if (accuracy >= 50) {
    return `${label}表现中等，建议继续巩固薄弱点。`;
  }
  return `${label}仍有明显薄弱点，建议降低速度并加强回顾。`;
}

const styles = StyleSheet.create({
  container: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  title: {fontSize: 18, fontWeight: '600', color: '#333', marginBottom: 4},
  subtitle: {fontSize: 14, color: '#999', marginBottom: 16},
  card: {
    backgroundColor: '#f8f8f8',
    borderRadius: 12,
    padding: 16,
    marginBottom: 12,
    borderWidth: 1,
    borderColor: 'transparent',
  },
  cardSelected: {borderColor: '#007AFF', backgroundColor: '#F4F9FF'},
  cardHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 12,
  },
  cardTitle: {fontSize: 16, fontWeight: '600', color: '#333'},
  cardAccuracy: {fontSize: 20, fontWeight: '700'},
  statsRow: {flexDirection: 'row', gap: 16, marginBottom: 12},
  statsText: {fontSize: 13, color: '#666'},
  progressTrack: {
    height: 8,
    borderRadius: 4,
    backgroundColor: '#e0e0e0',
    overflow: 'hidden',
  },
  progressFill: {height: '100%', borderRadius: 4},
  tipText: {fontSize: 14, color: '#666', marginTop: 12, lineHeight: 20},
});

import React, {useEffect, useState} from 'react';
import {
  RefreshControl,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import {
  fetchReportsOverview,
  type ReportsOverview,
} from '../../lib/reports-client';
import {DailyLineChart} from './daily-line-chart';
import {ModeBreakdownSection} from './mode-breakdown-section';

interface ReportsOverviewScreenProps {
  onBack?: () => void;
}

export function ReportsOverviewScreen({
  onBack,
}: ReportsOverviewScreenProps): React.JSX.Element {
  const [reports, setReports] = useState<ReportsOverview | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [selectedMode, setSelectedMode] = useState<string | null>(null);

  const loadReports = async () => {
    try {
      const next = await fetchReportsOverview();
      setReports(next);
    } finally {
      setRefreshing(false);
    }
  };

  useEffect(() => {
    void loadReports();
  }, []);

  if (!reports) {
    return (
      <SafeAreaView style={styles.container}>
        <View style={styles.center}>
          <Text style={styles.loadingText}>正在加载学习报告...</Text>
        </View>
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.header}>
        {onBack ? (
          <TouchableOpacity onPress={onBack}>
            <Text style={styles.backButton}>返回</Text>
          </TouchableOpacity>
        ) : (
          <View style={styles.headerSpacer} />
        )}
        <Text style={styles.headerTitle}>学习报告</Text>
        <View style={styles.headerSpacer} />
      </View>

      <ScrollView
        nestedScrollEnabled
        contentContainerStyle={styles.scroll}
        refreshControl={
          <RefreshControl
            refreshing={refreshing}
            onRefresh={() => {
              setRefreshing(true);
              void loadReports();
            }}
          />
        }>
        <View style={styles.streakCard}>
          <Text style={styles.streakLabel}>连续学习</Text>
          <Text style={styles.streakValue}>{reports.streakInfo.currentStreak} 天</Text>
          <Text style={styles.streakSubtext}>
            最长连续 {reports.streakInfo.longestStreak} 天
          </Text>
        </View>

        <View style={styles.grid}>
          <MetricCard label="累计学习天数" value={reports.totalStudyDays.toString()} />
          <MetricCard label="已学词数" value={reports.totalWordsLearned.toString()} />
          <MetricCard label="总答题数" value={reports.totalQuestionsAnswered.toString()} />
          <MetricCard
            label="整体正确率"
            value={`${Math.round(reports.overallAccuracy)}%`}
            highlight
          />
        </View>

        <DailyLineChart
          data={reports.dailySeries}
          title="每日正确率趋势"
          subtitle="左右滑动查看不同日期，点击图上节点查看当天详情。"
        />

        <ModeBreakdownSection
          breakdown={reports.modeBreakdown}
          selectedMode={selectedMode}
          onSelectMode={setSelectedMode}
          seriesByMode={reports.modeSeries}
        />
      </ScrollView>
    </SafeAreaView>
  );
}

function MetricCard({
  label,
  value,
  highlight,
}: {
  label: string;
  value: string;
  highlight?: boolean;
}): React.JSX.Element {
  return (
    <View style={[styles.metricCard, highlight && styles.metricCardHighlight]}>
      <Text style={[styles.metricValue, highlight && styles.metricValueHighlight]}>
        {value}
      </Text>
      <Text style={styles.metricLabel}>{label}</Text>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {flex: 1, backgroundColor: '#f5f5f5'},
  center: {flex: 1, justifyContent: 'center', alignItems: 'center'},
  loadingText: {fontSize: 16, color: '#666'},
  header: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e5e5e5',
  },
  backButton: {fontSize: 16, color: '#007AFF', minWidth: 40},
  headerTitle: {fontSize: 18, fontWeight: '600', color: '#333'},
  headerSpacer: {minWidth: 40},
  scroll: {padding: 16},
  streakCard: {
    backgroundColor: '#FFF4D6',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
  },
  streakLabel: {fontSize: 14, color: '#8A5A00'},
  streakValue: {fontSize: 28, fontWeight: '700', color: '#8A5A00', marginTop: 6},
  streakSubtext: {fontSize: 14, color: '#8A5A00', marginTop: 4},
  grid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
    marginBottom: 16,
  },
  metricCard: {
    minWidth: '47%',
    flex: 1,
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
  },
  metricCardHighlight: {
    backgroundColor: '#EAF4FF',
    borderWidth: 1,
    borderColor: '#007AFF',
  },
  metricValue: {fontSize: 24, fontWeight: '700', color: '#333', marginBottom: 6},
  metricValueHighlight: {color: '#007AFF'},
  metricLabel: {fontSize: 13, color: '#666'},
});

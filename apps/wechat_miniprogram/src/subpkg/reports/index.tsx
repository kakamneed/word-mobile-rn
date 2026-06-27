import { useEffect, useMemo, useState } from 'react';
import { ScrollView, Text, View } from '@tarojs/components';

import { Screen } from '@/components/Screen';
import {
  getCachedReportsOverview,
  saveCachedReportsOverview,
} from '@/sdk/cloudCacheStore';
import {
  buildReportChartPoints,
  reportAccuracy,
  ReportSeriesItem,
  REPORT_CHART_LEFT_GUTTER,
  REPORT_CHART_SEGMENT_THICKNESS,
  REPORT_CHART_TOUCH_SIZE,
  REPORT_CHART_TOP_GUTTER,
} from '@/sdk/reportChart';
import { sdk } from '@/sdk';
import { ReportsOverview, StudyMode } from '@/sdk/types';

import './index.scss';

const emptyReports: ReportsOverview = {
  totalStudyDays: 0,
  totalWordsLearned: 0,
  totalQuestionsAnswered: 0,
  overallAccuracy: 0,
  streakInfo: { currentStreak: 0, longestStreak: 0 },
  dailySeries: [],
  modeBreakdown: [],
  modeSeries: {},
};

const modeNames: Record<StudyMode, string> = {
  newWord: '新词学习',
  review: '复习',
  mixedTest: '混合测试',
  wrongWordReinforcement: '错词强化',
  rootAffix: '词根词缀',
};

const modeColors: Record<StudyMode, string> = {
  newWord: '#34c759',
  review: '#007aff',
  mixedTest: '#ff9500',
  wrongWordReinforcement: '#ff3b30',
  rootAffix: '#8e44ad',
};

function totalQuestions(item: { totalQuestions?: number; questionsAnswered?: number }) {
  return item.totalQuestions ?? item.questionsAnswered ?? 0;
}

function formatDuration(ms?: number) {
  const totalSeconds = Math.round((ms ?? 0) / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}m ${seconds}s`;
}

function Chart({
  data,
  selectedDate,
  compact = false,
  onSelect,
}: {
  data: ReportSeriesItem[];
  selectedDate?: string;
  compact?: boolean;
  onSelect?: (item: ReportSeriesItem) => void;
}) {
  const chart = useMemo(() => buildReportChartPoints(data, compact), [compact, data]);
  if (!data.length) return <Text className="chart-empty">暂无报告数据</Text>;

  return (
    <ScrollView scrollX className={compact ? 'line-chart line-chart--compact' : 'line-chart'}>
      <View className="line-chart__canvas" style={{ width: `${chart.chartWidth}px`, height: `${chart.chartHeight + 38}px` }}>
        {chart.segments.map((segment) => (
          <View
            key={segment.key}
            className="line-chart__segment"
            style={{
              left: `${segment.left}px`,
              top: `${segment.top}px`,
              width: `${segment.width}px`,
              height: `${REPORT_CHART_SEGMENT_THICKNESS}px`,
              background: compact ? '#6b5aa0' : '#211c28',
              transform: `rotate(${segment.angleDeg}deg)`,
            }}
          />
        ))}
        <View className="line-chart__axis line-chart__axis--left" style={{ left: `${REPORT_CHART_LEFT_GUTTER}px`, top: `${REPORT_CHART_TOP_GUTTER}px`, height: `${chart.plotHeight}px` }} />
        <View className="line-chart__axis line-chart__axis--bottom" style={{ left: `${REPORT_CHART_LEFT_GUTTER}px`, top: `${REPORT_CHART_TOP_GUTTER + chart.plotHeight}px`, width: `${chart.plotWidth}px` }} />
        <View className="line-chart__guide line-chart__guide--top" style={{ left: `${REPORT_CHART_LEFT_GUTTER}px`, top: `${REPORT_CHART_TOP_GUTTER}px`, width: `${chart.plotWidth}px` }} />
        <View className="line-chart__guide line-chart__guide--mid" style={{ left: `${REPORT_CHART_LEFT_GUTTER}px`, top: `${REPORT_CHART_TOP_GUTTER + chart.plotHeight / 2}px`, width: `${chart.plotWidth}px` }} />
        {chart.points.map((point) => (
          <View key={point.item.date}>
            <View
              className={`line-chart__dot ${selectedDate === point.item.date ? 'line-chart__dot--active' : ''}`}
              style={{ left: `${point.x - REPORT_CHART_TOUCH_SIZE / 2}px`, top: `${point.y - REPORT_CHART_TOUCH_SIZE / 2}px`, color: point.color }}
              onClick={(event) => {
                event.stopPropagation();
                onSelect?.(point.item);
              }}
            />
            <View className="line-chart__label" style={{ left: `${point.x - 28}px`, top: `${chart.chartHeight + 4}px` }}>
              <Text>{point.label}</Text>
              <Text>{point.accuracy}%</Text>
            </View>
          </View>
        ))}
      </View>
    </ScrollView>
  );
}

export default function ReportsPage() {
  const cached = getCachedReportsOverview();
  const [reports, setReports] = useState<ReportsOverview>(cached ?? emptyReports);
  const [loading, setLoading] = useState(!cached);
  const [selectedDay, setSelectedDay] = useState<ReportSeriesItem | undefined>(
    cached?.dailySeries[cached.dailySeries.length - 1],
  );
  const [selectedMode, setSelectedMode] = useState<StudyMode | null>(
    cached?.modeBreakdown[0]?.mode ?? null,
  );

  useEffect(() => {
    let disposed = false;
    setLoading(!getCachedReportsOverview());
    sdk.reports
      .getOverview()
      .then((overview) => {
        if (disposed) return;
        const next = { ...overview, modeSeries: overview.modeSeries ?? {} };
        saveCachedReportsOverview(next);
        setReports(next);
        setSelectedDay(next.dailySeries[next.dailySeries.length - 1]);
        setSelectedMode(next.modeBreakdown[0]?.mode ?? null);
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  const selectedAccuracy = selectedDay ? reportAccuracy(selectedDay) : 0;

  return (
    <Screen title="报告" activeTab="reports">
      {loading && <Text className="page-loading">正在同步云端报告...</Text>}

      <View className="report-hero">
        <Text className="report-hero__title">{reports.streakInfo.currentStreak} 天连续学习</Text>
        <Text className="report-hero__caption">报告页会把每日趋势和模式趋势一起展开展示。</Text>
      </View>

      <View className="report-metrics">
        <View className="report-metric"><Text>{reports.totalStudyDays}</Text><Text>学习天数</Text></View>
        <View className="report-metric"><Text>{reports.totalWordsLearned}</Text><Text>已学词数</Text></View>
        <View className="report-metric"><Text>{reports.totalQuestionsAnswered}</Text><Text>总答题数</Text></View>
        <View className="report-metric report-metric--muted"><Text>{reports.overallAccuracy}%</Text><Text>整体正确率</Text></View>
      </View>

      <View className="report-card">
        <Text className="report-card__title">每日正确率趋势</Text>
        {selectedDay && (
          <View className="selected-day-card">
            <Text>{selectedDay.date}</Text>
            <Text>正确率 {selectedAccuracy}% / 题量 {totalQuestions(selectedDay)} / 答对 {selectedDay.correctCount ?? 0}</Text>
          </View>
        )}
        <Chart data={reports.dailySeries} selectedDate={selectedDay?.date} onSelect={(item) => setSelectedDay(item)} />
        <View className="day-analysis">
          <Text>{selectedDay?.date ?? '--'} 当日分析</Text>
          <Text>答题数：{selectedDay ? totalQuestions(selectedDay) : 0}</Text>
          <Text>正确率：{selectedAccuracy}%</Text>
          <Text>学习时长：{formatDuration(selectedDay?.studyTimeMs ?? selectedDay?.totalTimeMs)}</Text>
        </View>
      </View>

      <View className="report-card">
        <Text className="report-card__title">按模式查看</Text>
        <View className="mode-report-list">
          {reports.modeBreakdown.map((mode) => {
            const value = reportAccuracy(mode);
            const color = modeColors[mode.mode];
            const selected = selectedMode === mode.mode;
            const modeSeries = reports.modeSeries[mode.mode] ?? [];
            return (
              <View className={`mode-report ${selected ? 'mode-report--expanded' : 'mode-report--compact'}`} key={mode.mode} onClick={() => setSelectedMode(selected ? null : mode.mode)}>
                <View className="mode-report__header">
                  <Text className="mode-report__name">{modeNames[mode.mode] ?? mode.mode}</Text>
                  <Text className="mode-report__percent" style={{ color }}>{value}%</Text>
                </View>
                <Text className="mode-report__meta">题目 {totalQuestions(mode)} / 正确 {mode.correctCount ?? 0}</Text>
                <View className="mode-report__track"><View className="mode-report__fill" style={{ width: `${value}%`, background: color }} /></View>
                {selected && (
                  <View className="mode-report__expanded">
                    <Text className="mode-report__advice">点开某个模式后，会显示该模式自己的趋势图和建议。</Text>
                    <Chart data={modeSeries} compact />
                  </View>
                )}
              </View>
            );
          })}
        </View>
      </View>
    </Screen>
  );
}

import React, {useMemo, useState} from 'react';
import {
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import type {DailyStat} from '../../lib/reports-client';

interface DailyLineChartProps {
  data: DailyStat[];
  title?: string;
  subtitle?: string;
  lineColor?: string;
  compact?: boolean;
}

const CHART_HEIGHT = 140;
const COMPACT_CHART_HEIGHT = 104;
const CHART_MIN_WIDTH = 300;
const STEP_X = 56;
const LEFT_GUTTER = 34;
const RIGHT_GUTTER = 18;
const TOP_GUTTER = 12;
const BOTTOM_GUTTER = 22;
const DOT_SIZE = 12;
const COMPACT_DOT_SIZE = 10;

export function DailyLineChart({
  data,
  title = '每日题量趋势',
  subtitle = '可左右滑动，点击点查看当天正确率',
  lineColor = '#007AFF',
  compact = false,
}: DailyLineChartProps): React.JSX.Element {
  const [selectedIndex, setSelectedIndex] = useState(Math.max(data.length - 1, 0));

  const chartHeight = compact ? COMPACT_CHART_HEIGHT : CHART_HEIGHT;
  const dotSize = compact ? COMPACT_DOT_SIZE : DOT_SIZE;
  const plotHeight = chartHeight - TOP_GUTTER - BOTTOM_GUTTER;
  const plotWidth = Math.max(
    CHART_MIN_WIDTH - LEFT_GUTTER - RIGHT_GUTTER,
    Math.max(data.length - 1, 0) * STEP_X,
  );
  const chartWidth = LEFT_GUTTER + plotWidth + RIGHT_GUTTER;

  const points = useMemo(() => {
    if (data.length === 0) {
      return [];
    }
    const maxValue = Math.max(...data.map(item => item.totalQuestions), 1);
    return data.map((item, index) => {
      const x = LEFT_GUTTER + (data.length === 1 ? plotWidth / 2 : index * STEP_X);
      const y = TOP_GUTTER + plotHeight - (item.totalQuestions / maxValue) * plotHeight;
      return {...item, x, y};
    });
  }, [data, plotHeight, plotWidth]);

  const maxValue = Math.max(...data.map(item => item.totalQuestions), 1);
  const selectedPoint =
    points[Math.min(Math.max(selectedIndex, 0), Math.max(points.length - 1, 0))];

  if (points.length === 0) {
    return <View />;
  }

  return (
    <View style={[styles.card, compact && styles.cardCompact]}>
      <Text style={[styles.title, compact && styles.titleCompact]}>{title}</Text>
      <Text style={[styles.subtitle, compact && styles.subtitleCompact]}>{subtitle}</Text>

      {selectedPoint ? (
        <View style={styles.selectedCard}>
          <Text style={styles.selectedDate}>{selectedPoint.date}</Text>
          <Text style={styles.selectedMeta}>
            {selectedPoint.totalQuestions} 题 / 正确 {selectedPoint.correctCount} / 正确率{' '}
            {Math.round(selectedPoint.accuracyPercent)}%
          </Text>
        </View>
      ) : null}

      <ScrollView horizontal showsHorizontalScrollIndicator={false}>
        <View>
          <View style={[styles.chartFrame, {height: chartHeight, width: chartWidth}]}>
            <Text style={[styles.yAxisLabel, {top: TOP_GUTTER - 8}]}>{maxValue}</Text>
            <Text style={[styles.yAxisLabel, {top: TOP_GUTTER + plotHeight / 2 - 8}]}>
              {Math.round(maxValue / 2)}
            </Text>
            <Text style={[styles.yAxisLabel, {top: TOP_GUTTER + plotHeight - 8}]}>0</Text>

            <View
              style={[
                styles.axisLine,
                styles.yAxis,
                {left: LEFT_GUTTER, top: TOP_GUTTER, height: plotHeight},
              ]}
            />
            <View
              style={[
                styles.axisLine,
                styles.xAxis,
                {left: LEFT_GUTTER, top: TOP_GUTTER + plotHeight, width: plotWidth},
              ]}
            />
            <View
              style={[
                styles.guideLine,
                {left: LEFT_GUTTER, top: TOP_GUTTER, width: plotWidth},
              ]}
            />
            <View
              style={[
                styles.guideLine,
                {left: LEFT_GUTTER, top: TOP_GUTTER + plotHeight / 2, width: plotWidth},
              ]}
            />

            {points.slice(0, -1).map((point, index) => {
              const next = points[index + 1];
              const dx = next.x - point.x;
              const dy = next.y - point.y;
              const length = Math.sqrt(dx * dx + dy * dy);
              const angle = `${Math.atan2(dy, dx)}rad`;

              return (
                <View
                  key={`${point.date}-${next.date}`}
                  style={[
                    styles.segment,
                    {
                      width: length,
                      left: point.x + dx / 2 - length / 2,
                      top: point.y + dy / 2 - 1,
                      backgroundColor: lineColor,
                      transform: [{rotate: angle}],
                    },
                  ]}
                />
              );
            })}

            {points.map((point, index) => {
              const pointColor = accuracyColor(point.accuracyPercent);
              return (
                <TouchableOpacity
                  key={point.date}
                  activeOpacity={0.8}
                  style={[
                    styles.dotWrap,
                    {
                      left: point.x - dotSize,
                      top: point.y - dotSize,
                      width: dotSize * 2,
                      height: dotSize * 2,
                    },
                  ]}
                  onPress={() => setSelectedIndex(index)}>
                  <View
                    style={[
                      styles.dot,
                      {
                        width: dotSize,
                        height: dotSize,
                        borderRadius: dotSize / 2,
                        backgroundColor: pointColor,
                        borderWidth: selectedIndex === index ? 2 : 0,
                        borderColor: '#fff',
                      },
                    ]}
                  />
                </TouchableOpacity>
              );
            })}
          </View>

          <View style={[styles.footer, {width: chartWidth}]}>
            <View style={{width: LEFT_GUTTER}} />
            {points.map(point => (
              <View key={point.date} style={[styles.footerItem, {width: STEP_X}]}>
                <Text style={styles.footerDate}>{point.date.slice(5)}</Text>
                <Text style={styles.footerValue}>{point.totalQuestions}</Text>
              </View>
            ))}
          </View>
        </View>
      </ScrollView>
    </View>
  );
}

function accuracyColor(accuracy: number): string {
  if (accuracy < 50) {
    return '#FF3B30';
  }
  if (accuracy < 85) {
    return '#FF9500';
  }
  return '#34C759';
}

const styles = StyleSheet.create({
  card: {backgroundColor: '#fff', borderRadius: 12, padding: 16, marginBottom: 16},
  cardCompact: {
    backgroundColor: '#EEF5FF',
    padding: 12,
    marginTop: 12,
    marginBottom: 0,
  },
  title: {fontSize: 18, fontWeight: '600', color: '#333'},
  titleCompact: {fontSize: 15},
  subtitle: {fontSize: 13, color: '#666', marginTop: 4, marginBottom: 12},
  subtitleCompact: {fontSize: 12, marginBottom: 8},
  selectedCard: {
    backgroundColor: '#F8F8F8',
    borderRadius: 10,
    paddingHorizontal: 12,
    paddingVertical: 10,
    marginBottom: 12,
  },
  selectedDate: {fontSize: 13, fontWeight: '600', color: '#333', marginBottom: 3},
  selectedMeta: {fontSize: 13, color: '#666', lineHeight: 18},
  chartFrame: {
    position: 'relative',
    marginBottom: 10,
  },
  axisLine: {
    position: 'absolute',
    backgroundColor: '#D0D7E2',
  },
  xAxis: {height: 1},
  yAxis: {width: 1},
  guideLine: {
    position: 'absolute',
    height: 1,
    borderTopWidth: 1,
    borderStyle: 'dashed',
    borderColor: '#E7ECF3',
  },
  yAxisLabel: {
    position: 'absolute',
    left: 0,
    width: LEFT_GUTTER - 8,
    fontSize: 10,
    color: '#999',
    textAlign: 'right',
  },
  segment: {
    position: 'absolute',
    height: 2,
  },
  dotWrap: {
    position: 'absolute',
    alignItems: 'center',
    justifyContent: 'center',
  },
  dot: {
    position: 'absolute',
  },
  footer: {
    flexDirection: 'row',
    justifyContent: 'flex-start',
  },
  footerItem: {alignItems: 'center'},
  footerDate: {fontSize: 11, color: '#999'},
  footerValue: {fontSize: 12, fontWeight: '600', color: '#333', marginTop: 2},
});

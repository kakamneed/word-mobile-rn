import { ReportsOverview } from './types';

export type ReportSeriesItem = ReportsOverview['dailySeries'][number];

export const REPORT_CHART_STEP_X = 56;
export const REPORT_CHART_EDGE_INSET = REPORT_CHART_STEP_X / 2;
export const REPORT_CHART_LEFT_GUTTER = 34;
export const REPORT_CHART_RIGHT_GUTTER = 18;
export const REPORT_CHART_TOP_GUTTER = 12;
export const REPORT_CHART_BOTTOM_GUTTER = 22;
export const REPORT_CHART_HEIGHT = 140;
export const REPORT_CHART_COMPACT_HEIGHT = 104;
export const REPORT_CHART_TOUCH_SIZE = 24;
export const REPORT_CHART_VISUAL_DOT_SIZE = 12;
export const REPORT_CHART_ACTIVE_DOT_SIZE = 16;
export const REPORT_CHART_SEGMENT_THICKNESS = 2;

export function reportAccuracy(item: {
  accuracyPercent?: number;
  accuracy?: number;
  totalQuestions?: number;
  questionsAnswered?: number;
  correctCount?: number;
}) {
  if (typeof item.accuracyPercent === 'number') return clampPercent(item.accuracyPercent);
  if (typeof item.accuracy === 'number') return clampPercent(item.accuracy);
  const total = item.totalQuestions ?? item.questionsAnswered ?? 0;
  const correct = item.correctCount ?? 0;
  return total ? clampPercent(Math.round((correct / total) * 100)) : 0;
}

export function reportAccuracyColor(accuracy: number) {
  if (accuracy < 50) return '#ff3b30';
  if (accuracy < 85) return '#ff9500';
  return '#34c759';
}

export function buildReportChartPoints(data: ReportSeriesItem[], compact = false) {
  const chartHeight = compact ? REPORT_CHART_COMPACT_HEIGHT : REPORT_CHART_HEIGHT;
  const plotHeight = chartHeight - REPORT_CHART_TOP_GUTTER - REPORT_CHART_BOTTOM_GUTTER;
  const plotWidth = Math.max(
    300 - REPORT_CHART_LEFT_GUTTER - REPORT_CHART_RIGHT_GUTTER,
    data.length * REPORT_CHART_STEP_X,
  );
  const chartWidth = REPORT_CHART_LEFT_GUTTER + plotWidth + REPORT_CHART_RIGHT_GUTTER;
  const points = data.map((item, index) => {
    const accuracy = reportAccuracy(item);
    const x = REPORT_CHART_LEFT_GUTTER + REPORT_CHART_EDGE_INSET + index * REPORT_CHART_STEP_X;
    const y = REPORT_CHART_TOP_GUTTER + plotHeight - (accuracy / 100) * plotHeight;
    return {
      item,
      index,
      selectedKey: item.date,
      label: item.date.replace(/^\d{4}-/, ''),
      compact,
      x,
      y,
      accuracy,
      color: reportAccuracyColor(accuracy),
      hitboxLeft: x - REPORT_CHART_TOUCH_SIZE / 2,
      hitboxTop: y - REPORT_CHART_TOUCH_SIZE / 2,
      visualCenterX: x,
      visualCenterY: y,
    };
  });
  const segments = points.slice(1).map((point, index) => {
    const previous = points[index];
    const deltaX = point.x - previous.x;
    const deltaY = point.y - previous.y;
    const width = Math.sqrt(deltaX * deltaX + deltaY * deltaY);
    return {
      key: `${previous.item.date}-${point.item.date}`,
      left: previous.x,
      top: previous.y - REPORT_CHART_SEGMENT_THICKNESS / 2,
      width,
      angleDeg: (Math.atan2(deltaY, deltaX) * 180) / Math.PI,
      startX: previous.x,
      startY: previous.y,
      endX: point.x,
      endY: point.y,
    };
  });
  return { points, segments, chartHeight, plotHeight, plotWidth, chartWidth };
}

function clampPercent(value: number) {
  return Math.max(0, Math.min(100, Math.round(value)));
}

import { initialReports } from '../src/sdk/mockData';
import {
  buildReportChartPoints,
  reportAccuracyColor,
  REPORT_CHART_BOTTOM_GUTTER,
  REPORT_CHART_COMPACT_HEIGHT,
  REPORT_CHART_HEIGHT,
  REPORT_CHART_LEFT_GUTTER,
  REPORT_CHART_RIGHT_GUTTER,
  REPORT_CHART_SEGMENT_THICKNESS,
  REPORT_CHART_STEP_X,
  REPORT_CHART_TOP_GUTTER,
  REPORT_CHART_TOUCH_SIZE,
  REPORT_CHART_VISUAL_DOT_SIZE,
} from '../src/sdk/reportChart';

if (REPORT_CHART_STEP_X !== 56) throw new Error('REPORT_CHART_STEP_X mismatch');
if (REPORT_CHART_LEFT_GUTTER !== 34) throw new Error('REPORT_CHART_LEFT_GUTTER mismatch');
if (REPORT_CHART_RIGHT_GUTTER !== 18) throw new Error('REPORT_CHART_RIGHT_GUTTER mismatch');
if (REPORT_CHART_TOP_GUTTER !== 12) throw new Error('REPORT_CHART_TOP_GUTTER mismatch');
if (REPORT_CHART_BOTTOM_GUTTER !== 22) throw new Error('REPORT_CHART_BOTTOM_GUTTER mismatch');
if (REPORT_CHART_HEIGHT !== 140) throw new Error('REPORT_CHART_HEIGHT mismatch');
if (REPORT_CHART_COMPACT_HEIGHT !== 104) throw new Error('REPORT_CHART_COMPACT_HEIGHT mismatch');
if (REPORT_CHART_TOUCH_SIZE !== 24) throw new Error('Expected report chart touch target to be 24');
if (REPORT_CHART_VISUAL_DOT_SIZE !== 12) throw new Error('Expected report visual dot to be 12');
if (REPORT_CHART_SEGMENT_THICKNESS !== 2) throw new Error('Expected report segment thickness to be 2');

if (reportAccuracyColor(49) !== '#ff3b30') throw new Error('Expected red below 50');
if (reportAccuracyColor(84) !== '#ff9500') throw new Error('Expected orange below 85');
if (reportAccuracyColor(85) !== '#34c759') throw new Error('Expected green at 85');

if (!initialReports.modeSeries?.newWord?.length) {
  throw new Error('Expected ReportsOverview mock data to include modeSeries');
}

const chart = buildReportChartPoints(initialReports.dailySeries.slice(0, 3));
const compactChart = buildReportChartPoints(initialReports.dailySeries.slice(0, 3), true);
if (!(compactChart.chartHeight < chart.chartHeight)) {
  throw new Error('Expected compact report chart to be shorter than the main chart');
}
if (!(chart.points[0].x < chart.points[1].x && chart.points[1].x < chart.points[2].x)) {
  throw new Error('Expected increasing x values');
}
for (const point of chart.points) {
  if (
    point.x < REPORT_CHART_LEFT_GUTTER ||
    point.x > REPORT_CHART_LEFT_GUTTER + chart.plotWidth ||
    point.y < REPORT_CHART_TOP_GUTTER ||
    point.y > REPORT_CHART_TOP_GUTTER + chart.plotHeight
  ) {
    throw new Error(`Expected y inside chart height: ${point.y}`);
  }
  if (point.hitboxLeft + REPORT_CHART_TOUCH_SIZE / 2 !== point.x) {
    throw new Error('Expected report hitbox horizontal center to match chart point');
  }
  if (point.hitboxTop + REPORT_CHART_TOUCH_SIZE / 2 !== point.y) {
    throw new Error('Expected report hitbox vertical center to match chart point');
  }
  if (point.visualCenterX !== point.x || point.visualCenterY !== point.y) {
    throw new Error('Expected report visual dot center to match chart point');
  }
}

for (let index = 1; index < chart.points.length; index += 1) {
  const previous = chart.points[index - 1];
  const point = chart.points[index];
  const dx = point.x - previous.x;
  const dy = point.y - previous.y;
  const length = Math.sqrt(dx * dx + dy * dy);
  const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
  const recomposedEndX = previous.x + Math.cos((angle * Math.PI) / 180) * length;
  const recomposedEndY = previous.y + Math.sin((angle * Math.PI) / 180) * length;
  if (Math.abs(recomposedEndX - point.x) > 0.0001 || Math.abs(recomposedEndY - point.y) > 0.0001) {
    throw new Error('Expected report line segment endpoint to share the same coordinates as the dot center');
  }
}

const expectedSvgPath = chart.points
  .map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`)
  .join(' ');
if (!expectedSvgPath.startsWith(`M ${chart.points[0].x} ${chart.points[0].y}`)) {
  throw new Error('Expected SVG line path to start from the first dot center');
}
if (!expectedSvgPath.endsWith(`L ${chart.points[chart.points.length - 1].x} ${chart.points[chart.points.length - 1].y}`)) {
  throw new Error('Expected SVG line path to end at the last dot center');
}

const selected = chart.points[1].item;
if (selected.date !== initialReports.dailySeries[1].date) {
  throw new Error('Expected chart points to carry source report rows for selection');
}

console.log(JSON.stringify({ points: chart.points.length, modeSeries: Object.keys(initialReports.modeSeries).length }, null, 2));

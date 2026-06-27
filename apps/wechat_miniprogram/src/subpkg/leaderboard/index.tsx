import { useEffect, useMemo, useState } from 'react';
import { Text, View } from '@tarojs/components';

import { Screen } from '@/components/Screen';
import { CrocodileRefreshPopup } from '@/components/CrocodileFrameAnimation';
import { sdk } from '@/sdk';
import { LeaderboardEntry, LeaderboardMetric, LeaderboardSummary } from '@/sdk/types';

import './index.scss';

type MetricOption = { label: string; value: LeaderboardMetric; usesPeriod: boolean };
type PeriodKey = 'weekly' | 'monthly' | 'allTime';

const metricOptions: MetricOption[] = [
  { label: '题数榜', value: 'weekly', usesPeriod: true },
  { label: '正确率榜', value: 'accuracy', usesPeriod: true },
  { label: '混测正确率榜', value: 'mixedAccuracy', usesPeriod: true },
  { label: '连续榜', value: 'streak', usesPeriod: false },
];

const periodOptions: Array<{ label: string; value: PeriodKey }> = [
  { label: '周榜', value: 'weekly' },
  { label: '月榜', value: 'monthly' },
  { label: '总榜', value: 'allTime' },
];

function metricForQuery(metric: LeaderboardMetric, period: PeriodKey): LeaderboardMetric {
  if (metric === 'weekly') return period;
  return metric;
}

function scopeLabel(metric: MetricOption, period: PeriodKey) {
  if (!metric.usesPeriod) return '连续榜 · 每日更新';
  const periodLabel = periodOptions.find((item) => item.value === period)?.label ?? '周榜';
  return `${periodLabel} · ${metric.label}`;
}

export default function LeaderboardPage() {
  const [metric, setMetric] = useState<LeaderboardMetric>('weekly');
  const [period, setPeriod] = useState<PeriodKey>('weekly');
  const [summary, setSummary] = useState<LeaderboardSummary>();
  const [loading, setLoading] = useState(true);

  const selectedMetric = useMemo(
    () => metricOptions.find((option) => option.value === metric) ?? metricOptions[0],
    [metric],
  );

  useEffect(() => {
    setLoading(true);
    sdk.leaderboard
      .getSummary(metricForQuery(metric, period))
      .then(setSummary)
      .finally(() => setLoading(false));
  }, [metric, period]);

  return (
    <Screen title="排行榜" activeTab="today">
      <View className="leaderboard-refresh">
        <CrocodileRefreshPopup active={loading} label="加载排行" />
      </View>

      <View className="leaderboard-tabs">
        {metricOptions.map((option) => (
          <View
            className={`leaderboard-chip ${option.value === metric ? 'leaderboard-chip--active' : ''}`}
            key={option.value}
            onClick={() => setMetric(option.value)}
          >
            <Text>{option.value === metric ? '✓ ' : ''}{option.label}</Text>
          </View>
        ))}
        <View className="leaderboard-chip leaderboard-chip--ghost">
          <Text>图片票选榜</Text>
        </View>
      </View>

      {selectedMetric.usesPeriod && (
        <View className="leaderboard-periods">
          {periodOptions.map((option) => (
            <View
              className={`leaderboard-period ${option.value === period ? 'leaderboard-period--active' : ''}`}
              key={option.value}
              onClick={() => setPeriod(option.value)}
            >
              <Text>{option.label}</Text>
            </View>
          ))}
        </View>
      )}

      <Text className="leaderboard-scope">{scopeLabel(selectedMetric, period)}</Text>

      {summary?.currentUser && <LeaderboardTile entry={summary.currentUser} metric={metric} featured />}

      <View className="leaderboard-list">
        {summary?.entries.map((entry) => (
          <LeaderboardTile key={entry.internalUserId} entry={entry} metric={metric} />
        ))}
      </View>

      {!loading && !summary?.entries.length && (
        <View className="leaderboard-empty">
          <Text>暂无排行榜数据</Text>
        </View>
      )}
    </Screen>
  );
}

function LeaderboardTile({
  entry,
  metric,
  featured = false,
}: {
  entry: LeaderboardEntry;
  metric: LeaderboardMetric;
  featured?: boolean;
}) {
  const initial = entry.displayName.trim().slice(0, 1).toUpperCase() || '?';
  return (
    <View className={`leaderboard-tile ${entry.isCurrentUser ? 'leaderboard-tile--current' : ''} ${featured ? 'leaderboard-tile--featured' : ''}`}>
      <View className="leaderboard-avatar">
        <Text>{initial}</Text>
        <View className="leaderboard-avatar__rank">
          <Text>{entry.rank}</Text>
        </View>
      </View>
      <View className="leaderboard-tile__body">
        <Text className="leaderboard-tile__name">{entry.displayName}</Text>
        <Text className="leaderboard-tile__summary">{summaryText(entry)}</Text>
      </View>
      <Text className="leaderboard-tile__score">{metricValue(entry, metric)}</Text>
    </View>
  );
}

function summaryText(entry: LeaderboardEntry) {
  return `分数 ${entry.score} · ${entry.isCurrentUser ? '当前用户' : '学习者'}`;
}

function metricValue(entry: LeaderboardEntry, metric: LeaderboardMetric) {
  if (metric === 'accuracy' || metric === 'mixedAccuracy') return entry.scoreLabel.includes('%') ? entry.scoreLabel : `${entry.scoreLabel}%`;
  if (metric === 'streak') return entry.scoreLabel.includes('天') ? entry.scoreLabel : `${entry.scoreLabel}天`;
  return entry.scoreLabel;
}

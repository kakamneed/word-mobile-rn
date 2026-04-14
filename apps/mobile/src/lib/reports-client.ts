import {
  getReportsOverview as bridgeGetReportsOverview,
  type DailyStat,
  type ModeBreakdown,
  type ReportsOverview,
  type StreakInfo,
} from './mobile-bridge';

export type {DailyStat, ModeBreakdown, ReportsOverview, StreakInfo};

export async function fetchReportsOverview(): Promise<ReportsOverview> {
  return bridgeGetReportsOverview();
}

export async function fetchDailyStats(
  startDate: string,
  endDate: string,
): Promise<DailyStat[]> {
  const overview = await bridgeGetReportsOverview();
  return overview.last7Days.filter(
    item => item.date >= startDate && item.date <= endDate,
  );
}

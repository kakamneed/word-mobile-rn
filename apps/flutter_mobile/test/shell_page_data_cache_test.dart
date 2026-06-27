import 'package:flutter_mobile/features/shell_page_data_cache.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('today cache reuses in-flight load until invalidated', () async {
    var todayLoads = 0;
    final cache = ShellPageDataCache.forTesting(
      loadToday: () async {
        todayLoads++;
        return TodayHomeState(
          todayDate: '2026-05-19',
          dailyProgress: {'completed': todayLoads},
        );
      },
      loadActivePlan: () async => null,
      loadWordbooks: () async => const <WordbookSummary>[],
      loadTodayAiContext: () async => null,
      loadAiHistory: () async => const <AiPassageHistoryItem>[],
    );

    final first = cache.loadToday();
    final second = cache.loadToday();
    expect(identical(first, second), isTrue);
    expect((await first).dailyProgress['completed'], 1);
    expect((await second).dailyProgress['completed'], 1);
    expect(todayLoads, 1);

    cache.invalidate(ShellPageDataScope.today);
    expect((await cache.loadToday()).dailyProgress['completed'], 2);
    expect(todayLoads, 2);
  });

  test('preload warms plan and ai scopes', () async {
    var activePlanLoads = 0;
    var wordbookLoads = 0;
    var aiContextLoads = 0;
    var aiHistoryLoads = 0;
    final cache = ShellPageDataCache.forTesting(
      loadToday: () async => const TodayHomeState(
        todayDate: '2026-05-19',
        dailyProgress: {},
      ),
      loadActivePlan: () async {
        activePlanLoads++;
        return null;
      },
      loadWordbooks: () async {
        wordbookLoads++;
        return const <WordbookSummary>[];
      },
      loadTodayAiContext: () async {
        aiContextLoads++;
        return null;
      },
      loadAiHistory: () async {
        aiHistoryLoads++;
        return const <AiPassageHistoryItem>[];
      },
    );

    cache.preload(ShellPageDataScope.plan);
    cache.preload(ShellPageDataScope.ai);

    await cache.loadActivePlan();
    await cache.loadWordbooks();
    await cache.loadTodayAiContext();
    await cache.loadAiHistory();

    expect(activePlanLoads, 1);
    expect(wordbookLoads, 1);
    expect(aiContextLoads, 1);
    expect(aiHistoryLoads, 1);
  });
}

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/leaderboard_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('leaderboard renders empty local summary state', (tester) async {
    final bridge = _LeaderboardBridge();

    await tester.pumpWidget(_leaderboardApp(bridge));
    await tester.pumpAndSettle();

    expect(find.textContaining('\u6682\u65e0'), findsOneWidget);
    expect(bridge.calls, contains('getReportsOverview'));
    expect(bridge.calls, contains('getLocalLeaderboard'));
  });

  testWidgets('leaderboard renders local data rows', (tester) async {
    final bridge = _LeaderboardBridge(
      leaderboardEntries: [
        {
          'rank': 1,
          'is_current_user': true,
          'user_id': 'local',
          'display_name': 'Local learner',
          'total_questions': 42,
          'correct_count': 38,
          'accuracy_percent': 90.5,
          'mixed_test_total_questions': 12,
          'mixed_test_correct_count': 10,
          'mixed_test_accuracy_percent': 83.3,
          'current_streak_days': 5,
          'updated_at': '2026-05-14T00:00:00Z',
        },
      ],
    );

    await tester.pumpWidget(_leaderboardApp(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Local learner'), findsOneWidget);
    expect(find.text('42'), findsOneWidget);
  });

  testWidgets('image vote mode renders images and records vote success', (
    tester,
  ) async {
    final bridge = _LeaderboardBridge(
      images: [
        {
          'id': 7,
          'ownerKey': 'local',
          'localPath': 'missing-image.jpg',
          'mimeType': 'image/jpeg',
          'originalFilename': 'reward.jpg',
          'moderationStatus': 'approved',
          'moderationReason': '',
          'isWithdrawn': false,
          'drawPoolEligible': true,
          'createdAt': '2026-05-14T00:00:00Z',
          'updatedAt': '2026-05-14T00:00:00Z',
          'voteCount': 3,
          'selectedAsTag': false,
        },
      ],
    );

    await tester.pumpWidget(_leaderboardApp(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(leaderboardImageVotesKey));
    await tester.pumpAndSettle();

    expect(find.text('reward.jpg'), findsOneWidget);
    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);

    await tester.tap(find.byKey(leaderboardVoteButtonKey));
    await tester.pumpAndSettle();

    expect(bridge.votedImageIds, [7]);
    expect(find.text('\u5df2\u6295\u7968'), findsOneWidget);
    final votedButton = tester.widget<FilledButton>(
      find.byKey(leaderboardVoteButtonKey),
    );
    expect(votedButton.onPressed, isNull);
  });

  testWidgets('image vote failure stays on page and shows failure message', (
    tester,
  ) async {
    final bridge = _LeaderboardBridge(
      failVote: true,
      images: [
        {
          'id': 7,
          'ownerKey': 'local',
          'localPath': 'missing-image.jpg',
          'mimeType': 'image/jpeg',
          'originalFilename': 'reward.jpg',
          'moderationStatus': 'approved',
          'moderationReason': '',
          'isWithdrawn': false,
          'drawPoolEligible': true,
          'createdAt': '2026-05-14T00:00:00Z',
          'updatedAt': '2026-05-14T00:00:00Z',
          'voteCount': 3,
          'selectedAsTag': false,
        },
      ],
    );

    await tester.pumpWidget(_leaderboardApp(bridge));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(leaderboardImageVotesKey));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(leaderboardVoteButtonKey));
    await tester.pump();

    expect(find.textContaining('\u6295\u7968\u5931\u8d25'), findsOneWidget);
    expect(find.text('reward.jpg'), findsOneWidget);
  });

  test('upload entitlement and upload failures surface bridge errors', () async {
    final bridge = _LeaderboardBridge(failUpload: true);
    final sdk = WordSdk.bridgeForTesting(bridge: bridge);

    await expectLater(
      sdk.rewardImages.getUploadEntitlement(),
      throwsA(isA<BridgeError>().having((e) => e.code, 'code', 'NO_UPLOADS')),
    );
    await expectLater(
      sdk.rewardImages.createUpload(localPath: 'missing.jpg'),
      throwsA(isA<BridgeError>().having((e) => e.code, 'code', 'UPLOAD_FAILED')),
    );
  });
}

Widget _leaderboardApp(_LeaderboardBridge bridge) {
  return MaterialApp(
    home: LeaderboardScreen(sdk: WordSdk.bridgeForTesting(bridge: bridge)),
  );
}

class _LeaderboardBridge extends RustBridge {
  _LeaderboardBridge({
    this.leaderboardEntries = const [],
    this.images = const [],
    this.failVote = false,
    this.failUpload = false,
  });

  final List<Map<String, dynamic>> leaderboardEntries;
  final List<Map<String, dynamic>> images;
  final bool failVote;
  final bool failUpload;
  final calls = <String>[];
  final votedImageIds = <int>[];

  @override
  Future<String> call(String method, [String? argument]) async {
    calls.add(method);
    switch (method) {
      case 'getReportsOverview':
        return jsonEncode({
          'totalStudyDays': 1,
          'totalWordsLearned': 2,
          'totalQuestionsAnswered': 10,
          'overallAccuracy': 80.0,
          'streakInfo': {'currentStreak': 2},
          'modeBreakdown': [
            {'mode': 'mixedTest', 'totalQuestions': 4, 'correctCount': 3},
          ],
          'last7Days': [],
          'dailySeries': [],
          'modeSeries': {},
        });
      case 'refreshLocalLeaderboardSummary':
        return jsonEncode({
          'rank': 1,
          'is_current_user': true,
          'user_id': 'local',
          'display_name': 'Local learner',
          'total_questions': 10,
          'correct_count': 8,
          'accuracy_percent': 80.0,
          'mixed_test_total_questions': 4,
          'mixed_test_correct_count': 3,
          'mixed_test_accuracy_percent': 75.0,
          'current_streak_days': 2,
          'updated_at': '2026-05-14T00:00:00Z',
        });
      case 'getLocalLeaderboard':
        return jsonEncode({'entries': leaderboardEntries});
      case 'listRewardImages':
        return jsonEncode({'images': images});
      case 'voteRewardImage':
        if (failVote) {
          throw BridgeError.domain('VOTE_FAILED', 'duplicate weekly vote');
        }
        final payload = jsonDecode(argument ?? '{}') as Map<String, dynamic>;
        votedImageIds.add((payload['imageId'] as num).toInt());
        return jsonEncode({
          'imageId': payload['imageId'],
          'weekStart': '2026-05-11',
          'inserted': true,
          'voteCount': 4,
        });
      case 'getRewardImageUploadEntitlement':
        if (failUpload) {
          throw BridgeError.domain('NO_UPLOADS', 'no upload entitlement');
        }
        return jsonEncode({
          'ownerKey': 'local',
          'availableUploads': 1,
          'lastGrantedStreakMilestone': 5,
          'nextMilestoneStreakDays': 10,
        });
      case 'createRewardImageUpload':
        if (failUpload) {
          throw BridgeError.runtime('UPLOAD_FAILED', 'file not found');
        }
        return jsonEncode(images.first);
      default:
        return '{}';
    }
  }

  @override
  Future<void> callVoid(String method, [String? argument]) async {}

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> isAvailable() async => true;
}

library;

import '../bridge/bridge.dart';

class RewardImageUploadEntitlement {
  const RewardImageUploadEntitlement({
    required this.ownerKey,
    required this.availableUploads,
    required this.lastGrantedStreakMilestone,
    required this.nextMilestoneStreakDays,
    this.updatedAt,
  });

  final String ownerKey;
  final int availableUploads;
  final int lastGrantedStreakMilestone;
  final int nextMilestoneStreakDays;
  final String? updatedAt;

  factory RewardImageUploadEntitlement.fromJson(Map<String, dynamic> json) {
    return RewardImageUploadEntitlement(
      ownerKey: json['ownerKey'] as String? ?? 'local',
      availableUploads: (json['availableUploads'] as num?)?.toInt() ?? 0,
      lastGrantedStreakMilestone:
          (json['lastGrantedStreakMilestone'] as num?)?.toInt() ?? 0,
      nextMilestoneStreakDays:
          (json['nextMilestoneStreakDays'] as num?)?.toInt() ?? 5,
      updatedAt: json['updatedAt'] as String?,
    );
  }
}

class RewardImage {
  const RewardImage({
    required this.id,
    required this.ownerKey,
    required this.localPath,
    required this.mimeType,
    required this.originalFilename,
    required this.moderationStatus,
    required this.moderationReason,
    required this.isWithdrawn,
    required this.drawPoolEligible,
    required this.createdAt,
    required this.updatedAt,
    required this.voteCount,
    required this.selectedAsTag,
    this.moderationCheckedAt,
  });

  final int id;
  final String ownerKey;
  final String localPath;
  final String mimeType;
  final String originalFilename;
  final String moderationStatus;
  final String moderationReason;
  final String? moderationCheckedAt;
  final bool isWithdrawn;
  final bool drawPoolEligible;
  final String createdAt;
  final String updatedAt;
  final int voteCount;
  final bool selectedAsTag;

  bool get isApproved => moderationStatus == 'approved';

  factory RewardImage.fromJson(Map<String, dynamic> json) {
    return RewardImage(
      id: (json['id'] as num?)?.toInt() ?? 0,
      ownerKey: json['ownerKey'] as String? ?? 'local',
      localPath: json['localPath'] as String? ?? '',
      mimeType: json['mimeType'] as String? ?? 'image/jpeg',
      originalFilename: json['originalFilename'] as String? ?? '',
      moderationStatus: json['moderationStatus'] as String? ?? 'pending',
      moderationReason: json['moderationReason'] as String? ?? '',
      moderationCheckedAt: json['moderationCheckedAt'] as String?,
      isWithdrawn: json['isWithdrawn'] as bool? ?? false,
      drawPoolEligible: json['drawPoolEligible'] as bool? ?? false,
      createdAt: json['createdAt'] as String? ?? '',
      updatedAt: json['updatedAt'] as String? ?? '',
      voteCount: (json['voteCount'] as num?)?.toInt() ?? 0,
      selectedAsTag: json['selectedAsTag'] as bool? ?? false,
    );
  }
}

class RewardImageVoteResult {
  const RewardImageVoteResult({
    required this.imageId,
    required this.weekStart,
    required this.inserted,
    required this.voteCount,
  });

  final int imageId;
  final String weekStart;
  final bool inserted;
  final int voteCount;

  factory RewardImageVoteResult.fromJson(Map<String, dynamic> json) {
    return RewardImageVoteResult(
      imageId: (json['imageId'] as num?)?.toInt() ?? 0,
      weekStart: json['weekStart'] as String? ?? '',
      inserted: json['inserted'] as bool? ?? false,
      voteCount: (json['voteCount'] as num?)?.toInt() ?? 0,
    );
  }
}

class LocalLeaderboardEntry {
  const LocalLeaderboardEntry({
    required this.rank,
    required this.isCurrentUser,
    required this.userId,
    required this.displayName,
    required this.totalQuestions,
    required this.correctCount,
    required this.accuracyPercent,
    required this.mixedTestTotalQuestions,
    required this.mixedTestCorrectCount,
    required this.mixedTestAccuracyPercent,
    required this.currentStreakDays,
    required this.updatedAt,
    this.avatarUrl,
    this.tagImage,
  });

  final int rank;
  final bool isCurrentUser;
  final String userId;
  final String displayName;
  final int totalQuestions;
  final int correctCount;
  final double accuracyPercent;
  final int mixedTestTotalQuestions;
  final int mixedTestCorrectCount;
  final double mixedTestAccuracyPercent;
  final int currentStreakDays;
  final String updatedAt;
  final String? avatarUrl;
  final RewardImage? tagImage;

  factory LocalLeaderboardEntry.fromJson(Map<String, dynamic> json) {
    final tagImageJson = json['tag_image'];
    return LocalLeaderboardEntry(
      rank: (json['rank'] as num?)?.toInt() ?? 0,
      isCurrentUser: json['is_current_user'] as bool? ?? false,
      userId: json['user_id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? 'Local learner',
      totalQuestions: (json['total_questions'] as num?)?.toInt() ?? 0,
      correctCount: (json['correct_count'] as num?)?.toInt() ?? 0,
      accuracyPercent: (json['accuracy_percent'] as num?)?.toDouble() ?? 0,
      mixedTestTotalQuestions:
          (json['mixed_test_total_questions'] as num?)?.toInt() ?? 0,
      mixedTestCorrectCount:
          (json['mixed_test_correct_count'] as num?)?.toInt() ?? 0,
      mixedTestAccuracyPercent:
          (json['mixed_test_accuracy_percent'] as num?)?.toDouble() ?? 0,
      currentStreakDays: (json['current_streak_days'] as num?)?.toInt() ?? 0,
      updatedAt: json['updated_at'] as String? ?? '',
      avatarUrl: _nonEmptyString(json['avatar_url']),
      tagImage: tagImageJson is Map<String, dynamic>
          ? RewardImage.fromJson(tagImageJson)
          : null,
    );
  }

  static String? _nonEmptyString(Object? value) {
    final text = '${value ?? ''}'.trim();
    return text.isEmpty ? null : text;
  }
}

class RewardImageClient {
  const RewardImageClient(this._bridge, this._codec);

  final RustBridge _bridge;
  final BridgeCodec _codec;

  Future<RewardImageUploadEntitlement> getUploadEntitlement() async {
    final raw = await _bridge.call('getRewardImageUploadEntitlement');
    return RewardImageUploadEntitlement.fromJson(_codec.decodeResponse(raw));
  }

  Future<RewardImageUploadEntitlement> refreshUploadEntitlement({
    required int currentStreakDays,
    int maxAvailableUploads = 3,
  }) async {
    final raw = await _bridge.call(
      'refreshRewardImageUploadEntitlement',
      _codec.encodeRequest({
        'currentStreakDays': currentStreakDays,
        'maxAvailableUploads': maxAvailableUploads,
      }),
    );
    return RewardImageUploadEntitlement.fromJson(_codec.decodeResponse(raw));
  }

  Future<RewardImage> createUpload({
    required String localPath,
    String mimeType = 'image/jpeg',
    String originalFilename = '',
  }) async {
    final raw = await _bridge.call(
      'createRewardImageUpload',
      _codec.encodeRequest({
        'localPath': localPath,
        'mimeType': mimeType,
        'originalFilename': originalFilename,
      }),
    );
    return RewardImage.fromJson(_codec.decodeResponse(raw));
  }

  Future<List<RewardImage>> listImages({
    bool publicOnly = false,
    String? weekStart,
  }) async {
    final request = <String, dynamic>{'publicOnly': publicOnly};
    if (weekStart != null) {
      request['weekStart'] = weekStart;
    }
    final raw = await _bridge.call(
      'listRewardImages',
      _codec.encodeRequest(request),
    );
    final json = _codec.decodeResponse(raw);
    return (json['images'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(RewardImage.fromJson)
        .toList(growable: false);
  }

  Future<RewardImage> moderateImage({
    required int imageId,
    required String status,
    String reason = '',
  }) async {
    final raw = await _bridge.call(
      'moderateRewardImage',
      _codec.encodeRequest({
        'imageId': imageId,
        'status': status,
        'reason': reason,
      }),
    );
    return RewardImage.fromJson(_codec.decodeResponse(raw));
  }

  Future<RewardImage> selectLeaderboardTag({required int imageId}) async {
    final raw = await _bridge.call(
      'selectLeaderboardRewardImageTag',
      _codec.encodeRequest({'imageId': imageId}),
    );
    return RewardImage.fromJson(_codec.decodeResponse(raw));
  }

  Future<RewardImageVoteResult> vote({
    required int imageId,
    String? weekStart,
  }) async {
    final request = <String, dynamic>{'imageId': imageId};
    if (weekStart != null) {
      request['weekStart'] = weekStart;
    }
    final raw = await _bridge.call(
      'voteRewardImage',
      _codec.encodeRequest(request),
    );
    return RewardImageVoteResult.fromJson(_codec.decodeResponse(raw));
  }

  Future<List<LocalLeaderboardEntry>> getLocalLeaderboard({
    required String metric,
    required String period,
    required String periodStart,
    int limit = 50,
  }) async {
    final raw = await _bridge.call(
      'getLocalLeaderboard',
      _codec.encodeRequest({
        'metric': metric,
        'period': period,
        'periodStart': periodStart,
        'limit': limit,
      }),
    );
    final json = _codec.decodeResponse(raw);
    return (json['entries'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(LocalLeaderboardEntry.fromJson)
        .toList(growable: false);
  }

  Future<LocalLeaderboardEntry> refreshLocalSummary({
    required String userKey,
    required String displayName,
    required String metricPeriod,
    required String periodStart,
    required int totalQuestions,
    required int correctCount,
    required int mixedTestTotalQuestions,
    required int mixedTestCorrectCount,
    required int currentStreakDays,
  }) async {
    final raw = await _bridge.call(
      'refreshLocalLeaderboardSummary',
      _codec.encodeRequest({
        'userKey': userKey,
        'displayName': displayName,
        'period': metricPeriod,
        'periodStart': periodStart,
        'totalQuestions': totalQuestions,
        'correctCount': correctCount,
        'mixedTestTotalQuestions': mixedTestTotalQuestions,
        'mixedTestCorrectCount': mixedTestCorrectCount,
        'currentStreakDays': currentStreakDays,
      }),
    );
    return LocalLeaderboardEntry.fromJson(_codec.decodeResponse(raw));
  }

  Future<Map<String, dynamic>> seedLocalDemo() async {
    final raw = await _bridge.call('seedLocalLeaderboardDemo');
    return _codec.decodeResponse(raw);
  }
}

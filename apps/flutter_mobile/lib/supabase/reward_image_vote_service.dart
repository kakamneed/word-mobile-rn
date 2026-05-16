library;

import 'supabase_auth_service.dart';
import 'supabase_config.dart';

class CloudRewardImageEntry {
  const CloudRewardImageEntry({
    required this.rank,
    required this.imageId,
    required this.ownerId,
    required this.displayName,
    required this.storagePath,
    required this.publicUrl,
    required this.originalFilename,
    required this.mimeType,
    required this.voteCount,
    required this.votedByMe,
    required this.selectedAsTag,
    required this.createdAt,
  });

  final int rank;
  final String imageId;
  final String ownerId;
  final String displayName;
  final String storagePath;
  final String publicUrl;
  final String originalFilename;
  final String mimeType;
  final int voteCount;
  final bool votedByMe;
  final bool selectedAsTag;
  final DateTime? createdAt;

  CloudRewardImageEntry copyWith({
    int? voteCount,
    bool? votedByMe,
  }) {
    return CloudRewardImageEntry(
      rank: rank,
      imageId: imageId,
      ownerId: ownerId,
      displayName: displayName,
      storagePath: storagePath,
      publicUrl: publicUrl,
      originalFilename: originalFilename,
      mimeType: mimeType,
      voteCount: voteCount ?? this.voteCount,
      votedByMe: votedByMe ?? this.votedByMe,
      selectedAsTag: selectedAsTag,
      createdAt: createdAt,
    );
  }

  factory CloudRewardImageEntry.fromJson(Map<String, dynamic> json) {
    return CloudRewardImageEntry(
      rank: (json['rank'] as num?)?.toInt() ?? 0,
      imageId: json['image_id'] as String? ?? '',
      ownerId: json['owner_id'] as String? ?? '',
      displayName: json['display_name'] as String? ?? 'Word learner',
      storagePath: json['storage_path'] as String? ?? '',
      publicUrl: json['public_url'] as String? ?? '',
      originalFilename: json['original_filename'] as String? ?? '',
      mimeType: json['mime_type'] as String? ?? 'image/jpeg',
      voteCount: (json['vote_count'] as num?)?.toInt() ?? 0,
      votedByMe: json['voted_by_me'] as bool? ?? false,
      selectedAsTag: json['selected_as_tag'] as bool? ?? false,
      createdAt: DateTime.tryParse('${json['created_at'] ?? ''}'),
    );
  }
}

class CloudRewardImageVoteResult {
  const CloudRewardImageVoteResult({
    required this.imageId,
    required this.weekStart,
    required this.inserted,
    required this.voteCount,
  });

  final String imageId;
  final String weekStart;
  final bool inserted;
  final int voteCount;

  factory CloudRewardImageVoteResult.fromJson(Map<String, dynamic> json) {
    return CloudRewardImageVoteResult(
      imageId: json['image_id'] as String? ?? '',
      weekStart: json['week_start'] as String? ?? '',
      inserted: json['inserted'] as bool? ?? false,
      voteCount: (json['vote_count'] as num?)?.toInt() ?? 0,
    );
  }
}

class RewardImageVoteService {
  RewardImageVoteService({SupabaseAuthService? authService})
      : _authService = authService ?? SupabaseAuthService();

  final SupabaseAuthService _authService;

  bool get isConfigured => SupabaseConfig.isConfigured;

  Future<bool> get isSignedIn async {
    if (!isConfigured) return false;
    await _authService.ensureInitialized();
    return _authService.client.auth.currentSession != null;
  }

  Future<List<CloudRewardImageEntry>> fetchVoteLeaderboard({
    required String weekStart,
    int limit = 50,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase is not configured.');
    }
    await _authService.ensureInitialized();
    final response = await _authService.client.rpc(
      'get_reward_image_vote_leaderboard',
      params: {
        'p_week_start': weekStart,
        'p_limit': limit,
      },
    );
    final rows = response as List<dynamic>? ?? const [];
    return rows
        .whereType<Map>()
        .map((row) => CloudRewardImageEntry.fromJson(row.cast<String, dynamic>()))
        .toList(growable: false);
  }

  Future<CloudRewardImageVoteResult> vote({
    required String imageId,
    required String weekStart,
  }) async {
    if (!isConfigured) {
      throw StateError('Supabase is not configured.');
    }
    await _authService.ensureInitialized();
    final response = await _authService.client.rpc(
      'vote_reward_image',
      params: {
        'p_image_id': imageId,
        'p_week_start': weekStart,
      },
    );
    final rows = response as List<dynamic>? ?? const [];
    final matchingRows = rows.whereType<Map>().cast<Map>();
    final row = matchingRows.isEmpty ? null : matchingRows.first;
    if (row == null) {
      throw StateError('vote_reward_image returned no rows.');
    }
    return CloudRewardImageVoteResult.fromJson(row.cast<String, dynamic>());
  }
}

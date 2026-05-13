/// Sync client - read-only SDK for Rust-owned sync queue status.
library;

import 'dart:convert';

import '../cloud/cloud_backend_config.dart';
import 'local_data_owner_client.dart';
import '../supabase/supabase_auth_service.dart';
import '../supabase/word_admin_auth_service.dart';
import '../bridge/bridge.dart';

class SyncOutboxItem {
  final int id;
  final String domain;
  final String payloadJson;
  final String idempotencyKey;
  final String createdAt;
  final int attemptCount;
  final String status;

  const SyncOutboxItem({
    required this.id,
    required this.domain,
    required this.payloadJson,
    required this.idempotencyKey,
    required this.createdAt,
    required this.attemptCount,
    required this.status,
  });

  factory SyncOutboxItem.fromJson(Map<String, dynamic> json) => SyncOutboxItem(
    id: json['id'] as int,
    domain: json['domain'] as String,
    payloadJson: json['payloadJson'] as String,
    idempotencyKey: json['idempotencyKey'] as String,
    createdAt: json['createdAt'] as String,
    attemptCount: json['attemptCount'] as int? ?? 0,
    status: json['status'] as String? ?? 'pending',
  );
}

class SyncDomainPendingCount {
  final String domain;
  final int pendingCount;

  const SyncDomainPendingCount({
    required this.domain,
    required this.pendingCount,
  });

  factory SyncDomainPendingCount.fromJson(Map<String, dynamic> json) =>
      SyncDomainPendingCount(
        domain: json['domain'] as String,
        pendingCount: json['pendingCount'] as int,
      );
}

class SyncStatus {
  final bool syncEnabled;
  final bool transportConfigured;
  final String accountSyncState;
  final int pendingCount;
  final String? lastSyncSucceededAt;
  final String? lastSyncErrorCode;
  final List<SyncDomainPendingCount> domainsPending;
  final List<SyncOutboxItem> pendingItems;
  final Map<String, dynamic>? lastCloudRestore;
  final Map<String, dynamic>? localCloudRestoreDiagnostics;

  const SyncStatus({
    required this.syncEnabled,
    required this.transportConfigured,
    required this.accountSyncState,
    required this.pendingCount,
    this.lastSyncSucceededAt,
    this.lastSyncErrorCode,
    this.domainsPending = const [],
    this.pendingItems = const [],
    this.lastCloudRestore,
    this.localCloudRestoreDiagnostics,
  });

  factory SyncStatus.fromJson(Map<String, dynamic> json) => SyncStatus(
    syncEnabled: json['syncEnabled'] as bool? ?? false,
    transportConfigured: json['transportConfigured'] as bool? ?? false,
    accountSyncState: json['accountSyncState'] as String? ?? 'unknown',
    pendingCount: json['pendingCount'] as int? ?? 0,
    lastSyncSucceededAt: json['lastSyncSucceededAt'] as String?,
    lastSyncErrorCode: json['lastSyncErrorCode'] as String?,
    domainsPending: (json['domainsPending'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(SyncDomainPendingCount.fromJson)
        .toList(growable: false),
    pendingItems: (json['pendingItems'] as List<dynamic>? ?? const [])
        .whereType<Map<String, dynamic>>()
        .map(SyncOutboxItem.fromJson)
        .toList(growable: false),
    lastCloudRestore: json['lastCloudRestore'] is Map
        ? Map<String, dynamic>.from(json['lastCloudRestore'] as Map)
        : null,
    localCloudRestoreDiagnostics: json['localCloudRestoreDiagnostics'] is Map
        ? Map<String, dynamic>.from(json['localCloudRestoreDiagnostics'] as Map)
        : null,
  );
}

class SyncClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;
  final SupabaseAuthService _authService;
  final WordAdminAuthService _wordAdminAuthService;

  SyncClient(
    this._bridge,
    this._codec, {
    SupabaseAuthService? authService,
    WordAdminAuthService? wordAdminAuthService,
  }) : _authService = authService ?? SupabaseAuthService(),
       _wordAdminAuthService = wordAdminAuthService ?? WordAdminAuthService();

  Future<SyncStatus> getSyncStatus() async {
    final raw = await _bridge.call('getSyncStatus');
    final json = _codec.decodeResponse(raw);
    return SyncStatus.fromJson(json);
  }

  Future<SyncStatus> flushPendingToCloud() async {
    final initialStatus = await getSyncStatus();
    if (initialStatus.pendingItems.isEmpty) return initialStatus;

    if (CloudBackendConfig.usesWordAdmin) {
      return _flushPendingToWordAdmin(initialStatus);
    }

    await _authService.ensureInitialized();
    final client = _authService.client;
    final userId = client.auth.currentUser?.id;
    if (userId == null || userId.isEmpty) {
      return initialStatus;
    }

    for (final item in initialStatus.pendingItems) {
      try {
        if (item.domain == 'plan_config') {
          await _uploadPlanConfig(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'wordbook_preferences') {
          await _uploadWordbookPreferences(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'report_snapshot') {
          await _uploadReportSnapshot(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'study_word_points') {
          await _uploadStudyWordPoints(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'wrong_word_entries') {
          await _uploadWrongWordEntries(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'ai_passages') {
          await _uploadAiPassage(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else if (item.domain == 'croc_bti_profile') {
          await _uploadCrocBtiProfile(userId: userId, item: item);
          await _recordSyncResult(item.id, succeeded: true);
        } else {
          await _recordSyncResult(
            item.id,
            succeeded: false,
            failureCode: 'unsupported_domain',
            failureMessage: 'Unsupported sync domain: ${item.domain}',
          );
        }
      } catch (error) {
        await _recordSyncResult(
          item.id,
          succeeded: false,
          failureCode: '${item.domain}_upload_failed',
          failureMessage: error.toString(),
        );
      }
    }

    return getSyncStatus();
  }

  Future<SyncStatus> _flushPendingToWordAdmin(SyncStatus initialStatus) async {
    for (final item in initialStatus.pendingItems) {
      try {
        final payload = jsonDecode(item.payloadJson);
        if (payload is! Map<String, dynamic>) {
          throw const FormatException('Expected sync payload object');
        }
        await _wordAdminAuthService.flushSyncItem(
          domain: item.domain,
          payload: payload,
        );
        await _recordSyncResult(item.id, succeeded: true);
      } catch (error) {
        await _recordSyncResult(
          item.id,
          succeeded: false,
          failureCode: '${item.domain}_word_admin_upload_failed',
          failureMessage: error.toString(),
        );
      }
    }

    return getSyncStatus();
  }

  Future<SyncStatus> backfillLocalLearningToCloud({
    int windowDays = 365,
  }) async {
    await _bridge.call(
      'enqueueCloudBackfill',
      _codec.encodeRequest({'windowDays': windowDays}),
    );
    return flushPendingToCloud();
  }

  /// Fetch AI passage rows directly from Supabase, bypassing the local sync
  /// queue. Used when local history is empty but cloud data may exist.
  Future<List<Map<String, dynamic>>> fetchCloudAiPassages() async {
    if (!CloudBackendConfig.usesWordAdmin) {
      await _authService.ensureInitialized();
      final userId = _authService.client.auth.currentUser?.id;
      if (userId == null || userId.isEmpty) return const [];
      try {
        final rows = await _authService.client
            .from('ai_passages')
            .select('passage_id,title,payload_json,validation_status,generated_at')
            .eq('user_id', userId)
            .order('generated_at', ascending: false);
        return _mapList(rows);
      } catch (_) {
        return const [];
      }
    }
    // WordAdmin path
    try {
      final snapshot = await _wordAdminAuthService.getSyncSnapshot();
      return _mapList(snapshot['aiPassages']);
    } catch (_) {
      return const [];
    }
  }

  Future<int> restoreCloudAiPassagesToLocal(
    List<Map<String, dynamic>> rows,
  ) async {
    if (rows.isEmpty) return 0;
    final raw = await _bridge.call(
      'restoreCloudAiPassageSnapshot',
      _codec.encodeRequest({'aiPassages': rows}),
    );
    final result = _codec.decodeResponse(raw);
    return result['restoredAiPassages'] as int? ?? 0;
  }

  Future<bool> shouldRestoreCloudData(
    String userId,
    LocalDataOwnerResult ownerResult,
  ) async {
    if (!CloudBackendConfig.usesWordAdmin) {
      return !ownerResult.restoredSnapshot &&
          (ownerResult.resetPerformed || !ownerResult.hasLocalLearningData);
    }

    final snapshot = await _wordAdminAuthService.getSyncSnapshot();
    final remoteSummary = _snapshotSummary(snapshot);
    return remoteSummary.totalRows > 0;
  }

  Future<void> restoreCloudDataToLocal(String userId) async {
    final normalizedUserId = userId.trim();
    if (normalizedUserId.isEmpty) return;

    try {
      if (CloudBackendConfig.usesWordAdmin) {
        await _restoreWordAdminDataToLocal(normalizedUserId);
        return;
      }
      await _authService.ensureInitialized();
      final planRows = await _authService.client
          .from('plan_configs')
          .select()
          .eq('user_id', normalizedUserId)
          .limit(1);
      final wordbookRows = await _authService.client
          .from('wordbook_preferences')
          .select('wordbook_id,is_active')
          .eq('user_id', normalizedUserId);
      final pointRows = await _authService.client
          .from('study_word_points')
          .select(
            'point_date,entry_id,mode,question_type,attempt_count,correct_count,wrong_count,total_response_time_ms,last_answered_at',
          )
          .eq('user_id', normalizedUserId)
          .order('point_date');
      final reportRows = await _authService.client
          .from('report_snapshots')
          .select('snapshot_date,payload_json')
          .eq('user_id', normalizedUserId)
          .order('snapshot_date');
      final wrongWordRows = await _authService.client
          .from('wrong_word_entries')
          .select(
            'entry_id,error_count,last_wrong_at,priority_score,hint_text,hint_source,hint_updated_at',
          )
          .eq('user_id', normalizedUserId);
      final aiPassageRows = await _authService.client
          .from('ai_passages')
          .select('passage_id,title,payload_json,validation_status,generated_at')
          .eq('user_id', normalizedUserId)
          .order('generated_at', ascending: false);
      final crocBtiRows = await _authService.client
          .from('croc_bti_profiles')
          .select('*')
          .eq('user_id', normalizedUserId)
          .limit(1);

      final planList = _mapList(planRows);
      final wordbookList = _mapList(wordbookRows);
      final pointList = _mapList(pointRows);
      final reportList = _mapList(reportRows);
      final wrongWordList = _mapList(wrongWordRows);
      final aiPassageList = _mapList(aiPassageRows);
      final crocBtiProfile = crocBtiRows.isNotEmpty
          ? crocBtiRows.first
          : null;
      final request = {
        'userId': normalizedUserId,
        'planConfig': planList.isEmpty ? null : planList.first,
        'wordbookPreferences': wordbookList,
        'studyWordPoints': pointList,
        'reportSnapshots': reportList,
        'wrongWordEntries': wrongWordList,
        'aiPassages': aiPassageList,
        'crocBtiProfile': crocBtiProfile is Map<String, dynamic>
            ? crocBtiProfile
            : null,
      };
      final raw = await _bridge.call(
        'restoreCloudDataSnapshot',
        _codec.encodeRequest(request),
      );
      final result = _codec.decodeResponse(raw);
      await _recordCloudRestoreAttempt({
        'userId': normalizedUserId,
        'succeeded': true,
        'planRows': planList.length,
        'wordbookRows': wordbookList.length,
        'studyPointRows': pointList.length,
        'reportSnapshotRows': reportList.length,
        'wrongWordRows': wrongWordList.length,
        'aiPassageRows': aiPassageList.length,
        'crocBtiProfileRows': crocBtiProfile is Map ? 1 : 0,
        'restoredStudyPoints': result['restoredStudyPoints'] as int? ?? 0,
        'restoredReportSnapshots':
            result['restoredReportSnapshots'] as int? ?? 0,
        'restoredWordHints': result['restoredWordHints'] as int? ?? 0,
        'restoredAiPassages': result['restoredAiPassages'] as int? ?? 0,
        'restoredCrocBtiProfile':
            result['restoredCrocBtiProfile'] as bool? ?? false,
      });
    } catch (error) {
      await _recordCloudRestoreAttempt({
        'userId': normalizedUserId,
        'succeeded': false,
        'error': error.toString(),
      });
      rethrow;
    }
  }

  Future<void> _restoreWordAdminDataToLocal(String userId) async {
    final snapshot = await _wordAdminAuthService.getSyncSnapshot();
    final plan = snapshot['planConfig'];
    final wordbookRows = _mapList(snapshot['wordbookPreferences']);
    final pointRows = _mapList(snapshot['studyWordPoints']);
    final reportRows = _mapList(snapshot['reportSnapshots']);
    final wrongWordRows = _mapList(snapshot['wrongWordEntries']);
    final aiPassageRows = _mapList(snapshot['aiPassages']);
    final crocBtiProfile = snapshot['crocBtiProfile'];
    final request = {
      'userId': userId,
      'planConfig': plan is Map<String, dynamic> ? plan : null,
      'wordbookPreferences': wordbookRows,
      'studyWordPoints': pointRows,
      'reportSnapshots': reportRows,
      'wrongWordEntries': wrongWordRows,
      'aiPassages': aiPassageRows,
      'crocBtiProfile': crocBtiProfile is Map<String, dynamic>
          ? crocBtiProfile
          : null,
    };
    final raw = await _bridge.call(
      'restoreCloudDataSnapshot',
      _codec.encodeRequest(request),
    );
    final result = _codec.decodeResponse(raw);
    await _recordCloudRestoreAttempt({
      'userId': userId,
      'succeeded': true,
      'planRows': plan is Map<String, dynamic> ? 1 : 0,
      'wordbookRows': wordbookRows.length,
      'studyPointRows': pointRows.length,
      'reportSnapshotRows': reportRows.length,
      'wrongWordRows': wrongWordRows.length,
      'aiPassageRows': aiPassageRows.length,
      'crocBtiProfileRows': crocBtiProfile is Map<String, dynamic> ? 1 : 0,
      'restoredStudyPoints': result['restoredStudyPoints'] as int? ?? 0,
      'restoredReportSnapshots': result['restoredReportSnapshots'] as int? ?? 0,
      'restoredWordHints': result['restoredWordHints'] as int? ?? 0,
      'restoredAiPassages': result['restoredAiPassages'] as int? ?? 0,
      'restoredCrocBtiProfile':
          result['restoredCrocBtiProfile'] as bool? ?? false,
    });
  }

  _WordAdminSnapshotSummary _snapshotSummary(Map<String, dynamic> snapshot) {
    final wordbookRows = _mapList(snapshot['wordbookPreferences']).length;
    final pointRows = _mapList(snapshot['studyWordPoints']).length;
    final reportRows = _mapList(snapshot['reportSnapshots']).length;
    final wrongWordRows = _mapList(snapshot['wrongWordEntries']).length;
    final aiPassageRows = _mapList(snapshot['aiPassages']).length;
    final crocBtiProfileRows = snapshot['crocBtiProfile'] is Map ? 1 : 0;
    return _WordAdminSnapshotSummary(
      wordbookRows: wordbookRows,
      studyPointRows: pointRows,
      reportSnapshotRows: reportRows,
      wrongWordRows: wrongWordRows,
      aiPassageRows: aiPassageRows,
      crocBtiProfileRows: crocBtiProfileRows,
    );
  }

  Future<void> _uploadPlanConfig({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected plan_config payload object');
    }
    final plan = payload['plan'];
    if (plan is! Map<String, dynamic>) {
      throw const FormatException('Expected plan_config plan object');
    }

    await _authService.client.from('plan_configs').upsert({
      'user_id': userId,
      'name': plan['name'] as String? ?? 'Default plan',
      'new_words_per_day': _intValue(plan['newWordsPerDay']),
      'review_words_per_day': _intValue(plan['reviewWordsPerDay']),
      'mixed_test_per_day': _intValue(plan['mixedTestPerDay']),
      'wrong_word_test_per_day': _intValue(plan['wrongWordTestPerDay']),
      'root_affix_per_day': _nullableIntValue(plan['rootAffixPerDay']),
      'growth_rule_mode': plan['growthRuleMode'] as String?,
      'shared_growth_rule': plan['sharedGrowthRule'],
      'growth_rules_by_mode': plan['growthRulesByMode'],
      'version': DateTime.now().millisecondsSinceEpoch,
    }, onConflict: 'user_id');
  }

  Future<void> _uploadWordbookPreferences({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException(
        'Expected wordbook_preferences payload object',
      );
    }
    final selection = payload['selection'];
    if (selection is! Map<String, dynamic>) {
      throw const FormatException('Expected wordbook selection object');
    }
    final rows = selection.entries
        .map((entry) {
          return {
            'user_id': userId,
            'wordbook_id': int.tryParse(entry.key) ?? 0,
            'is_active': entry.value == true,
          };
        })
        .where((row) => (row['wordbook_id'] as int) > 0)
        .toList(growable: false);
    if (rows.isEmpty) return;
    await _authService.client
        .from('wordbook_preferences')
        .upsert(rows, onConflict: 'user_id,wordbook_id');
  }

  Future<void> _uploadReportSnapshot({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected report_snapshot payload object');
    }
    final snapshotDate = payload['snapshotDate'] as String?;
    final overview = payload['overview'];
    if (snapshotDate == null || overview is! Map<String, dynamic>) {
      throw const FormatException('Expected report snapshot date and overview');
    }
    await _authService.client.from('report_snapshots').upsert({
      'user_id': userId,
      'snapshot_date': snapshotDate,
      'payload_json': overview,
    }, onConflict: 'user_id,snapshot_date');
  }

  Future<void> _uploadStudyWordPoints({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected study_word_points payload object');
    }
    final points = payload['points'];
    if (points is! List<dynamic>) {
      throw const FormatException('Expected study word points list');
    }

    final rows = points
        .whereType<Map<String, dynamic>>()
        .map((point) {
          return {
            'user_id': userId,
            'point_date': point['pointDate'] as String? ?? '',
            'entry_id': _intValue(point['entryId']),
            'mode': point['mode'] as String? ?? 'unknown',
            'question_type': _normalizeQuestionType(point['questionType']),
            'attempt_count': _intValue(point['attemptCount']),
            'correct_count': _intValue(point['correctCount']),
            'wrong_count': _intValue(point['wrongCount']),
            'total_response_time_ms': _intValue(point['totalResponseTimeMs']),
            'last_answered_at': point['lastAnsweredAt'] as String? ?? '',
          };
        })
        .where((row) {
          return (row['point_date'] as String).isNotEmpty &&
              (row['entry_id'] as int) > 0 &&
              (row['last_answered_at'] as String).isNotEmpty;
        })
        .toList(growable: false);

    if (rows.isEmpty) return;
    await _authService.client
        .from('study_word_points')
        .upsert(
          rows,
          onConflict: 'user_id,point_date,entry_id,mode,question_type',
        );
  }

  Future<void> _uploadWrongWordEntries({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected wrong_word_entries payload object');
    }
    final entries = payload['entries'];
    if (entries is! List<dynamic>) {
      throw const FormatException('Expected wrong word entries list');
    }

    final rows = entries
        .whereType<Map<String, dynamic>>()
        .map((entry) {
          return {
            'user_id': userId,
            'entry_id': _intValue(entry['entryId']),
            'error_count': _intValue(entry['errorCount']),
            'last_wrong_at': entry['lastWrongAt'] as String? ?? '',
            'priority_score': _numValue(entry['priorityScore']),
            'hint_text': entry['hintText'] as String? ?? '',
            'hint_source': entry['hintSource'] as String? ?? '',
            'hint_updated_at': _nullableNonEmptyString(entry['hintUpdatedAt']),
            'projection_version': _intValue(
              entry['projectionVersion'],
              fallback: 1,
            ),
          };
        })
        .where((row) {
          return (row['entry_id'] as int) > 0 &&
              (row['error_count'] as int) > 0 &&
              (row['last_wrong_at'] as String).isNotEmpty;
        })
        .toList(growable: false);

    if (rows.isEmpty) return;
    await _authService.client
        .from('wrong_word_entries')
        .upsert(rows, onConflict: 'user_id,entry_id');
  }

  Future<void> _uploadAiPassage({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected ai_passages payload object');
    }
    final passage = payload['passage'];
    if (passage is! Map<String, dynamic>) {
      throw const FormatException('Expected AI passage object');
    }
    final localPassageId = passage['passageId'] as String? ?? '';
    if (localPassageId.isEmpty) {
      throw const FormatException('Expected AI passage id');
    }

    await _authService.client.from('ai_passages').upsert({
      'passage_id': _stableUuidForLocalId(localPassageId),
      'user_id': userId,
      'title': passage['title'] as String? ?? '',
      'payload_json': passage,
      'validation_status': passage['validationStatus'] as String? ?? 'pending',
      'generated_at':
          passage['generatedAt'] as String? ?? DateTime.now().toIso8601String(),
    }, onConflict: 'passage_id');
  }

  Future<void> _uploadCrocBtiProfile({
    required String userId,
    required SyncOutboxItem item,
  }) async {
    final payload = jsonDecode(item.payloadJson);
    if (payload is! Map<String, dynamic>) {
      throw const FormatException('Expected croc_bti_profile payload object');
    }
    final profile = payload['profile'];
    if (profile is! Map<String, dynamic>) {
      throw const FormatException('Expected Croc BTI profile object');
    }
    final resultCode = '${profile['resultCode'] ?? ''}'.trim();
    if (resultCode.isEmpty) {
      throw const FormatException('Expected Croc BTI result code');
    }

    await _authService.client.from('croc_bti_profiles').upsert({
      'user_id': userId,
      'result_code': resultCode,
      'title': profile['title'] as String? ?? '',
      'summary': profile['summary'] as String? ?? '',
      'advice': profile['advice'] as String? ?? '',
      'answers_json': _jsonObject(profile['answers']),
      'axis_scores_json': _jsonObject(profile['axisScores']),
      'weights_json': _jsonObject(profile['weights']),
      'plan_input_json': _jsonObject(profile['planInput']),
      'question_type_weights_json': _jsonObject(
        profile['questionTypeWeightsByMode'],
      ),
      'daily_learning_minutes': _intValue(
        profile['dailyLearningMinutes'],
        fallback: 40,
      ).clamp(10, 240),
      'source': profile['source'] as String? ?? 'croc_bti',
      'version': _intValue(profile['version'], fallback: 1),
      'evaluated_at':
          profile['evaluatedAt'] as String? ?? DateTime.now().toIso8601String(),
    }, onConflict: 'user_id');
  }

  Future<void> _recordSyncResult(
    int itemId, {
    required bool succeeded,
    String? failureCode,
    String? failureMessage,
  }) async {
    await _bridge.call(
      'recordSyncResult',
      _codec.encodeRequest({
        'itemId': itemId,
        'succeeded': succeeded,
        'failureCode': ?failureCode,
        'failureMessage': ?failureMessage,
      }),
    );
  }

  Future<void> _recordCloudRestoreAttempt(Map<String, dynamic> payload) async {
    try {
      await _bridge.call(
        'recordCloudRestoreAttempt',
        _codec.encodeRequest(payload),
      );
    } catch (_) {
      // Restore diagnostics must never hide the original restore result.
    }
  }

  int _intValue(Object? value, {int fallback = 0}) {
    if (value is num) return value.toInt();
    return fallback;
  }

  num _numValue(Object? value) {
    if (value is num) return value;
    return 0;
  }

  int? _nullableIntValue(Object? value) {
    if (value is num) return value.toInt();
    return null;
  }

  String? _nullableNonEmptyString(Object? value) {
    final text = value?.toString().trim() ?? '';
    return text.isEmpty ? null : text;
  }

  Map<String, dynamic> _jsonObject(Object? value) {
    if (value is Map<String, dynamic>) return value;
    if (value is Map) return value.cast<String, dynamic>();
    return const <String, dynamic>{};
  }

  String _normalizeQuestionType(Object? value) {
    final raw = value?.toString() ?? '';
    return switch (raw) {
      'enToCnChoice' ||
      'exampleToCnChoice' ||
      'exampleToCnChoiceNoTranslation' ||
      'cnToEnChoice' ||
      'enToCnInput' ||
      'wordSkeletonInput' ||
      'glossToRootInput' ||
      'rootToGlossInput' => raw,
      'spelling' || 'input' => 'enToCnInput',
      _ => 'enToCnChoice',
    };
  }

  List<Map<String, dynamic>> _mapList(Object? value) {
    if (value is List<dynamic>) {
      return value
          .whereType<Map<dynamic, dynamic>>()
          .map(
            (row) => row.map((key, value) => MapEntry(key.toString(), value)),
          )
          .toList(growable: false);
    }
    return const [];
  }

  String _stableUuidForLocalId(String value) {
    const namespace = '6f7fb5309f7349a39b9135d96e3f4e21';
    final source = utf8.encode('$namespace:$value');
    var h1 = 0x811c9dc5;
    var h2 = 0x01000193;
    var h3 = 0x9e3779b9;
    var h4 = 0x85ebca6b;
    for (final byte in source) {
      h1 = ((h1 ^ byte) * 0x01000193) & 0xffffffff;
      h2 = ((h2 + byte) * 0x85ebca6b) & 0xffffffff;
      h3 = ((h3 ^ (byte << 1)) * 0xc2b2ae35) & 0xffffffff;
      h4 = ((h4 + (byte << 2)) * 0x27d4eb2d) & 0xffffffff;
    }
    final hex = [
      h1.toRadixString(16).padLeft(8, '0'),
      h2.toRadixString(16).padLeft(8, '0'),
      h3.toRadixString(16).padLeft(8, '0'),
      h4.toRadixString(16).padLeft(8, '0'),
    ].join();
    return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-'
        '4${hex.substring(13, 16)}-8${hex.substring(17, 20)}-'
        '${hex.substring(20, 32)}';
  }
}

class _WordAdminSnapshotSummary {
  const _WordAdminSnapshotSummary({
    required this.wordbookRows,
    required this.studyPointRows,
    required this.reportSnapshotRows,
    required this.wrongWordRows,
    required this.aiPassageRows,
    required this.crocBtiProfileRows,
  });

  final int wordbookRows;
  final int studyPointRows;
  final int reportSnapshotRows;
  final int wrongWordRows;
  final int aiPassageRows;
  final int crocBtiProfileRows;

  int get totalRows =>
      wordbookRows +
      studyPointRows +
      reportSnapshotRows +
      wrongWordRows +
      aiPassageRows +
      crocBtiProfileRows;
}

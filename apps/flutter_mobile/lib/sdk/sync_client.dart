/// Sync client - read-only SDK for Rust-owned sync queue status.
library;

import '../bridge/bridge.dart';

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

  const SyncStatus({
    required this.syncEnabled,
    required this.transportConfigured,
    required this.accountSyncState,
    required this.pendingCount,
    this.lastSyncSucceededAt,
    this.lastSyncErrorCode,
    this.domainsPending = const [],
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
      );
}

class SyncClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const SyncClient(this._bridge, this._codec);

  Future<SyncStatus> getSyncStatus() async {
    final raw = await _bridge.call('getSyncStatus');
    final json = _codec.decodeResponse(raw);
    return SyncStatus.fromJson(json);
  }
}

/// Local data ownership client.
library;

import '../bridge/bridge.dart';

class LocalDataOwnerResult {
  const LocalDataOwnerResult({
    required this.ownerUserId,
    required this.resetPerformed,
    required this.restoredSnapshot,
    required this.hasLocalLearningData,
    this.previousOwnerUserId,
  });

  final String ownerUserId;
  final bool resetPerformed;
  final bool restoredSnapshot;
  final bool hasLocalLearningData;
  final String? previousOwnerUserId;

  factory LocalDataOwnerResult.fromJson(Map<String, dynamic> json) =>
      LocalDataOwnerResult(
        ownerUserId: json['ownerUserId'] as String? ?? '',
        previousOwnerUserId: json['previousOwnerUserId'] as String?,
        resetPerformed: json['resetPerformed'] as bool? ?? false,
        restoredSnapshot: json['restoredSnapshot'] as bool? ?? false,
        hasLocalLearningData: json['hasLocalLearningData'] as bool? ?? true,
      );
}

class LocalDataOwnerClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const LocalDataOwnerClient(this._bridge, this._codec);

  Future<LocalDataOwnerResult> reconcile(String userId) async {
    final raw = await _bridge.call(
      'reconcileLocalDataOwner',
      _codec.encodeRequest({'userId': userId}),
    );
    return LocalDataOwnerResult.fromJson(_codec.decodeResponse(raw));
  }

  Future<void> preserveGuestLocalData() async {
    await _bridge.call('preserveGuestLocalData');
  }
}

abstract interface class LocalDataOwnerGateway {
  Future<LocalDataOwnerResult> reconcile(String userId);
  Future<void> preserveGuestLocalData();
}

class RustLocalDataOwnerGateway implements LocalDataOwnerGateway {
  const RustLocalDataOwnerGateway(this._client);

  final LocalDataOwnerClient _client;

  @override
  Future<LocalDataOwnerResult> reconcile(String userId) =>
      _client.reconcile(userId);

  @override
  Future<void> preserveGuestLocalData() => _client.preserveGuestLocalData();
}

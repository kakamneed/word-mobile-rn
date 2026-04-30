/// Bootstrap client - typed SDK for the bootstrap API.
library;

import '../bridge/bridge.dart';

/// Bootstrap state returned by the Rust core.
class BootstrapState {
  final bool appReady;
  final bool firstRunRequired;
  final String databaseStatus;
  final String snapshotStatus;
  final String connectivityStatus;
  final String aiConfigStatus;
  final bool settingsEntryAvailable;
  final String? blockingReason;

  const BootstrapState({
    required this.appReady,
    required this.firstRunRequired,
    required this.databaseStatus,
    required this.snapshotStatus,
    required this.connectivityStatus,
    required this.aiConfigStatus,
    required this.settingsEntryAvailable,
    this.blockingReason,
  });

  factory BootstrapState.fromJson(Map<String, dynamic> json) => BootstrapState(
        appReady: json['appReady'] as bool,
        firstRunRequired: json['firstRunRequired'] as bool,
        databaseStatus: json['databaseStatus'] as String,
        snapshotStatus: json['snapshotStatus'] as String,
        connectivityStatus: json['connectivityStatus'] as String,
        aiConfigStatus: json['aiConfigStatus'] as String,
        settingsEntryAvailable: json['settingsEntryAvailable'] as bool,
        blockingReason: json['blockingReason'] as String?,
      );
}

/// Client for the bootstrap API.
class BootstrapClient {
  final RustBridge _bridge;
  final BridgeCodec _codec;

  const BootstrapClient(this._bridge, this._codec);

  /// Initialize the underlying Rust bridge runtime.
  Future<void> initializeBridge() async {
    await _bridge.initialize();
  }

  /// Get the bootstrap state. Must be called after RustBridge.initialize().
  Future<BootstrapState> getBootstrapState() async {
    final raw = await _bridge.call('getBootstrapState');
    final json = _codec.decodeResponse(raw);
    return BootstrapState.fromJson(json);
  }

  /// Mark onboarding as completed.
  Future<void> markOnboardingCompleted() async {
    await _bridge.callVoid('markOnboardingCompleted');
  }
}

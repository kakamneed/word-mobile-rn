/// Application-level state container.
///
/// Holds the current bootstrap state and initializes the Rust runtime.
/// This is the single source of truth for app-level lifecycle state.
library;

import 'package:flutter/foundation.dart';

import '../bridge/bridge_error.dart';
import '../sdk/sdk.dart';
import '../supabase/auth_session_manager.dart';

/// The current phase of the app.
enum AppPhase { uninitialized, initializing, ready, onboarding, error }

/// App-level state that manages Rust bridge initialization and lifecycle.
class AppState extends ChangeNotifier {
  final WordSdk _sdk;
  final AuthSessionManager _authSessionManager;

  AppPhase _phase = AppPhase.uninitialized;
  AuthAccountState _authState = const AuthAccountState.uninitialized();
  BootstrapState? _bootstrapState;
  BridgeError? _lastError;
  bool _isInitializing = false;

  AppState(this._sdk, {AuthSessionManager? authSessionManager})
      : _authSessionManager =
            authSessionManager ??
            AuthSessionManager(
              localDataOwner: RustLocalDataOwnerGateway(_sdk.localDataOwner),
              restoreCloudData: _sdk.sync.restoreCloudDataToLocal,
              backfillLocalLearning: _sdk.sync.backfillLocalLearningToCloud,
            );

  AppPhase get phase => _phase;
  AuthAccountPhase get authPhase => _authState.phase;
  AuthAccountState get authState => _authState;
  BootstrapState? get bootstrapState => _bootstrapState;
  BridgeError? get lastError => _lastError;
  String? get authMessage => _authState.message;
  WordSdk get sdk => _sdk;

  bool get isReady => _phase == AppPhase.ready;
  bool get needsOnboarding => _phase == AppPhase.onboarding;
  bool get isSignedIn => _authState.isSignedIn;

  /// Initialize the Rust runtime and evaluate bootstrap state.
  Future<void> initialize() async {
    if (_isInitializing ||
        _phase == AppPhase.ready ||
        _phase == AppPhase.onboarding) {
      return;
    }

    _isInitializing = true;
    _phase = AppPhase.initializing;
    _lastError = null;
    notifyListeners();

    try {
      await _sdk.bootstrap.initializeBridge();
      final state = await _sdk.bootstrap.getBootstrapState();
      _bootstrapState = state;

      if (!state.appReady && state.firstRunRequired) {
        _phase = AppPhase.onboarding;
      } else if (state.appReady) {
        _phase = AppPhase.ready;
      } else {
        _phase = AppPhase.error;
        _lastError = BridgeError.runtime(
          'BOOTSTRAP_BLOCKED',
          state.blockingReason ?? 'Unknown bootstrap blocker',
        );
      }
    } on BridgeError catch (e) {
      _phase = AppPhase.error;
      _lastError = e;
    } catch (e) {
      _phase = AppPhase.error;
      _lastError = BridgeError.runtime('INIT_FAILED', e.toString());
    }

    if (_phase == AppPhase.ready || _phase == AppPhase.onboarding) {
      await refreshAuthState();
    }

    _isInitializing = false;
    notifyListeners();
  }

  /// Retry startup initialization after a recoverable failure.
  Future<void> retryInitialize() async {
    _phase = AppPhase.uninitialized;
    _lastError = null;
    _authState = const AuthAccountState.uninitialized();
    notifyListeners();
    await initialize();
  }

  /// Complete onboarding and transition to ready state.
  Future<void> completeOnboarding() async {
    if (_phase != AppPhase.onboarding) return;
    try {
      await _sdk.bootstrap.markOnboardingCompleted();
      final state = await _sdk.bootstrap.getBootstrapState();
      _bootstrapState = state;
      if (state.appReady) {
        _phase = AppPhase.ready;
        _lastError = null;
        await refreshAuthState();
      }
    } on BridgeError catch (e) {
      _lastError = e;
    }
    notifyListeners();
  }

  Future<void> refreshAuthState() async {
    _authState = const AuthAccountState.checking();
    notifyListeners();
    _authState = await _authSessionManager.resolveStartupState();
    notifyListeners();
  }

  void applyAuthState(AuthAccountState authState) {
    _authState = authState;
    notifyListeners();
  }
}

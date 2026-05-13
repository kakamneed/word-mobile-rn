library;

import 'package:supabase_flutter/supabase_flutter.dart';

import '../sdk/local_data_owner_client.dart';
import '../cloud/cloud_backend_config.dart';
import 'supabase_auth_service.dart';
import 'supabase_config.dart';
import 'word_admin_auth_service.dart';

enum AuthAccountPhase {
  uninitialized,
  checking,
  notConfigured,
  guestLocalOnly,
  signedInNeedsBind,
  signedInActive,
  signedInExpired,
  signedOutRetainedLocal,
  accountDeletedOrRevoked,
  error,
}

class AuthAccountState {
  final AuthAccountPhase phase;
  final Session? _session;
  final String? message;

  const AuthAccountState._({
    required this.phase,
    Session? session,
    this.message,
  }) : _session = session;

  const AuthAccountState.uninitialized()
    : this._(phase: AuthAccountPhase.uninitialized);

  const AuthAccountState.checking() : this._(phase: AuthAccountPhase.checking);

  const AuthAccountState.notConfigured()
    : this._(
        phase: AuthAccountPhase.notConfigured,
        message: 'Cloud backend not configured',
      );

  const AuthAccountState.guestLocalOnly([
    String message = 'Guest local-only mode',
  ]) : this._(phase: AuthAccountPhase.guestLocalOnly, message: message);

  const AuthAccountState.signedOutRetainedLocal()
    : this._(
        phase: AuthAccountPhase.signedOutRetainedLocal,
        message: 'Signed out; local learning data is retained',
      );

  const AuthAccountState.accountDeletedOrRevoked(String message)
    : this._(phase: AuthAccountPhase.accountDeletedOrRevoked, message: message);

  const AuthAccountState.error(String message)
    : this._(phase: AuthAccountPhase.error, message: message);

  AuthAccountState.signedInActive(Session session)
    : this._(phase: AuthAccountPhase.signedInActive, session: session);

  AuthAccountState.signedInNeedsBind(Session session, [String? message])
    : this._(
        phase: AuthAccountPhase.signedInNeedsBind,
        session: session,
        message:
            message ??
            'Signed in. Cloud sync is paused until local and cloud data are checked.',
      );

  AuthAccountState.signedInExpired(String message, {Session? session})
    : this._(
        phase: AuthAccountPhase.signedInExpired,
        session: session,
        message: message,
      );

  bool get isSignedIn =>
      (phase == AuthAccountPhase.signedInActive ||
          phase == AuthAccountPhase.signedInNeedsBind) &&
      _session != null;

  bool get allowsLocalStudy =>
      phase != AuthAccountPhase.uninitialized &&
      phase != AuthAccountPhase.checking;

  bool get allowsCloudWork => phase == AuthAccountPhase.signedInActive;

  String? get userEmail => _session?.user.email;

  String? get userId => _session?.user.id;

  int? get expiresAt => _session?.expiresAt;
}

class AuthSessionManager {
  final SupabaseAuthGateway _auth;
  final bool Function() _isConfigured;
  final LocalDataOwnerGateway? _localDataOwner;
  final Future<void> Function(String userId)? _restoreCloudData;
  final Future<void> Function()? _backfillLocalLearning;
  final Future<bool> Function(String userId, LocalDataOwnerResult ownerResult)?
  _shouldRestoreCloudData;

  AuthSessionManager({
    SupabaseAuthGateway? auth,
    bool Function()? isConfigured,
    LocalDataOwnerGateway? localDataOwner,
    Future<void> Function(String userId)? restoreCloudData,
    Future<void> Function()? backfillLocalLearning,
    Future<bool> Function(String userId, LocalDataOwnerResult ownerResult)?
    shouldRestoreCloudData,
  }) : _auth = auth ?? _defaultAuthGateway(),
       _isConfigured = isConfigured ?? _defaultIsConfigured,
       _localDataOwner = localDataOwner,
       _restoreCloudData = restoreCloudData,
       _backfillLocalLearning = backfillLocalLearning,
       _shouldRestoreCloudData = shouldRestoreCloudData;

  static SupabaseAuthGateway _defaultAuthGateway() {
    return CloudBackendConfig.usesWordAdmin
        ? WordAdminAuthService()
        : SupabaseAuthService();
  }

  static bool _defaultIsConfigured() {
    if (CloudBackendConfig.usesWordAdmin) {
      return CloudBackendConfig.fromEnvironment().isConfigured;
    }
    return SupabaseConfig.isConfigured;
  }

  Future<AuthAccountState> resolveStartupState() async {
    if (!_isConfigured()) {
      return const AuthAccountState.notConfigured();
    }

    try {
      final session = await _auth.restoreSession();
      return _stateFromSession(session);
    } catch (error) {
      return _stateFromAuthFailure(error);
    }
  }

  Future<AuthAccountState> signUp({
    required String email,
    required String password,
  }) async {
    try {
      final response = await _auth.signUp(email: email, password: password);
      final session = response.session ?? await _auth.restoreSession();
      return _stateFromSession(
        session,
        emptyMessage:
            'Signup request completed. Check email confirmation before cloud sync is enabled.',
      );
    } catch (error) {
      return _stateFromAuthFailure(error);
    }
  }

  Future<AuthAccountState> signIn({
    required String email,
    required String password,
  }) async {
    try {
      final response = await _auth.signIn(email: email, password: password);
      final session = response.session ?? await _auth.restoreSession();
      return _stateFromSession(session);
    } catch (error) {
      return _stateFromAuthFailure(error);
    }
  }

  Future<AuthAccountState> signOutRetainingLocalData() async {
    try {
      await _localDataOwner?.preserveGuestLocalData();
      await _auth.signOut();
      return const AuthAccountState.signedOutRetainedLocal();
    } catch (error) {
      return AuthAccountState.error(error.toString());
    }
  }

  Future<AuthAccountState> _stateFromSession(
    Session? session, {
    String? emptyMessage,
  }) async {
    if (session == null) {
      await _localDataOwner?.preserveGuestLocalData();
      return emptyMessage == null
          ? const AuthAccountState.guestLocalOnly()
          : AuthAccountState.guestLocalOnly(emptyMessage);
    }

    if (!session.isExpired) {
      return _stateFromVerifiedSession(session);
    }

    try {
      final refreshed = await _auth.refreshSession();
      final refreshedSession =
          refreshed.session ?? await _auth.restoreSession();
      if (refreshedSession != null && !refreshedSession.isExpired) {
        return _stateFromVerifiedSession(refreshedSession);
      }
    } catch (error) {
      return _stateFromAuthFailure(error, expiredSession: session);
    }

    return AuthAccountState.signedInExpired(
      'Supabase session is expired; local study remains available.',
      session: session,
    );
  }

  Future<AuthAccountState> _stateFromVerifiedSession(Session session) async {
    try {
      await _auth.verifyCloudDataAccess(session.user.id);
      final ownerResult = await _localDataOwner?.reconcile(session.user.id);
      if (ownerResult != null) {
        final shouldRestore = _shouldRestoreCloudData == null
            ? !ownerResult.restoredSnapshot &&
                  (ownerResult.resetPerformed ||
                      !ownerResult.hasLocalLearningData)
            : await _shouldRestoreCloudData.call(session.user.id, ownerResult);
        if (shouldRestore) {
          await _restoreCloudData?.call(session.user.id);
        }
        if (ownerResult.hasLocalLearningData) {
          await _backfillLocalLearning?.call();
        }
      } else {
        await _backfillLocalLearning?.call();
      }
      return AuthAccountState.signedInActive(session);
    } catch (error) {
      final message = error.toString();
      if (_isNetworkFailure(message.toLowerCase())) {
        await _switchToGuestLocalData();
        return AuthAccountState.guestLocalOnly(
          'Network connection failed; using local guest data.',
        );
      }
      return AuthAccountState.signedInNeedsBind(
        session,
        'Cloud data check failed: $error',
      );
    }
  }

  Future<AuthAccountState> _stateFromAuthFailure(
    Object error, {
    Session? expiredSession,
  }) async {
    final message = error.toString();
    final lower = message.toLowerCase();

    if (_isNetworkFailure(lower)) {
      await _switchToGuestLocalData();
      return AuthAccountState.guestLocalOnly(
        'Network connection failed; using local guest data.',
      );
    }

    if (lower.contains('revoked') ||
        lower.contains('deleted') ||
        lower.contains('user not found')) {
      await _switchToGuestLocalData();
      return AuthAccountState.accountDeletedOrRevoked(message);
    }

    if (expiredSession != null ||
        lower.contains('expired') ||
        lower.contains('invalid refresh') ||
        lower.contains('invalid_grant')) {
      await _switchToGuestLocalData();
      return AuthAccountState.signedInExpired(
        '$message; local study remains available.',
      );
    }

    return AuthAccountState.error(message);
  }

  Future<void> _switchToGuestLocalData() async {
    await _localDataOwner?.preserveGuestLocalData();
  }

  bool _isNetworkFailure(String lowerMessage) {
    return lowerMessage.contains('socketexception') ||
        lowerMessage.contains('connection refused') ||
        lowerMessage.contains('connection failed') ||
        lowerMessage.contains('connection reset') ||
        lowerMessage.contains('failed host lookup') ||
        lowerMessage.contains('network is unreachable') ||
        lowerMessage.contains('network connection failed') ||
        lowerMessage.contains('network error') ||
        lowerMessage.contains('timed out') ||
        lowerMessage.contains('timeout') ||
        lowerMessage.contains('httpclientexception') ||
        lowerMessage.contains('clientexception');
  }
}

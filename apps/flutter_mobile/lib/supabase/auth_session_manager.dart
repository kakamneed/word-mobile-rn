library;

import 'package:supabase_flutter/supabase_flutter.dart';

import '../sdk/local_data_owner_client.dart';
import 'supabase_auth_service.dart';
import 'supabase_config.dart';

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
        message: 'Supabase not configured',
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

  AuthAccountState.signedInNeedsBind(Session session)
    : this._(
        phase: AuthAccountPhase.signedInNeedsBind,
        session: session,
        message:
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

  AuthSessionManager({
    SupabaseAuthGateway? auth,
    bool Function()? isConfigured,
    LocalDataOwnerGateway? localDataOwner,
    Future<void> Function(String userId)? restoreCloudData,
    Future<void> Function()? backfillLocalLearning,
  })
    : _auth = auth ?? SupabaseAuthService(),
      _isConfigured = isConfigured ?? (() => SupabaseConfig.isConfigured),
      _localDataOwner = localDataOwner,
      _restoreCloudData = restoreCloudData,
      _backfillLocalLearning = backfillLocalLearning;

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
      if (ownerResult != null &&
          !ownerResult.restoredSnapshot &&
          (ownerResult.resetPerformed || !ownerResult.hasLocalLearningData)) {
        await _restoreCloudData?.call(session.user.id);
      } else if (ownerResult != null && ownerResult.hasLocalLearningData) {
        await _backfillLocalLearning?.call();
      }
      return AuthAccountState.signedInActive(session);
    } catch (_) {
      return AuthAccountState.signedInNeedsBind(session);
    }
  }

  AuthAccountState _stateFromAuthFailure(
    Object error, {
    Session? expiredSession,
  }) {
    final message = error.toString();
    final lower = message.toLowerCase();

    if (lower.contains('revoked') ||
        lower.contains('deleted') ||
        lower.contains('user not found')) {
      return AuthAccountState.accountDeletedOrRevoked(message);
    }

    if (expiredSession != null ||
        lower.contains('expired') ||
        lower.contains('invalid refresh') ||
        lower.contains('invalid_grant')) {
      return AuthAccountState.signedInExpired(
        '$message; local study remains available.',
        session: expiredSession,
      );
    }

    return AuthAccountState.error(message);
  }
}

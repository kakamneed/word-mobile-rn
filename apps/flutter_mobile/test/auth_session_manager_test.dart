import 'package:flutter_mobile/supabase/auth_session_manager.dart';
import 'package:flutter_mobile/sdk/local_data_owner_client.dart';
import 'package:flutter_mobile/supabase/supabase_auth_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

void main() {
  test(
    'reports notConfigured before touching Supabase when env is absent',
    () async {
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(),
        isConfigured: () => false,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.notConfigured);
      expect(state.allowsCloudWork, isFalse);
      expect(state.allowsLocalStudy, isTrue);
    },
  );

  test('resolves empty secure session as guest local-only', () async {
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(),
      isConfigured: () => true,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.guestLocalOnly);
    expect(state.allowsCloudWork, isFalse);
    expect(state.allowsLocalStudy, isTrue);
  });

  test(
    'logout publishes retained-local instead of generic signed-out',
    () async {
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(),
        isConfigured: () => true,
      );

      final state = await manager.signOutRetainingLocalData();

      expect(state.phase, AuthAccountPhase.signedOutRetainedLocal);
      expect(state.message, contains('local learning data is retained'));
      expect(state.allowsCloudWork, isFalse);
    },
  );

  test(
    'signup without immediate session keeps user in local-only mode',
    () async {
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(signUpResponse: AuthResponse()),
        isConfigured: () => true,
      );

      final state = await manager.signUp(
        email: 'new@example.com',
        password: 'secret',
      );

      expect(state.phase, AuthAccountPhase.guestLocalOnly);
      expect(state.message, contains('email confirmation'));
    },
  );

  test('valid restored session waits for bind before cloud work', () async {
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(restoredSession: _validSession()),
      isConfigured: () => true,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.signedInNeedsBind);
    expect(state.isSignedIn, isTrue);
    expect(state.allowsLocalStudy, isTrue);
    expect(state.allowsCloudWork, isFalse);
  });

  test('startup network failure switches to guest local data', () async {
    final localDataOwner = _FakeLocalDataOwnerGateway();
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(
        restoreError: Exception('SocketException: Connection refused'),
      ),
      localDataOwner: localDataOwner,
      isConfigured: () => true,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.guestLocalOnly);
    expect(state.isSignedIn, isFalse);
    expect(state.allowsCloudWork, isFalse);
    expect(localDataOwner.preserveGuestLocalDataCount, 1);
  });

  test(
    'cloud access network failure does not keep prior account data',
    () async {
      final localDataOwner = _FakeLocalDataOwnerGateway();
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(
          restoredSession: _validSession(),
          verifyError: Exception('SocketException: Connection refused'),
        ),
        localDataOwner: localDataOwner,
        isConfigured: () => true,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.guestLocalOnly);
      expect(state.isSignedIn, isFalse);
      expect(state.allowsCloudWork, isFalse);
      expect(localDataOwner.reconciledUserIds, isEmpty);
      expect(localDataOwner.preserveGuestLocalDataCount, 1);
    },
  );

  test(
    'valid restored session becomes active after cloud access check',
    () async {
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(
          restoredSession: _validSession(),
          cloudDataAccessAvailable: true,
        ),
        localDataOwner: _FakeLocalDataOwnerGateway(),
        isConfigured: () => true,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.signedInActive);
      expect(state.isSignedIn, isTrue);
      expect(state.allowsCloudWork, isTrue);
    },
  );

  test('active session reconciles local data owner', () async {
    final localDataOwner = _FakeLocalDataOwnerGateway();
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(
        restoredSession: _validSession(),
        cloudDataAccessAvailable: true,
      ),
      localDataOwner: localDataOwner,
      isConfigured: () => true,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.signedInActive);
    expect(localDataOwner.reconciledUserIds, ['user-1']);
  });

  test(
    'active session restores cloud data when no local snapshot exists',
    () async {
      final restoredUserIds = <String>[];
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(
          restoredSession: _validSession(),
          cloudDataAccessAvailable: true,
        ),
        localDataOwner: _FakeLocalDataOwnerGateway(
          result: const LocalDataOwnerResult(
            ownerUserId: 'user-1',
            resetPerformed: true,
            restoredSnapshot: false,
            hasLocalLearningData: false,
          ),
        ),
        restoreCloudData: (userId) async {
          restoredUserIds.add(userId);
        },
        isConfigured: () => true,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.signedInActive);
      expect(restoredUserIds, ['user-1']);
    },
  );

  test(
    'active session keeps restored local snapshot without cloud pull',
    () async {
      final restoredUserIds = <String>[];
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(
          restoredSession: _validSession(),
          cloudDataAccessAvailable: true,
        ),
        localDataOwner: _FakeLocalDataOwnerGateway(
          result: const LocalDataOwnerResult(
            ownerUserId: 'user-1',
            resetPerformed: true,
            restoredSnapshot: true,
            hasLocalLearningData: true,
          ),
        ),
        restoreCloudData: (userId) async {
          restoredUserIds.add(userId);
        },
        isConfigured: () => true,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.signedInActive);
      expect(restoredUserIds, isEmpty);
    },
  );

  test(
    'active session restores cloud data when local learning data is empty',
    () async {
      final restoredUserIds = <String>[];
      final manager = AuthSessionManager(
        auth: _FakeAuthGateway(
          restoredSession: _validSession(),
          cloudDataAccessAvailable: true,
        ),
        localDataOwner: _FakeLocalDataOwnerGateway(
          result: const LocalDataOwnerResult(
            ownerUserId: 'user-1',
            resetPerformed: false,
            restoredSnapshot: false,
            hasLocalLearningData: false,
          ),
        ),
        restoreCloudData: (userId) async {
          restoredUserIds.add(userId);
        },
        isConfigured: () => true,
      );

      final state = await manager.resolveStartupState();

      expect(state.phase, AuthAccountPhase.signedInActive);
      expect(restoredUserIds, ['user-1']);
    },
  );

  test('active session backfills local learning data when present', () async {
    var backfillCount = 0;
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(
        restoredSession: _validSession(),
        cloudDataAccessAvailable: true,
      ),
      localDataOwner: _FakeLocalDataOwnerGateway(
        result: const LocalDataOwnerResult(
          ownerUserId: 'user-1',
          resetPerformed: false,
          restoredSnapshot: false,
          hasLocalLearningData: true,
        ),
      ),
      backfillLocalLearning: () async {
        backfillCount += 1;
      },
      isConfigured: () => true,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.signedInActive);
    expect(backfillCount, 1);
  });
}

class _FakeAuthGateway implements SupabaseAuthGateway {
  _FakeAuthGateway({
    this.signUpResponse,
    this.restoredSession,
    this.restoreError,
    this.verifyError,
    this.cloudDataAccessAvailable = false,
  });

  final AuthResponse? signUpResponse;
  final Session? restoredSession;
  final Object? restoreError;
  final Object? verifyError;
  final bool cloudDataAccessAvailable;

  @override
  Future<AuthResponse> refreshSession() async => AuthResponse();

  @override
  Future<Session?> restoreSession() async {
    final error = restoreError;
    if (error != null) {
      throw error;
    }
    return restoredSession;
  }

  @override
  Future<AuthResponse> signIn({
    required String email,
    required String password,
  }) async => AuthResponse();

  @override
  Future<void> signOut() async {}

  @override
  Future<AuthResponse> signUp({
    required String email,
    required String password,
  }) async => signUpResponse ?? AuthResponse();

  @override
  Future<void> verifyCloudDataAccess(String userId) async {
    final error = verifyError;
    if (error != null) {
      throw error;
    }
    if (!cloudDataAccessAvailable) {
      throw StateError('cloud data not bound yet');
    }
  }
}

class _FakeLocalDataOwnerGateway implements LocalDataOwnerGateway {
  _FakeLocalDataOwnerGateway({this.result});

  final LocalDataOwnerResult? result;
  final reconciledUserIds = <String>[];
  var preserveGuestLocalDataCount = 0;

  @override
  Future<LocalDataOwnerResult> reconcile(String userId) async {
    reconciledUserIds.add(userId);
    return result ??
        LocalDataOwnerResult(
          ownerUserId: userId,
          resetPerformed: false,
          restoredSnapshot: false,
          hasLocalLearningData: true,
        );
  }

  @override
  Future<void> preserveGuestLocalData() async {
    preserveGuestLocalDataCount += 1;
  }
}

Session _validSession() {
  return Session(
    accessToken: 'not-a-real-jwt',
    tokenType: 'bearer',
    user: const User(
      id: 'user-1',
      appMetadata: {},
      userMetadata: {},
      aud: 'authenticated',
      email: 'user@example.com',
      createdAt: '2026-04-30T00:00:00Z',
    ),
  );
}

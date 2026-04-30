import 'package:flutter_mobile/supabase/auth_session_manager.dart';
import 'package:flutter_mobile/supabase/supabase_auth_service.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

void main() {
  test('reports notConfigured before touching Supabase when env is absent', () async {
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(),
      isConfigured: () => false,
    );

    final state = await manager.resolveStartupState();

    expect(state.phase, AuthAccountPhase.notConfigured);
    expect(state.allowsCloudWork, isFalse);
    expect(state.allowsLocalStudy, isTrue);
  });

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

  test('logout publishes retained-local instead of generic signed-out', () async {
    final manager = AuthSessionManager(
      auth: _FakeAuthGateway(),
      isConfigured: () => true,
    );

    final state = await manager.signOutRetainingLocalData();

    expect(state.phase, AuthAccountPhase.signedOutRetainedLocal);
    expect(state.message, contains('local learning data is retained'));
    expect(state.allowsCloudWork, isFalse);
  });

  test('signup without immediate session keeps user in local-only mode', () async {
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
  });
}

class _FakeAuthGateway implements SupabaseAuthGateway {
  _FakeAuthGateway({this.signUpResponse});

  final AuthResponse? signUpResponse;

  @override
  Future<AuthResponse> refreshSession() async => AuthResponse();

  @override
  Future<Session?> restoreSession() async => null;

  @override
  Future<AuthResponse> signIn({
    required String email,
    required String password,
  }) async =>
      AuthResponse();

  @override
  Future<void> signOut() async {}

  @override
  Future<AuthResponse> signUp({
    required String email,
    required String password,
  }) async =>
      signUpResponse ?? AuthResponse();
}

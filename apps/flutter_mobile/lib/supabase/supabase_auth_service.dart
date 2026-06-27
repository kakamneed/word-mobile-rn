library;

import 'package:supabase_flutter/supabase_flutter.dart';

import 'supabase_config.dart';

abstract class SupabaseAuthGateway {
  Future<Session?> restoreSession();

  Future<AuthResponse> signUp({
    required String email,
    required String password,
  });

  Future<AuthResponse> signIn({
    required String email,
    required String password,
  });

  Future<void> resendSignupConfirmation({required String email});

  Future<void> requestPasswordReset({
    required String email,
    String? redirectTo,
  });

  Future<AuthResponse> verifySignupOtp({
    required String email,
    required String token,
  });

  Future<AuthResponse> verifyPasswordRecoveryOtp({
    required String email,
    required String token,
  });

  Future<void> updatePassword({required String password});

  Future<AuthResponse> refreshSession();

  Future<void> signOut();

  Future<void> verifyCloudDataAccess(String userId);
}

class SupabaseAuthService implements SupabaseAuthGateway {
  static bool _initialized = false;

  Future<void> ensureInitialized() async {
    if (_initialized) return;
    final config = SupabaseConfig.fromEnvironment();
    if (config == null) {
      throw StateError('Supabase environment is not configured.');
    }

    await Supabase.initialize(url: config.url, anonKey: config.anonKey);
    _initialized = true;
  }

  SupabaseClient get client => Supabase.instance.client;

  @override
  Future<Session?> restoreSession() async {
    await ensureInitialized();
    return client.auth.currentSession;
  }

  @override
  Future<AuthResponse> signUp({
    required String email,
    required String password,
  }) async {
    await ensureInitialized();
    return client.auth.signUp(email: email, password: password);
  }

  @override
  Future<AuthResponse> signIn({
    required String email,
    required String password,
  }) async {
    await ensureInitialized();
    return client.auth.signInWithPassword(email: email, password: password);
  }

  @override
  Future<void> resendSignupConfirmation({required String email}) async {
    await ensureInitialized();
    await client.auth.resend(type: OtpType.signup, email: email);
  }

  @override
  Future<void> requestPasswordReset({
    required String email,
    String? redirectTo,
  }) async {
    await ensureInitialized();
    await client.auth.resetPasswordForEmail(email, redirectTo: redirectTo);
  }

  @override
  Future<AuthResponse> verifySignupOtp({
    required String email,
    required String token,
  }) async {
    await ensureInitialized();
    return client.auth.verifyOTP(
      email: email,
      token: token,
      type: OtpType.signup,
    );
  }

  @override
  Future<AuthResponse> verifyPasswordRecoveryOtp({
    required String email,
    required String token,
  }) async {
    await ensureInitialized();
    return client.auth.verifyOTP(
      email: email,
      token: token,
      type: OtpType.recovery,
    );
  }

  @override
  Future<void> updatePassword({required String password}) async {
    await ensureInitialized();
    await client.auth.updateUser(UserAttributes(password: password));
  }

  @override
  Future<AuthResponse> refreshSession() async {
    await ensureInitialized();
    return client.auth.refreshSession();
  }

  @override
  Future<void> signOut() async {
    await ensureInitialized();
    await client.auth.signOut();
  }

  @override
  Future<void> verifyCloudDataAccess(String userId) async {
    await ensureInitialized();
    await client
        .from('profiles')
        .select('user_id')
        .eq('user_id', userId)
        .limit(1);
    await client
        .from('plan_configs')
        .select('plan_id')
        .eq('user_id', userId)
        .limit(1);
  }
}

library;

import 'dart:convert';
import 'dart:io';

import 'package:shared_preferences/shared_preferences.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

import '../cloud/cloud_backend_config.dart';
import 'supabase_auth_service.dart';

class WordAdminAuthService implements SupabaseAuthGateway {
  WordAdminAuthService({CloudBackendConfig? config})
      : _config = config ?? CloudBackendConfig.fromEnvironment();

  static const _accessTokenKey = 'word_admin.auth.access_token';
  static const _refreshTokenKey = 'word_admin.auth.refresh_token';
  static const _expiresAtKey = 'word_admin.auth.expires_at';
  static const _userIdKey = 'word_admin.auth.user_id';
  static const _userEmailKey = 'word_admin.auth.user_email';

  final CloudBackendConfig _config;

  String get _baseUrl {
    final value = _config.wordAdminApiUrl.trim();
    if (value.isEmpty) {
      throw StateError('Word Admin API URL is not configured.');
    }
    return value.endsWith('/') ? value.substring(0, value.length - 1) : value;
  }

  @override
  Future<Session?> restoreSession() async {
    final prefs = await SharedPreferences.getInstance();
    final accessToken = prefs.getString(_accessTokenKey);
    final refreshToken = prefs.getString(_refreshTokenKey);
    final userId = prefs.getString(_userIdKey);
    final userEmail = prefs.getString(_userEmailKey);
    if (refreshToken != null && refreshToken.isNotEmpty) {
      try {
        final refreshed = await refreshSession();
        if (refreshed.session != null) return refreshed.session;
      } catch (_) {
        // Fall back to probing the stored access token below.
      }
    }
    if (accessToken == null || accessToken.isEmpty || userId == null || userId.isEmpty) {
      return null;
    }

    final me = await _request(
      'GET',
      '/v1/me',
      accessToken: accessToken,
    );
    final user = _userFromJson(me['user'] as Map<String, dynamic>);
    await _storeUser(user);
    return _sessionFromStoredValues(
      accessToken: accessToken,
      refreshToken: refreshToken,
      userId: user.id,
      userEmail: user.email ?? userEmail ?? '',
      expiresAtIso: prefs.getString(_expiresAtKey),
    );
  }

  @override
  Future<AuthResponse> signUp({
    required String email,
    required String password,
  }) async {
    final response = await _request(
      'POST',
      '/v1/auth/signup',
      body: {
        'email': email,
        'password': password,
      },
    );
    return _authResponseFromJson(response);
  }

  @override
  Future<AuthResponse> signIn({
    required String email,
    required String password,
  }) async {
    final response = await _request(
      'POST',
      '/v1/auth/login',
      body: {
        'email': email,
        'password': password,
      },
    );
    return _authResponseFromJson(response);
  }

  @override
  Future<AuthResponse> refreshSession() async {
    final prefs = await SharedPreferences.getInstance();
    final refreshToken = prefs.getString(_refreshTokenKey);
    if (refreshToken == null || refreshToken.isEmpty) {
      throw StateError('Word Admin refresh token is missing.');
    }
    final response = await _request(
      'POST',
      '/v1/auth/refresh',
      body: {
        'refreshToken': refreshToken,
      },
    );
    final storedUser = await _storedUser();
    return _authResponseFromJson(response, fallbackUser: storedUser);
  }

  @override
  Future<void> signOut() async {
    final prefs = await SharedPreferences.getInstance();
    final accessToken = prefs.getString(_accessTokenKey);
    if (accessToken != null && accessToken.isNotEmpty) {
      try {
        await _request('POST', '/v1/auth/logout', accessToken: accessToken);
      } catch (_) {
        // Local sign-out must still clear tokens when the dev backend is gone.
      }
    }
    await prefs.remove(_accessTokenKey);
    await prefs.remove(_refreshTokenKey);
    await prefs.remove(_expiresAtKey);
    await prefs.remove(_userIdKey);
    await prefs.remove(_userEmailKey);
  }

  @override
  Future<void> verifyCloudDataAccess(String userId) async {
    final prefs = await SharedPreferences.getInstance();
    final accessToken = prefs.getString(_accessTokenKey);
    if (accessToken == null || accessToken.isEmpty) {
      throw StateError('Word Admin access token is missing.');
    }
    await _request('GET', '/v1/me', accessToken: accessToken);
  }

  Future<Map<String, dynamic>> getSyncSnapshot() async {
    final prefs = await SharedPreferences.getInstance();
    final accessToken = prefs.getString(_accessTokenKey);
    if (accessToken == null || accessToken.isEmpty) {
      throw StateError('Word Admin access token is missing.');
    }
    final response = await _request(
      'GET',
      '/v1/sync/snapshot',
      accessToken: accessToken,
    );
    final snapshot = response['snapshot'];
    return snapshot is Map<String, dynamic> ? snapshot : <String, dynamic>{};
  }

  Future<Map<String, dynamic>> flushSyncItem({
    required String domain,
    required Map<String, dynamic> payload,
  }) async {
    final prefs = await SharedPreferences.getInstance();
    final accessToken = prefs.getString(_accessTokenKey);
    if (accessToken == null || accessToken.isEmpty) {
      throw StateError('Word Admin access token is missing.');
    }
    return _request(
      'POST',
      '/v1/sync/flush',
      accessToken: accessToken,
      body: {
        'domain': domain,
        'payload': payload,
      },
    );
  }

  Future<Map<String, dynamic>> _request(
    String method,
    String path, {
    Map<String, dynamic>? body,
    String? accessToken,
  }) async {
    final uri = Uri.parse('$_baseUrl$path');
    final client = HttpClient();
    try {
      final request = await client.openUrl(method, uri);
      request.headers.contentType = ContentType.json;
      if (accessToken != null && accessToken.isNotEmpty) {
        request.headers.set(HttpHeaders.authorizationHeader, 'Bearer $accessToken');
      }
      if (body != null) {
        request.write(jsonEncode(body));
      }
      final response = await request.close();
      final text = await utf8.decodeStream(response);
      final decoded = text.isEmpty ? <String, dynamic>{} : jsonDecode(text);
      final json = decoded is Map<String, dynamic>
          ? decoded
          : <String, dynamic>{'value': decoded};
      if (response.statusCode < 200 || response.statusCode >= 300) {
        final error = json['error'];
        if (error is Map) {
          throw StateError('${error['code']}: ${error['message']}');
        }
        throw StateError('Word Admin API request failed: ${response.statusCode}');
      }
      return json;
    } finally {
      client.close(force: true);
    }
  }

  Future<AuthResponse> _authResponseFromJson(
    Map<String, dynamic> json, {
    User? fallbackUser,
  }) async {
    final sessionJson = json['session'];
    final userJson = json['user'];
    final user = userJson is Map<String, dynamic>
        ? _userFromJson(userJson)
        : fallbackUser;
    if (sessionJson is! Map<String, dynamic> || user == null) {
      return AuthResponse(user: user);
    }

    final accessToken = '${sessionJson['accessToken'] ?? ''}';
    final refreshToken = '${sessionJson['refreshToken'] ?? ''}';
    final expiresAtIso = '${sessionJson['expiresAt'] ?? ''}';
    final session = _sessionFromStoredValues(
      accessToken: accessToken,
      refreshToken: refreshToken,
      userId: user.id,
      userEmail: user.email ?? '',
      expiresAtIso: expiresAtIso,
    );
    await _storeSession(session);
    return AuthResponse(session: session, user: user);
  }

  Session _sessionFromStoredValues({
    required String accessToken,
    required String? refreshToken,
    required String userId,
    required String userEmail,
    String? expiresAtIso,
  }) {
    return Session(
      accessToken: accessToken,
      refreshToken: refreshToken,
      tokenType: 'bearer',
      user: User(
        id: userId,
        appMetadata: const {},
        userMetadata: const {},
        aud: 'authenticated',
        email: userEmail,
        createdAt: DateTime.now().toUtc().toIso8601String(),
      ),
    );
  }

  User _userFromJson(Map<String, dynamic> json) {
    return User(
      id: '${json['id'] ?? ''}',
      appMetadata: const {},
      userMetadata: {
        'display_name': json['displayName'],
        'locale': json['locale'],
      },
      aud: 'authenticated',
      email: '${json['email'] ?? ''}',
      createdAt:
          '${json['createdAt'] ?? DateTime.now().toUtc().toIso8601String()}',
    );
  }

  Future<User?> _storedUser() async {
    final prefs = await SharedPreferences.getInstance();
    final userId = prefs.getString(_userIdKey);
    if (userId == null || userId.isEmpty) return null;
    return User(
      id: userId,
      appMetadata: const {},
      userMetadata: const {},
      aud: 'authenticated',
      email: prefs.getString(_userEmailKey),
      createdAt: DateTime.now().toUtc().toIso8601String(),
    );
  }

  Future<void> _storeUser(User user) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_userIdKey, user.id);
    final email = user.email;
    if (email != null) {
      await prefs.setString(_userEmailKey, email);
    }
  }

  Future<void> _storeSession(Session session) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setString(_accessTokenKey, session.accessToken);
    final refreshToken = session.refreshToken;
    if (refreshToken != null) {
      await prefs.setString(_refreshTokenKey, refreshToken);
    }
    await prefs.setString(_userIdKey, session.user.id);
    final email = session.user.email;
    if (email != null) {
      await prefs.setString(_userEmailKey, email);
    }
  }
}

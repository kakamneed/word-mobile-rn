import 'dart:async';

import 'package:app_links/app_links.dart';
import 'package:flutter/foundation.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

class AuthDeepLinkHandler {
  AuthDeepLinkHandler({
    AppLinks? appLinks,
    SupabaseClient? client,
    this.onPasswordRecovery,
  }) : _appLinks = appLinks ?? AppLinks(),
       _client = client ?? Supabase.instance.client;

  final AppLinks _appLinks;
  final SupabaseClient _client;
  final VoidCallback? onPasswordRecovery;
  StreamSubscription<Uri>? _subscription;
  bool _handledInitialLink = false;

  Future<void> start() async {
    if (_subscription != null) return;
    _subscription = _appLinks.uriLinkStream.listen(_handleUri, onError: (_) {});
    await handleInitialLink();
  }

  Future<void> handleInitialLink() async {
    if (_handledInitialLink) return;
    _handledInitialLink = true;
    final uri = await _appLinks.getInitialLink();
    if (uri != null) {
      await _handleUri(uri);
    }
  }

  Future<void> _handleUri(Uri uri) async {
    if (!_looksLikeSupabaseAuthCallback(uri)) return;
    try {
      final response = await _client.auth.getSessionFromUrl(uri);
      if (_isPasswordRecovery(response.redirectType)) {
        onPasswordRecovery?.call();
      }
    } catch (_) {
      // Supabase's built-in deep link listener may already have consumed the
      // one-time recovery code. In that case the auth-state listener opens UI.
    }
  }

  bool _looksLikeSupabaseAuthCallback(Uri uri) {
    return uri.queryParameters.containsKey('code') ||
        uri.queryParameters.containsKey('error_description') ||
        uri.fragment.contains('access_token') ||
        uri.fragment.contains('error_description');
  }

  bool _isPasswordRecovery(String? redirectType) {
    final normalized = redirectType?.toLowerCase();
    return normalized == 'recovery' || normalized == 'password_recovery';
  }

  Future<void> dispose() async {
    await _subscription?.cancel();
    _subscription = null;
  }
}

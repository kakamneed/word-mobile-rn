import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../supabase/auth_session_manager.dart';

enum AuthEntryMode { signIn, signUp }

class AuthScreen extends StatefulWidget {
  const AuthScreen({
    super.key,
    this.onAuthChanged,
    this.sdk,
    this.initialMode = AuthEntryMode.signIn,
  });

  final void Function(AuthAccountState authState)? onAuthChanged;
  final WordSdk? sdk;
  final AuthEntryMode initialMode;

  @override
  State<AuthScreen> createState() => _AuthScreenState();
}

class _AuthScreenState extends State<AuthScreen> {
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  late final AuthSessionManager _sessionManager;

  bool _loading = true;
  bool _submitting = false;
  late bool _signupMode;
  AuthAccountState _authState = const AuthAccountState.uninitialized();

  @override
  void initState() {
    super.initState();
    _sessionManager = AuthSessionManager(
      localDataOwner: widget.sdk == null
          ? null
          : RustLocalDataOwnerGateway(widget.sdk!.localDataOwner),
      restoreCloudData: widget.sdk?.sync.restoreCloudDataToLocal,
      backfillLocalLearning: widget.sdk?.sync.backfillLocalLearningToCloud,
    );
    _signupMode = widget.initialMode == AuthEntryMode.signUp;
    _restore();
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _restore() async {
    setState(() {
      _loading = true;
      _authState = const AuthAccountState.checking();
    });
    final authState = await _sessionManager.resolveStartupState();
    if (mounted) {
      setState(() {
        _authState = authState;
        _loading = false;
      });
    }
  }

  Future<void> _submit() async {
    setState(() {
      _submitting = true;
      _authState = const AuthAccountState.checking();
    });

    final email = _emailController.text.trim();
    final password = _passwordController.text;
    final authState = _signupMode
        ? await _sessionManager.signUp(email: email, password: password)
        : await _sessionManager.signIn(email: email, password: password);
    if (mounted) {
      setState(() {
        _authState = authState;
      });
    }
    widget.onAuthChanged?.call(authState);
    if (authState.isSignedIn && mounted) {
      Navigator.of(context).pop(authState);
      return;
    }
    if (mounted) {
      setState(() {
        _submitting = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_authState.phase == AuthAccountPhase.notConfigured) {
      final body = const Padding(
        padding: EdgeInsets.all(24),
        child: Text(
          '账号服务尚未配置。本地学习仍可使用，但登录和云同步需要先配置 SUPABASE_URL 与 SUPABASE_ANON_KEY。',
        ),
      );
      return Scaffold(
        appBar: AppBar(title: const Text('账号')),
        body: body,
      );
    }

    final body = _loading
        ? const Center(child: CircularProgressIndicator())
        : ListView(
            padding: const EdgeInsets.all(16),
            children: [
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        _accountTitle,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        _authState.userEmail ??
                            _authMessage(_authState) ??
                            '当前没有登录账号',
                      ),
                      if (_authState.phase ==
                          AuthAccountPhase.guestLocalOnly) ...[
                        const SizedBox(height: 8),
                        const Text('你可以先继续本地学习。登录后，云同步会在数据绑定检查完成后开启。'),
                      ],
                      if (_authState.phase ==
                          AuthAccountPhase.signedOutRetainedLocal) ...[
                        const SizedBox(height: 8),
                        const Text('本机学习数据已保留。重新登录后可继续连接云端能力。'),
                      ],
                      if (_authState.phase ==
                          AuthAccountPhase.signedInNeedsBind) ...[
                        const SizedBox(height: 8),
                        const Text('已登录。云同步会在本机和云端数据检查完成后开启。'),
                      ],
                      const SizedBox(height: 8),
                      Text(
                        '本地学习：'
                        '${_authState.allowsLocalStudy ? '可用' : '不可用'}',
                      ),
                      Text(
                        '云端能力：'
                        '${_authState.allowsCloudWork ? '可用' : '暂未开启'}',
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 16),
              TextField(
                controller: _emailController,
                keyboardType: TextInputType.emailAddress,
                decoration: const InputDecoration(
                  labelText: '邮箱',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _passwordController,
                obscureText: true,
                decoration: const InputDecoration(
                  labelText: '密码',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              FilledButton(
                onPressed: _submitting ? null : _submit,
                child: Text(
                  _submitting ? '提交中...' : (_signupMode ? '注册账号' : '登录'),
                ),
              ),
              const SizedBox(height: 8),
              TextButton(
                onPressed: _submitting
                    ? null
                    : () {
                        setState(() {
                          _signupMode = !_signupMode;
                        });
                      },
                child: Text(_signupMode ? '已有账号，去登录' : '没有账号，去注册'),
              ),
              if (_authState.message != null) ...[
                const SizedBox(height: 16),
                Text(
                  _authMessage(_authState) ?? _authState.message!,
                  style: TextStyle(
                    color: _authState.phase == AuthAccountPhase.error
                        ? Theme.of(context).colorScheme.error
                        : null,
                  ),
                ),
              ],
            ],
          );

    return Scaffold(
      appBar: AppBar(title: Text(_signupMode ? '注册' : '登录')),
      body: body,
    );
  }

  String get _accountTitle {
    return switch (_authState.phase) {
      AuthAccountPhase.uninitialized => '账号未检查',
      AuthAccountPhase.checking => '正在检查账号',
      AuthAccountPhase.notConfigured => '账号服务未配置',
      AuthAccountPhase.guestLocalOnly => '游客本地模式',
      AuthAccountPhase.signedInNeedsBind => '已登录，正在检查数据',
      AuthAccountPhase.signedInActive => '已登录',
      AuthAccountPhase.signedInExpired => '登录已过期',
      AuthAccountPhase.signedOutRetainedLocal => '已退出，本机数据已保留',
      AuthAccountPhase.accountDeletedOrRevoked => '账号访问受限',
      AuthAccountPhase.error => '账号异常',
    };
  }

  String? _authMessage(AuthAccountState state) {
    final message = state.message;
    if (message == null || message.trim().isEmpty) return null;
    if (state.phase != AuthAccountPhase.error &&
        state.phase != AuthAccountPhase.guestLocalOnly &&
        state.phase != AuthAccountPhase.signedInNeedsBind &&
        state.phase != AuthAccountPhase.signedOutRetainedLocal &&
        state.phase != AuthAccountPhase.notConfigured) {
      return message;
    }
    return _localizedAuthMessage(message);
  }

  String _localizedAuthMessage(String message) {
    final lower = message.toLowerCase();
    if (lower.contains('invalid login credentials') ||
        lower.contains('invalid credentials') ||
        lower.contains('email not confirmed')) {
      return '登录失败：邮箱或密码不正确。';
    }
    if (lower.contains('user already registered') ||
        lower.contains('already registered') ||
        lower.contains('already exists')) {
      return '注册失败：这个邮箱已经注册过，请直接登录。';
    }
    if (lower.contains('password') &&
        (lower.contains('weak') ||
            lower.contains('short') ||
            lower.contains('at least'))) {
      return '注册失败：密码太短或强度不够，请至少输入 6 位密码。';
    }
    if (lower.contains('email') &&
        (lower.contains('invalid') || lower.contains('format'))) {
      return '邮箱格式不正确，请检查后再试。';
    }
    if (lower.contains('signup') && lower.contains('disabled')) {
      return '注册失败：当前 Supabase 项目未开启邮箱注册。';
    }
    if (lower.contains('network') ||
        lower.contains('socket') ||
        lower.contains('timeout') ||
        lower.contains('failed host lookup')) {
      return '网络连接失败，请检查网络后重试。';
    }
    if (lower.contains('supabase environment is not configured') ||
        lower.contains('supabase not configured')) {
      return '账号服务尚未配置，请确认打包时已带上 SUPABASE_URL 和 SUPABASE_ANON_KEY。';
    }
    if (lower.contains('cloud data') || lower.contains('bound')) {
      return '已登录，但云端数据访问还未准备好，请稍后重试。';
    }
    if (lower.contains('expired') ||
        lower.contains('invalid refresh') ||
        lower.contains('invalid_grant')) {
      return '登录状态已过期，请重新登录。';
    }
    return '操作失败：${message.replaceFirst('Exception: ', '').replaceFirst('AuthException(message: ', '')}';
  }
}

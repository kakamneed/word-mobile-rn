import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../supabase/auth_session_manager.dart';
import '../widgets/crocodile_frame_animation.dart';

enum AuthEntryMode { signIn, signUp, resetPassword }

enum _AuthFormMode { signIn, signUp, forgotPassword, resetPassword }

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
  static const _passwordResetRedirect = 'wordmobile://auth/reset-password';

  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _newPasswordController = TextEditingController();
  late final AuthSessionManager _sessionManager;

  bool _loading = true;
  bool _submitting = false;
  late _AuthFormMode _mode;
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
      shouldRestoreCloudData: widget.sdk?.sync.shouldRestoreCloudData,
    );
    _mode = switch (widget.initialMode) {
      AuthEntryMode.signUp => _AuthFormMode.signUp,
      AuthEntryMode.resetPassword => _AuthFormMode.resetPassword,
      AuthEntryMode.signIn => _AuthFormMode.signIn,
    };
    _restore();
  }

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    _newPasswordController.dispose();
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
    final newPassword = _newPasswordController.text;

    final authState = switch (_mode) {
      _AuthFormMode.signUp =>
        await _sessionManager.signUp(email: email, password: password),
      _AuthFormMode.signIn =>
        await _sessionManager.signIn(email: email, password: password),
      _AuthFormMode.forgotPassword =>
        await _sessionManager.requestPasswordReset(
          email: email,
          redirectTo: _passwordResetRedirect,
        ),
      _AuthFormMode.resetPassword =>
        await _sessionManager.updatePassword(password: newPassword),
    };

    if (!mounted) return;
    setState(() {
      _authState = authState;
    });
    widget.onAuthChanged?.call(authState);

    if (authState.isSignedIn || authState.phase == AuthAccountPhase.passwordUpdated) {
      Navigator.of(context).pop(authState);
      return;
    }

    setState(() {
      _submitting = false;
    });
  }

  Future<void> _resendVerification() async {
    final email = _emailController.text.trim();
    if (email.isEmpty) {
      setState(() {
        _authState = const AuthAccountState.error('请输入邮箱后再重发验证邮件。');
      });
      return;
    }
    setState(() {
      _submitting = true;
      _authState = const AuthAccountState.checking();
    });
    final authState = await _sessionManager.resendSignupConfirmation(email: email);
    if (!mounted) return;
    setState(() {
      _authState = authState;
      _submitting = false;
    });
  }

  void _switchMode(_AuthFormMode mode) {
    setState(() {
      _mode = mode;
      _authState = const AuthAccountState.guestLocalOnly();
      _passwordController.clear();
      if (mode != _AuthFormMode.resetPassword) {
        _newPasswordController.clear();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    if (_authState.phase == AuthAccountPhase.notConfigured) {
      return const Scaffold(
        appBar: _AuthAppBar(title: '账号'),
        body: Padding(
          padding: EdgeInsets.all(24),
          child: Text(
            '账号服务尚未配置。本地学习仍可使用，但登录、注册、邮箱验证和云同步需要先配置 SUPABASE_URL 与 SUPABASE_ANON_KEY。',
          ),
        ),
      );
    }

    return Scaffold(
      appBar: _AuthAppBar(title: _title),
      body: _loading
          ? const Center(child: CrocodileLoadingAnimation(label: '正在检查账号状态...'))
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                _StatusCard(
                  title: _accountTitle,
                  message: _authMessage(_authState) ?? _authState.userEmail ?? _defaultHint,
                  allowsLocalStudy: _authState.allowsLocalStudy,
                  allowsCloudWork: _authState.allowsCloudWork,
                ),
                const SizedBox(height: 16),
                if (_mode != _AuthFormMode.resetPassword) ...[
                  TextField(
                    controller: _emailController,
                    keyboardType: TextInputType.emailAddress,
                    decoration: const InputDecoration(
                      labelText: '邮箱',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                if (_mode == _AuthFormMode.signIn ||
                    _mode == _AuthFormMode.signUp) ...[
                  TextField(
                    controller: _passwordController,
                    obscureText: true,
                    decoration: const InputDecoration(
                      labelText: '密码',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                if (_mode == _AuthFormMode.resetPassword) ...[
                  TextField(
                    controller: _newPasswordController,
                    obscureText: true,
                    decoration: const InputDecoration(
                      labelText: '新密码',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  const SizedBox(height: 12),
                ],
                FilledButton(
                  onPressed: _submitting ? null : _submit,
                  child: Text(_submitting ? '处理中...' : _primaryButtonLabel),
                ),
                const SizedBox(height: 8),
                if (_authState.phase == AuthAccountPhase.emailVerificationPending)
                  OutlinedButton(
                    onPressed: _submitting ? null : _resendVerification,
                    child: const Text('重新发送验证邮件'),
                  ),
                if (_mode == _AuthFormMode.signIn)
                  TextButton(
                    onPressed: _submitting
                        ? null
                        : () => _switchMode(_AuthFormMode.forgotPassword),
                    child: const Text('忘记密码？'),
                  ),
                if (_mode == _AuthFormMode.forgotPassword)
                  TextButton(
                    onPressed: _submitting
                        ? null
                        : () => _switchMode(_AuthFormMode.resetPassword),
                    child: const Text('已打开重置链接，设置新密码'),
                  ),
                TextButton(
                  onPressed: _submitting
                      ? null
                      : () => _switchMode(
                            _mode == _AuthFormMode.signUp
                                ? _AuthFormMode.signIn
                                : _AuthFormMode.signUp,
                          ),
                  child: Text(
                    _mode == _AuthFormMode.signUp ? '已有账号，去登录' : '没有账号，去注册',
                  ),
                ),
                if (_mode == _AuthFormMode.forgotPassword ||
                    _mode == _AuthFormMode.resetPassword)
                  TextButton(
                    onPressed: _submitting
                        ? null
                        : () => _switchMode(_AuthFormMode.signIn),
                    child: const Text('返回登录'),
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
            ),
    );
  }

  String get _title {
    return switch (_mode) {
      _AuthFormMode.signIn => '登录',
      _AuthFormMode.signUp => '注册',
      _AuthFormMode.forgotPassword => '找回密码',
      _AuthFormMode.resetPassword => '设置新密码',
    };
  }

  String get _primaryButtonLabel {
    return switch (_mode) {
      _AuthFormMode.signIn => '登录',
      _AuthFormMode.signUp => '注册账号',
      _AuthFormMode.forgotPassword => '发送重置邮件',
      _AuthFormMode.resetPassword => '保存新密码',
    };
  }

  String get _defaultHint {
    return switch (_mode) {
      _AuthFormMode.signIn => '使用已验证的邮箱账号登录。',
      _AuthFormMode.signUp => '注册后请先查收验证邮件，验证完成后再登录。',
      _AuthFormMode.forgotPassword => '输入邮箱，我们会发送一封密码重置邮件。',
      _AuthFormMode.resetPassword => '打开邮件中的重置链接后，在这里设置新密码。',
    };
  }

  String get _accountTitle {
    return switch (_authState.phase) {
      AuthAccountPhase.uninitialized => '账号未初始化',
      AuthAccountPhase.checking => '正在检查账号',
      AuthAccountPhase.notConfigured => '账号服务未配置',
      AuthAccountPhase.guestLocalOnly => '游客本地模式',
      AuthAccountPhase.emailVerificationPending => '等待邮箱验证',
      AuthAccountPhase.passwordResetEmailSent => '重置邮件已发送',
      AuthAccountPhase.passwordUpdated => '密码已更新',
      AuthAccountPhase.signedInNeedsBind => '已登录，正在检查云端数据',
      AuthAccountPhase.signedInActive => '已登录',
      AuthAccountPhase.signedInExpired => '登录已过期',
      AuthAccountPhase.signedOutRetainedLocal => '已退出登录',
      AuthAccountPhase.accountDeletedOrRevoked => '账号不可用',
      AuthAccountPhase.error => '账号异常',
    };
  }

  String? _authMessage(AuthAccountState state) {
    final message = state.message;
    if (message == null || message.trim().isEmpty) return null;
    return _localizedAuthMessage(message);
  }

  String _localizedAuthMessage(String message) {
    final lower = message.toLowerCase();
    if (lower.contains('verification email sent') ||
        lower.contains('signup request completed') ||
        lower.contains('email confirmation')) {
      return '验证邮件已发送，请到邮箱完成验证。验证完成后再回到应用登录。';
    }
    if (lower.contains('verification email resent')) {
      return '验证邮件已重新发送，请查看邮箱。';
    }
    if (lower.contains('password reset email sent')) {
      return '密码重置邮件已发送，请打开邮件中的链接后设置新密码。';
    }
    if (lower.contains('password updated')) {
      return '密码已更新，请使用新密码登录。';
    }
    if (lower.contains('email not confirmed')) {
      return '邮箱还没有验证，请先打开验证邮件完成确认。';
    }
    if (lower.contains('invalid login credentials') ||
        lower.contains('invalid credentials')) {
      return '邮箱或密码不正确，请检查后重试。';
    }
    if (lower.contains('user already registered') ||
        lower.contains('already registered') ||
        lower.contains('already exists')) {
      return '这个邮箱已经注册过，请直接登录或找回密码。';
    }
    if (lower.contains('password') &&
        (lower.contains('weak') ||
            lower.contains('short') ||
            lower.contains('at least'))) {
      return '密码强度不够，请至少使用 6 位字符。';
    }
    if (lower.contains('email') &&
        (lower.contains('invalid') || lower.contains('format'))) {
      return '邮箱格式不正确，请检查后重试。';
    }
    if (lower.contains('signup') && lower.contains('disabled')) {
      return '注册功能尚未在 Supabase 后台开启。';
    }
    if (lower.contains('rate') || lower.contains('too many')) {
      return '操作太频繁，请稍后再试。';
    }
    if (lower.contains('expired') ||
        lower.contains('invalid refresh') ||
        lower.contains('invalid_grant') ||
        lower.contains('token')) {
      return '链接或登录状态已过期，请重新发送邮件后再试。';
    }
    if (lower.contains('network') ||
        lower.contains('socket') ||
        lower.contains('timeout') ||
        lower.contains('failed host lookup')) {
      return '网络连接失败，请检查网络后重试。';
    }
    if (lower.contains('supabase environment is not configured') ||
        lower.contains('supabase not configured')) {
      return '账号服务尚未配置，需要在构建时提供 SUPABASE_URL 和 SUPABASE_ANON_KEY。';
    }
    if (lower.contains('cloud data') || lower.contains('bound')) {
      return '已登录，但云端数据检查尚未完成，本地学习仍可继续。';
    }
    if (lower.contains('word admin does not support')) {
      return '当前后端暂不支持这个账号操作。';
    }
    return message
        .replaceFirst('Exception: ', '')
        .replaceFirst('AuthException(message: ', '');
  }
}

class _AuthAppBar extends StatelessWidget implements PreferredSizeWidget {
  const _AuthAppBar({required this.title});

  final String title;

  @override
  Size get preferredSize => const Size.fromHeight(kToolbarHeight);

  @override
  Widget build(BuildContext context) => AppBar(title: Text(title));
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    required this.title,
    required this.message,
    required this.allowsLocalStudy,
    required this.allowsCloudWork,
  });

  final String title;
  final String message;
  final bool allowsLocalStudy;
  final bool allowsCloudWork;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 8),
            Text(message),
            const SizedBox(height: 12),
            Text('本地学习：${allowsLocalStudy ? '可用' : '暂不可用'}'),
            Text('云端能力：${allowsCloudWork ? '已开启' : '暂未开启'}'),
          ],
        ),
      ),
    );
  }
}

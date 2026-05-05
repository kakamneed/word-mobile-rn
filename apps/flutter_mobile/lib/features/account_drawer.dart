import 'dart:io';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import '../supabase/auth_session_manager.dart';
import 'auth_screen.dart';
import 'profile_settings_screen.dart';

class AccountDrawer extends StatefulWidget {
  const AccountDrawer({
    super.key,
    required this.appState,
    required this.profileSettings,
    this.onProfileSettingsChanged,
    this.onAuthChanged,
    this.onOpenOnboarding,
    this.onOpenLeaderboard,
    this.onOpenSettings,
  });

  final AppState appState;
  final LocalProfileSettings profileSettings;
  final Future<void> Function()? onProfileSettingsChanged;
  final VoidCallback? onAuthChanged;
  final VoidCallback? onOpenOnboarding;
  final VoidCallback? onOpenLeaderboard;
  final VoidCallback? onOpenSettings;

  @override
  State<AccountDrawer> createState() => _AccountDrawerState();
}

class _AccountDrawerState extends State<AccountDrawer> {
  bool _signingOut = false;

  Future<void> _openAuth(AuthEntryMode mode) async {
    final navigator = Navigator.of(context);
    navigator.pop();
    await navigator.push(
      MaterialPageRoute(
        builder: (_) => AuthScreen(
          sdk: widget.appState.sdk,
          initialMode: mode,
          onAuthChanged: (authState) {
            widget.appState.applyAuthState(authState);
            widget.onAuthChanged?.call();
          },
        ),
      ),
    );
  }

  Future<void> _openProfileInfo() async {
    if (!widget.appState.authState.isSignedIn) return;
    final navigator = Navigator.of(context);
    navigator.pop();
    final changed = await navigator.push<bool>(
      MaterialPageRoute(
        builder: (_) => ProfileSettingsScreen(appState: widget.appState),
      ),
    );
    if (changed == true) {
      await widget.onProfileSettingsChanged?.call();
    }
  }

  void _openSettings() {
    final onOpenSettings = widget.onOpenSettings;
    if (onOpenSettings == null) return;
    Navigator.of(context).pop();
    onOpenSettings();
  }

  void _openOnboarding() {
    final onOpenOnboarding = widget.onOpenOnboarding;
    if (onOpenOnboarding == null) return;
    Navigator.of(context).pop();
    onOpenOnboarding();
  }

  Future<void> _signOut() async {
    setState(() {
      _signingOut = true;
    });
    final sessionManager = AuthSessionManager(
      localDataOwner: RustLocalDataOwnerGateway(widget.appState.sdk.localDataOwner),
    );
    final authState = await sessionManager.signOutRetainingLocalData();
    widget.appState.applyAuthState(authState);
    widget.onAuthChanged?.call();
    if (!mounted) return;
    setState(() {
      _signingOut = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final authState = widget.appState.authState;
    final signedIn = authState.isSignedIn;
    return ListView(
      padding: EdgeInsets.zero,
      children: [
        DrawerHeader(
          margin: EdgeInsets.zero,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              InkWell(
                borderRadius: BorderRadius.circular(28),
                onTap: signedIn ? _openProfileInfo : null,
                child: CircleAvatar(
                  radius: 26,
                  backgroundColor: signedIn
                      ? widget.profileSettings.avatarColor
                      : Theme.of(context).colorScheme.surfaceContainerHighest,
                  foregroundImage: signedIn
                      ? _avatarImageProvider(widget.profileSettings)
                      : null,
                  child: widget.profileSettings.hasAvatarImage
                      ? null
                      : _avatarChild(signedIn, authState.userEmail),
                ),
              ),
              const SizedBox(height: 12),
              Text(
                signedIn ? _profileName(authState.userEmail) : '游客模式',
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 4),
              Text(
                _phaseText(authState),
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
        if (signedIn) ...[
          _OnboardingTile(
            enabled: widget.onOpenOnboarding != null,
            onTap: _openOnboarding,
          ),
          ListTile(
            leading: const Icon(Icons.account_circle_outlined),
            title: const Text('个人信息'),
            subtitle: const Text('昵称和头像'),
            onTap: _openProfileInfo,
          ),
          ListTile(
            leading: const Icon(Icons.leaderboard_outlined),
            title: const Text('排行榜'),
            subtitle: const Text('预留入口，后续接入云端排行'),
            enabled: widget.onOpenLeaderboard != null,
            onTap: widget.onOpenLeaderboard,
          ),
          ListTile(
            leading: const Icon(Icons.settings_outlined),
            title: const Text('设置'),
            subtitle: const Text('主题和偏好'),
            enabled: widget.onOpenSettings != null,
            onTap: _openSettings,
          ),
          const Divider(height: 24),
          ListTile(
            leading: const Icon(Icons.logout),
            title: const Text('退出登录'),
            subtitle: const Text('保留本机学习数据'),
            enabled: !_signingOut,
            onTap: _signingOut ? null : _signOut,
          ),
        ] else ...[
          _OnboardingTile(
            enabled: widget.onOpenOnboarding != null,
            onTap: _openOnboarding,
          ),
          ListTile(
            leading: const Icon(Icons.login),
            title: const Text('登录'),
            subtitle: const Text('使用已有账号继续'),
            onTap: () => _openAuth(AuthEntryMode.signIn),
          ),
          ListTile(
            leading: const Icon(Icons.person_add_alt_1),
            title: const Text('注册'),
            subtitle: const Text('创建新账号'),
            onTap: () => _openAuth(AuthEntryMode.signUp),
          ),
        ],
      ],
    );
  }

  String _phaseText(AuthAccountState authState) {
    return switch (authState.phase) {
      AuthAccountPhase.uninitialized => '账号未初始化',
      AuthAccountPhase.checking => '正在检查账号',
      AuthAccountPhase.notConfigured => '账号服务尚未配置',
      AuthAccountPhase.guestLocalOnly => '本地游客模式',
      AuthAccountPhase.signedInNeedsBind => '已登录，等待数据绑定检查',
      AuthAccountPhase.signedInActive => '已登录',
      AuthAccountPhase.signedInExpired => '登录已过期',
      AuthAccountPhase.signedOutRetainedLocal => '已退出，保留本地数据',
      AuthAccountPhase.accountDeletedOrRevoked => '账号不可用',
      AuthAccountPhase.error => '账号异常',
    };
  }

  String _profileName(String? email) {
    final displayName = widget.profileSettings.displayName.trim();
    if (displayName.isNotEmpty) return displayName;
    return email ?? '已登录';
  }

  Widget _avatarChild(bool signedIn, String? email) {
    if (!signedIn) {
      return const Icon(Icons.person_outline);
    }
    final text = widget.profileSettings.avatarText(email);
    if (text.isEmpty) {
      return const Icon(Icons.person, color: Color(0xFF104E3D));
    }
    return Text(
      text,
      style: const TextStyle(
        color: Color(0xFF104E3D),
        fontSize: 22,
        fontWeight: FontWeight.w700,
      ),
    );
  }

  ImageProvider? _avatarImageProvider(LocalProfileSettings settings) {
    final imagePath = settings.avatarImagePath;
    if (imagePath == null || imagePath.trim().isEmpty) return null;
    final file = File(imagePath);
    if (!file.existsSync()) return null;
    return FileImage(file);
  }
}

class _OnboardingTile extends StatelessWidget {
  const _OnboardingTile({required this.enabled, required this.onTap});

  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      leading: const Icon(Icons.tips_and_updates_outlined),
      title: const Text('新手引导'),
      subtitle: const Text('重新设置词数并回顾功能'),
      enabled: enabled,
      onTap: onTap,
    );
  }
}

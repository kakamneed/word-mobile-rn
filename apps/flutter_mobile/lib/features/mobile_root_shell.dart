import 'dart:io';

import 'package:flutter/material.dart';

import '../sdk/sdk.dart';
import '../state/app_state.dart';
import '../supabase/auth_session_manager.dart';
import 'account_drawer.dart';
import 'ai_screen.dart';
import 'auth_screen.dart';
import 'croc_bti_screen.dart';
import 'leaderboard_screen.dart';
import 'onboarding_flow.dart';
import 'plan_screen.dart';
import 'profile_settings_screen.dart';
import 'reports_screen.dart';
import 'settings_screen.dart';
import 'study_screen.dart';
import 'theme_settings.dart';
import 'today_shell_screen.dart';
import 'wrong_words_screen.dart';

class MobileRootShell extends StatefulWidget {
  const MobileRootShell({
    super.key,
    required this.appState,
    required this.themeController,
  });

  final AppState appState;
  final ThemeSettingsController themeController;

  @override
  State<MobileRootShell> createState() => _MobileRootShellState();
}

class _MobileRootShellState extends State<MobileRootShell> {
  final _scaffoldKey = GlobalKey<ScaffoldState>();

  int _mainIndex = 0;
  int _todayReloadSeed = 0;
  int _wrongReloadSeed = 0;
  int _reportsReloadSeed = 0;
  int _aiReloadSeed = 0;
  int _studyReloadSeed = 0;
  int _planReloadSeed = 0;
  bool _planHasUnappliedChanges = false;
  bool _aiGenerateOnOpen = false;
  bool _aiShowPassageFirst = false;
  bool _loginPromptShown = false;
  LocalProfileSettings _profileSettings = const LocalProfileSettings();
  String? _loadedProfileUserId;

  bool _showingStudy = false;
  String _studyMode = 'newWord';
  ResumeSessionHint? _studyResumeHint;

  int get _bodyIndex => _showingStudy ? 5 : _mainIndex;

  @override
  void initState() {
    super.initState();
    widget.appState.addListener(_handleAppStateChanged);
    _loadProfileSettings();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _maybeShowLoginPrompt();
    });
  }

  @override
  void didUpdateWidget(covariant MobileRootShell oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.appState == widget.appState) return;
    oldWidget.appState.removeListener(_handleAppStateChanged);
    widget.appState.addListener(_handleAppStateChanged);
    _loadProfileSettings();
  }

  @override
  void dispose() {
    widget.appState.removeListener(_handleAppStateChanged);
    super.dispose();
  }

  void _handleAppStateChanged() {
    final currentUserId = widget.appState.authState.userId;
    if (currentUserId == _loadedProfileUserId) return;
    if (mounted) {
      setState(() {
        _todayReloadSeed++;
        _wrongReloadSeed++;
        _reportsReloadSeed++;
        _aiReloadSeed++;
        _studyReloadSeed++;
        _planReloadSeed++;
        _showingStudy = false;
        _studyResumeHint = null;
      });
    }
    _loadProfileSettings();
  }

  Future<void> _loadProfileSettings() async {
    final targetUserId = widget.appState.authState.userId;
    if (mounted) {
      setState(() {
        _profileSettings = const LocalProfileSettings();
      });
    }
    final settings = await loadLocalProfileSettings(userId: targetUserId);
    if (!mounted) return;
    if (widget.appState.authState.userId != targetUserId) return;
    setState(() {
      _loadedProfileUserId = targetUserId;
      _profileSettings = settings;
    });
  }

  Future<void> _handleProfileSettingsChanged() async {
    await _loadProfileSettings();
    if (!mounted) return;
    setState(() {
      _reportsReloadSeed++;
    });
  }

  Future<void> _switchToMain(int index) async {
    if (!mounted) return;
    if (_mainIndex == 1 && index != 1 && _planHasUnappliedChanges) {
      final leave = await _confirmLeavePlanWithUnappliedChanges();
      if (leave != true || !mounted) return;
      setState(() {
        _planHasUnappliedChanges = false;
      });
    }
    final wasShowingStudy = _showingStudy;
    setState(() {
      _showingStudy = false;
      if (wasShowingStudy) {
        _todayReloadSeed++;
        _wrongReloadSeed++;
        _reportsReloadSeed++;
      } else if (index == 0 && _mainIndex != 0) {
        _todayReloadSeed++;
      }
      if (index == 4) {
        _aiGenerateOnOpen = false;
        _aiShowPassageFirst = false;
      }
      _mainIndex = index;
    });
  }

  Future<bool?> _confirmLeavePlanWithUnappliedChanges() {
    return showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('计划未应用'),
        content: const Text('当前计划改动还没有保存或同步到今日，离开后会丢弃本次编辑。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('继续编辑'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text('离开'),
          ),
        ],
      ),
    );
  }

  Future<void> _openStudy(String mode, ResumeSessionHint? hint) async {
    if (!mounted) return;
    final effectiveHint =
        hint ?? await widget.appState.sdk.study.getResumeSessionHint();
    final effectiveMode = mode;
    final hintForScreen =
        effectiveHint.hasResume && effectiveHint.mode == effectiveMode
            ? effectiveHint
            : null;
    if (!mounted) return;
    setState(() {
      _studyReloadSeed++;
      _studyMode = effectiveMode;
      _studyResumeHint = hintForScreen;
      _showingStudy = true;
    });
  }

  Future<void> _openAi({
    bool generateOnOpen = false,
    bool showPassageFirst = false,
  }) async {
    if (!mounted) return;
    setState(() {
      _showingStudy = false;
      _mainIndex = 4;
      _aiReloadSeed++;
      _aiGenerateOnOpen = generateOnOpen;
      _aiShowPassageFirst = showPassageFirst;
    });
  }

  void _handleStudyClosed() {
    if (!mounted) return;
    setState(() {
      _showingStudy = false;
      _mainIndex = 0;
      _todayReloadSeed++;
      _wrongReloadSeed++;
      _reportsReloadSeed++;
      _aiReloadSeed++;
    });
  }

  Future<void> _openAccountDrawer() async {
    _scaffoldKey.currentState?.openEndDrawer();
  }

  Future<void> _openAuth(AuthEntryMode mode) async {
    final authState = await Navigator.of(context).push<AuthAccountState>(
      MaterialPageRoute(
        builder: (_) => AuthScreen(
          sdk: widget.appState.sdk,
          initialMode: mode,
          onAuthChanged: widget.appState.applyAuthState,
        ),
      ),
    );
    if (authState != null) {
      widget.appState.applyAuthState(authState);
      await _loadProfileSettings();
      setState(() {
        _todayReloadSeed++;
        _wrongReloadSeed++;
        _reportsReloadSeed++;
        _aiReloadSeed++;
        _showingStudy = false;
        _studyResumeHint = null;
      });
    }
  }

  Future<void> _maybeShowLoginPrompt() async {
    if (_loginPromptShown || !mounted || widget.appState.authState.isSignedIn) {
      return;
    }
    final authPhase = widget.appState.authPhase;
    if (authPhase == AuthAccountPhase.uninitialized ||
        authPhase == AuthAccountPhase.checking ||
        authPhase == AuthAccountPhase.notConfigured) {
      return;
    }
    _loginPromptShown = true;
    final action = await showDialog<_LoginPromptAction>(
      context: context,
      builder: (context) => AlertDialog(
        icon: const Icon(Icons.account_circle_outlined),
        title: const Text('登录后继续同步进度'),
        content: const Text('你当前可以直接以游客模式学习。登录或注册后，后续可保留账号资料和云端能力。'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(_LoginPromptAction.skip),
            child: const Text('跳过'),
          ),
          OutlinedButton(
            onPressed: () =>
                Navigator.of(context).pop(_LoginPromptAction.signUp),
            child: const Text('注册'),
          ),
          FilledButton(
            onPressed: () =>
                Navigator.of(context).pop(_LoginPromptAction.signIn),
            child: const Text('登录'),
          ),
        ],
      ),
    );
    if (!mounted) return;
    switch (action) {
      case _LoginPromptAction.signIn:
        await _openAuth(AuthEntryMode.signIn);
      case _LoginPromptAction.signUp:
        await _openAuth(AuthEntryMode.signUp);
      case _LoginPromptAction.skip:
      case null:
        break;
    }
  }

  Future<void> _openLeaderboard() async {
    final navigator = Navigator.of(context);
    _scaffoldKey.currentState?.closeEndDrawer();
    await navigator.push(
      MaterialPageRoute(
        builder: (_) => LeaderboardScreen(sdk: widget.appState.sdk),
      ),
    );
  }

  Future<void> _openSettings() async {
    final navigator = Navigator.of(context);
    _scaffoldKey.currentState?.closeEndDrawer();
    await navigator.push(
      MaterialPageRoute(
        builder: (_) => SettingsScreen(themeController: widget.themeController),
      ),
    );
  }

  Future<void> _openOnboarding() async {
    final navigator = Navigator.of(context);
    _scaffoldKey.currentState?.closeEndDrawer();
    final completed = await navigator.push<bool>(
      MaterialPageRoute(
        builder: (_) =>
            OnboardingFlow(appState: widget.appState, replayMode: true),
      ),
    );
    if (!mounted || completed != true) return;
    setState(() {
      _todayReloadSeed++;
      _aiReloadSeed++;
    });
  }

  Future<void> _openCrocBti() async {
    final navigator = Navigator.of(context);
    _scaffoldKey.currentState?.closeEndDrawer();
    final applied = await navigator.push<bool>(
      MaterialPageRoute(
        builder: (_) => CrocBtiScreen(
          sdk: widget.appState.sdk,
          userId: widget.appState.authState.userId,
          onApplied: () {
            if (!mounted) return;
            setState(() {
              _todayReloadSeed++;
              _wrongReloadSeed++;
              _reportsReloadSeed++;
              _aiReloadSeed++;
              _studyReloadSeed++;
              _planReloadSeed++;
              _planHasUnappliedChanges = false;
            });
          },
        ),
      ),
    );
    if (!mounted || applied != true) return;
    setState(() {
      _todayReloadSeed++;
      _wrongReloadSeed++;
      _reportsReloadSeed++;
      _aiReloadSeed++;
      _studyReloadSeed++;
      _planReloadSeed++;
      _planHasUnappliedChanges = false;
    });
  }

  void _handleWrongWordsImported() {
    if (!mounted) return;
    setState(() {
      _wrongReloadSeed++;
      _reportsReloadSeed++;
      _todayReloadSeed++;
    });
  }

  @override
  Widget build(BuildContext context) {
    final pages = [
      TodayShellScreen(
        key: const ValueKey('today'),
        appState: widget.appState,
        refreshSeed: _todayReloadSeed,
        onOpenPlan: () => _switchToMain(1),
        onOpenStudy: _openStudy,
        onOpenWrongWords: () => _switchToMain(2),
        onOpenReports: () => _switchToMain(3),
        onOpenAi: _openAi,
        onOpenAccount: _openAccountDrawer,
      ),
      PlanScreen(
        key: ValueKey('plan-$_planReloadSeed'),
        sdk: widget.appState.sdk,
        onTodayPlanApplied: () {
          if (!mounted) return;
          setState(() {
            _planHasUnappliedChanges = false;
            _todayReloadSeed++;
            _wrongReloadSeed++;
            _reportsReloadSeed++;
            _aiReloadSeed++;
            _studyReloadSeed++;
          });
        },
        onDirtyChanged: (dirty) {
          if (!mounted || _planHasUnappliedChanges == dirty) return;
          setState(() {
            _planHasUnappliedChanges = dirty;
          });
        },
      ),
      WrongWordsScreen(
        key: ValueKey('wrong-$_wrongReloadSeed'),
        sdk: widget.appState.sdk,
        onStartStudy: (mode) => _openStudy(mode, null),
      ),
      ReportsScreen(
        key: ValueKey('reports-$_reportsReloadSeed'),
        sdk: widget.appState.sdk,
      ),
      AiScreen(
        key: ValueKey('ai-$_aiReloadSeed'),
        sdk: widget.appState.sdk,
        isSignedIn: widget.appState.isSignedIn,
        generateOnOpen: _aiGenerateOnOpen,
        showPassageFirst: _aiShowPassageFirst,
        onWrongWordsImported: _handleWrongWordsImported,
      ),
      StudyScreen(
        key: ValueKey(
          'study-$_studyReloadSeed-$_studyMode-${_studyResumeHint?.current ?? 0}-${_studyResumeHint?.word ?? ''}',
        ),
        sdk: widget.appState.sdk,
        mode: _studyMode,
        resumeHint: _studyResumeHint,
        onCloseToToday: _handleStudyClosed,
        onOpenStudyMode: (mode) => _openStudy(mode, null),
      ),
    ];

    return Scaffold(
      key: _scaffoldKey,
      endDrawer: Drawer(
        child: SafeArea(
          child: AccountDrawer(
            appState: widget.appState,
            profileSettings: _profileSettings,
            onProfileSettingsChanged: _handleProfileSettingsChanged,
            onAuthChanged: () {
              _loadProfileSettings();
              setState(() {
                _todayReloadSeed++;
                _wrongReloadSeed++;
                _reportsReloadSeed++;
                _aiReloadSeed++;
                _planReloadSeed++;
                _showingStudy = false;
                _studyResumeHint = null;
              });
            },
            onOpenOnboarding: _openOnboarding,
            onOpenCrocBti: _openCrocBti,
            onOpenLeaderboard: _openLeaderboard,
            onOpenSettings: _openSettings,
          ),
        ),
      ),
      body: Stack(
        children: [
          IndexedStack(index: _bodyIndex, children: pages),
          if (!_showingStudy)
            Positioned(
            top: MediaQuery.paddingOf(context).top + 8,
            right: 12,
            child: Builder(
              builder: (context) => _AccountAvatarButton(
                authState: widget.appState.authState,
                profileSettings: _profileSettings,
                onPressed: () => _scaffoldKey.currentState?.openEndDrawer(),
              ),
            ),
            ),
        ],
      ),
      bottomNavigationBar: _showingStudy ? null : NavigationBar(
        selectedIndex: _mainIndex,
        onDestinationSelected: (index) {
          _switchToMain(index);
        },
        destinations: const [
          NavigationDestination(
            icon: Icon(Icons.today_outlined),
            selectedIcon: Icon(Icons.today),
            label: '今日',
          ),
          NavigationDestination(
            icon: Icon(Icons.tune_outlined),
            selectedIcon: Icon(Icons.tune),
            label: '计划',
          ),
          NavigationDestination(
            icon: Icon(Icons.menu_book_outlined),
            selectedIcon: Icon(Icons.menu_book),
            label: '错词',
          ),
          NavigationDestination(
            icon: Icon(Icons.query_stats_outlined),
            selectedIcon: Icon(Icons.query_stats),
            label: '报告',
          ),
          NavigationDestination(
            icon: Icon(Icons.auto_awesome_outlined),
            selectedIcon: Icon(Icons.auto_awesome),
            label: 'AI',
          ),
        ],
      ),
    );
  }
}

enum _LoginPromptAction { signIn, signUp, skip }

class _AccountAvatarButton extends StatelessWidget {
  const _AccountAvatarButton({
    required this.authState,
    required this.profileSettings,
    required this.onPressed,
  });

  final AuthAccountState authState;
  final LocalProfileSettings profileSettings;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final signedIn = authState.isSignedIn;
    final label = signedIn ? authState.userEmail ?? '账号' : '游客账号';
    return Tooltip(
      message: label,
      child: Material(
        color: Theme.of(context).colorScheme.surface.withValues(alpha: 0.85),
        shape: const CircleBorder(),
        elevation: 0,
        child: IconButton(
          onPressed: onPressed,
          padding: const EdgeInsets.all(1),
          constraints: const BoxConstraints.tightFor(width: 38, height: 38),
          splashRadius: 22,
          icon: CircleAvatar(
            radius: 18,
            backgroundColor: signedIn
                ? profileSettings.avatarColor
                : Theme.of(context).colorScheme.surfaceContainerHighest,
            foregroundImage: signedIn ? _avatarImageProvider() : null,
            child: signedIn && _avatarImageProvider() != null
                ? null
                : _avatarChild(context, signedIn),
          ),
        ),
      ),
    );
  }

  Widget _avatarChild(BuildContext context, bool signedIn) {
    if (!signedIn) {
      return Icon(
        Icons.person_outline,
        size: 22,
        color: Theme.of(context).colorScheme.onSurfaceVariant,
      );
    }
    final text = profileSettings.avatarText(authState.userEmail);
    if (text.isEmpty) {
      return const Icon(Icons.person, size: 22, color: Color(0xFF104E3D));
    }
    return Text(
      text,
      style: const TextStyle(
        color: Color(0xFF104E3D),
        fontSize: 15,
        fontWeight: FontWeight.w700,
      ),
    );
  }

  ImageProvider? _avatarImageProvider() {
    final imagePath = profileSettings.avatarImagePath;
    if (imagePath == null || imagePath.trim().isEmpty) return null;
    final file = File(imagePath);
    if (!file.existsSync()) return null;
    return FileImage(file);
  }
}

import 'package:flutter/material.dart';

import 'features/mobile_root_shell.dart';
import 'features/onboarding_flow.dart';
import 'features/theme_settings.dart';
import 'sdk/sdk.dart';
import 'state/app_state.dart';
import 'widgets/crocodile_frame_animation.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key, this.sdk});

  final WordSdk? sdk;

  @override
  Widget build(BuildContext context) {
    return _ThemeRoot(
      childBuilder: (themeController) => MaterialApp(
        title: '单词移动端',
        theme: themeController.theme,
        home: AppRoot(
          sdk: sdk ?? WordSdk(),
          themeController: themeController,
        ),
      ),
    );
  }
}

class _ThemeRoot extends StatefulWidget {
  const _ThemeRoot({required this.childBuilder});

  final Widget Function(ThemeSettingsController themeController) childBuilder;

  @override
  State<_ThemeRoot> createState() => _ThemeRootState();
}

class _ThemeRootState extends State<_ThemeRoot> {
  final _themeController = ThemeSettingsController();

  @override
  void initState() {
    super.initState();
    _themeController.load();
  }

  @override
  void dispose() {
    _themeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _themeController,
      builder: (context, _) => widget.childBuilder(_themeController),
    );
  }
}

class AppRoot extends StatefulWidget {
  const AppRoot({
    super.key,
    required this.sdk,
    required this.themeController,
  });

  final WordSdk sdk;
  final ThemeSettingsController themeController;

  @override
  State<AppRoot> createState() => _AppRootState();
}

class _AppRootState extends State<AppRoot> {
  late final AppState _appState;

  @override
  void initState() {
    super.initState();
    _appState = AppState(widget.sdk);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _appState.initialize();
    });
  }

  @override
  void dispose() {
    _appState.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: _appState,
      builder: (context, _) {
        switch (_appState.phase) {
          case AppPhase.uninitialized:
          case AppPhase.initializing:
            return const _BootstrapLoadingScreen();
          case AppPhase.onboarding:
            return OnboardingFlow(appState: _appState);
          case AppPhase.ready:
            return MobileRootShell(
              appState: _appState,
              themeController: widget.themeController,
            );
          case AppPhase.error:
            return _StartupErrorScreen(appState: _appState);
        }
      },
    );
  }
}

class _BootstrapLoadingScreen extends StatelessWidget {
  const _BootstrapLoadingScreen();

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            CrocodileFrameAnimation(
              frames: CrocodileFrameAnimation.rollFrames,
              width: 172,
              height: 132,
              frameDuration: Duration(milliseconds: 130),
              semanticLabel: 'Loading',
            ),
            SizedBox(height: 16),
            Text('Loading study engine...'),
          ],
        ),
      ),
    );
  }
}

class _StartupErrorScreen extends StatelessWidget {
  const _StartupErrorScreen({required this.appState});

  final AppState appState;

  @override
  Widget build(BuildContext context) {
    final error = appState.lastError;
    return Scaffold(
      appBar: AppBar(title: const Text('启动异常')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              '应用启动流程未能完成。',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 12),
            Text(error?.userMessage ?? '未知启动错误'),
            const SizedBox(height: 8),
            Text('错误代码：${error?.code ?? 'UNKNOWN'}'),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: () => appState.retryInitialize(),
              child: const Text('重试'),
            ),
          ],
        ),
      ),
    );
  }
}

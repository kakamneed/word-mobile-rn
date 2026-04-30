import 'package:flutter/material.dart';

import 'features/mobile_root_shell.dart';
import 'sdk/sdk.dart';
import 'state/app_state.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key, this.sdk});

  final WordSdk? sdk;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Word Mobile Flutter',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF1F6F5E)),
        useMaterial3: true,
      ),
      home: AppRoot(sdk: sdk ?? WordSdk()),
    );
  }
}

class AppRoot extends StatefulWidget {
  const AppRoot({super.key, required this.sdk});

  final WordSdk sdk;

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
            return _OnboardingScreen(appState: _appState);
          case AppPhase.ready:
            return MobileRootShell(appState: _appState);
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
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text('Initializing Rust runtime...'),
          ],
        ),
      ),
    );
  }
}

class _OnboardingScreen extends StatelessWidget {
  const _OnboardingScreen({required this.appState});

  final AppState appState;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Welcome')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'First launch setup',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 12),
            const Text(
              'This route is driven by authoritative bootstrap state, not local page defaults.',
            ),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: () => appState.completeOnboarding(),
              child: const Text('Complete onboarding'),
            ),
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
      appBar: AppBar(title: const Text('Startup error')),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Flutter shell could not complete bootstrap.',
              style: Theme.of(context).textTheme.headlineSmall,
            ),
            const SizedBox(height: 12),
            Text(error?.userMessage ?? 'Unknown startup error'),
            const SizedBox(height: 8),
            Text('Code: ${error?.code ?? 'UNKNOWN'}'),
            const SizedBox(height: 24),
            FilledButton(
              onPressed: () => appState.retryInitialize(),
              child: const Text('Retry'),
            ),
          ],
        ),
      ),
    );
  }
}
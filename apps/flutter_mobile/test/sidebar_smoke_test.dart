import 'package:flutter/material.dart';
import 'package:flutter_mobile/bridge/bridge.dart';
import 'package:flutter_mobile/features/account_drawer.dart';
import 'package:flutter_mobile/features/mobile_root_shell.dart';
import 'package:flutter_mobile/features/profile_settings_screen.dart';
import 'package:flutter_mobile/sdk/sdk.dart';
import 'package:flutter_mobile/state/app_state.dart';
import 'package:flutter_mobile/supabase/auth_session_manager.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:supabase_flutter/supabase_flutter.dart';

void main() {
  test('root route inventory covers every Phase 10 sidebar/root entry', () {
    expect(rootRouteInventoryForTest, [
      RootRouteInventoryEntry.today,
      RootRouteInventoryEntry.plan,
      RootRouteInventoryEntry.wrongWords,
      RootRouteInventoryEntry.reports,
      RootRouteInventoryEntry.ai,
      RootRouteInventoryEntry.studyHandoff,
      RootRouteInventoryEntry.accountDrawer,
      RootRouteInventoryEntry.leaderboard,
      RootRouteInventoryEntry.settings,
      RootRouteInventoryEntry.onboardingReplay,
      RootRouteInventoryEntry.crocBti,
      RootRouteInventoryEntry.profile,
    ]);
  });

  testWidgets('guest account drawer exposes replay Croc BTI and auth entries', (
    tester,
  ) async {
    final appState = AppState(_testSdk());
    appState.applyAuthState(const AuthAccountState.guestLocalOnly());

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AccountDrawer(
            appState: appState,
            profileSettings: const LocalProfileSettings(),
            onOpenOnboarding: () {},
            onOpenCrocBti: () {},
            onOpenLeaderboard: () {},
            onOpenSettings: () {},
          ),
        ),
      ),
    );

    expect(find.byKey(accountDrawerOnboardingReplayKey), findsOneWidget);
    expect(find.byKey(accountDrawerCrocBtiKey), findsOneWidget);
    expect(find.byKey(accountDrawerCheckUpdatesKey), findsOneWidget);
    expect(find.byKey(accountDrawerSignInKey), findsOneWidget);
    expect(find.byKey(accountDrawerSignUpKey), findsOneWidget);
    expect(find.byKey(accountDrawerLeaderboardKey), findsNothing);
    expect(find.byKey(accountDrawerSettingsKey), findsNothing);
  });

  testWidgets(
    'signed-in account drawer exposes profile leaderboard settings entries',
    (tester) async {
      final appState = AppState(_testSdk());
      appState.applyAuthState(AuthAccountState.signedInActive(_validSession()));

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AccountDrawer(
              appState: appState,
              profileSettings: const LocalProfileSettings(),
              onOpenOnboarding: () {},
              onOpenCrocBti: () {},
              onOpenLeaderboard: () {},
              onOpenSettings: () {},
            ),
          ),
        ),
      );

      expect(find.byKey(accountDrawerOnboardingReplayKey), findsOneWidget);
      expect(find.byKey(accountDrawerCrocBtiKey), findsOneWidget);
      expect(find.byKey(accountDrawerProfileKey), findsOneWidget);
      expect(find.byKey(accountDrawerLeaderboardKey), findsOneWidget);
      expect(find.byKey(accountDrawerSettingsKey), findsOneWidget);
      expect(find.byKey(accountDrawerCheckUpdatesKey), findsOneWidget);
      expect(find.byKey(accountDrawerSignInKey), findsNothing);
      expect(find.byKey(accountDrawerSignUpKey), findsNothing);
    },
  );
}

WordSdk _testSdk() {
  return WordSdk.bridgeForTesting(bridge: const _NoopBridge());
}

class _NoopBridge extends RustBridge {
  const _NoopBridge();

  @override
  Future<String> call(String method, [String? argument]) async => '{}';

  @override
  Future<void> callVoid(String method, [String? argument]) async {}

  @override
  Future<void> initialize() async {}

  @override
  Future<bool> isAvailable() async => true;
}

Session _validSession() {
  return Session(
    accessToken: 'not-a-real-jwt',
    tokenType: 'bearer',
    user: const User(
      id: 'user-1',
      appMetadata: {},
      userMetadata: {},
      aud: 'authenticated',
      email: 'user@example.com',
      createdAt: '2026-05-14T00:00:00Z',
    ),
  );
}

# Bridge, Data, Sidebar, Leaderboard, And Image Workflows

## Purpose

Use this guide when changing Flutter bridge DTOs, `WordSdk`, `BridgeCodec`, `BridgeError`, SQLite-backed persistence, local data owner/auth-adjacent ownership, account drawer/sidebar entries, leaderboard, image vote mode, or reward image upload entitlement.

## Canonical Files

Flutter:

- `apps/flutter_mobile/lib/sdk/sdk.dart`
- `apps/flutter_mobile/lib/bridge/rust_bridge.dart`
- `apps/flutter_mobile/lib/bridge/bridge_codec.dart`
- `apps/flutter_mobile/lib/bridge/bridge_error.dart`
- `apps/flutter_mobile/lib/features/account_drawer.dart`
- `apps/flutter_mobile/lib/features/leaderboard_screen.dart`
- `apps/flutter_mobile/lib/sdk/reward_image_client.dart`
- `apps/flutter_mobile/lib/supabase/auth_session_manager.dart`
- `apps/flutter_mobile/lib/supabase/leaderboard_service.dart`
- `apps/flutter_mobile/test/sidebar_smoke_test.dart`
- `apps/flutter_mobile/test/leaderboard_screen_test.dart`

Native/Rust/data:

- `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java`
- `apps/flutter_mobile/android/app/src/main/kotlin/com/wordmobile/flutter_mobile/MainActivity.kt`
- `apps/flutter_mobile/ios/Runner/AppDelegate.swift`
- `crates/platform-mobile/src/bridge.rs`
- `crates/platform-mobile/src/android.rs`
- `crates/platform-mobile/src/ios.rs`
- `crates/platform-mobile/src/paths.rs`
- `crates/storage-core/src/persistence/schema.rs`

## Workflow

1. Feature screens should call `WordSdk` typed clients, not `RustBridge` directly.
2. Keep `BridgeCodec` and `BridgeError` as the Flutter protocol boundary.
3. Treat SQLite/Rust as local learning truth. Supabase/cloud is sync/remote-adjacent unless a feature is explicitly remote-only.
4. Keep Flutter leaderboard local-first through `RewardImageClient`.
5. Treat `LeaderboardService` as future remote integration unless a task explicitly activates remote leaderboard.
6. For account drawer/sidebar changes, verify signed-out and signed-in route inventories separately.
7. For image vote/upload changes, verify public list, vote success/failure, missing image fallback, entitlement failure, and create upload failure.

## Invariants

- Local leaderboard rows come from `RewardImageClient.getLocalLeaderboard`.
- Image vote mode lists reward images through `RewardImageClient.listImages`.
- Votes go through `RewardImageClient.vote` and refresh the list without leaving the page.
- Upload entitlement/create failures surface visible bridge/domain errors.
- Sidebar signed-out users see onboarding/Croc BTI/sign in/sign up; signed-in users see onboarding/Croc BTI/profile/leaderboard/settings.

## Verification

Run or record:

```powershell
cargo test -p word-platform-mobile reward --lib
cargo test -p word-platform-mobile leaderboard --lib
cargo test -p word-platform-mobile image --lib
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
D:\flutter\flutter\bin\flutter.bat test --no-pub test\sidebar_smoke_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\leaderboard_screen_test.dart -r expanded
```

## Stale Paths To Avoid

- Active Flutter feature imports of `../supabase/leaderboard_service.dart`.
- Active Flutter feature imports of `bridge/rust_bridge.dart`.
- React Native `apps/mobile/**` route references as Flutter truth.
- Empty placeholder adapters that silently report success.

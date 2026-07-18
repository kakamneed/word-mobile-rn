# Feature: Supabase

> Slug: `supabase`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Supabase is the current cloud provider for mobile account identity, cloud sync, profile data, announcements, leaderboard projections, report/AI/wrong-word restore, and operational diagnostics.

The provider must serve the product contract in `docs/features/cloud-services.md`: local-first learning, explicit account ownership, recoverable cloud failures, and provider portability.

## UX Contract

- Users see account state in Chinese and can continue local study when Supabase is unavailable.
- Signup and password recovery use in-app email OTP as the preferred flow.
- Password reset links/deep links remain fallback only.
- After verified login, the app returns to the home shell and runs local/cloud ownership checks.
- The avatar drawer exposes login/register only when signed out.
- Gated cloud actions explain that login is required.
- Supabase Free pause is treated as temporary cloud outage from the app perspective.

## Shared Domain/Data Contract

Current Supabase project facts:

- Development project ref: `pmevdtgogsudnfgxjgiz`
- URL shape: `https://pmevdtgogsudnfgxjgiz.supabase.co`
- Client config: `SUPABASE_URL` and `SUPABASE_ANON_KEY` are compile-time Dart defines sourced from `.env.supabase.local`
- Secrets: service role keys and provider secrets stay out of Flutter and git

Current Supabase responsibilities:

- Supabase Auth manages email/password identity, OTP verification, sessions, refresh, and user metadata.
- RLS policies scope user-owned tables by `auth.uid() = user_id`.
- PostgreSQL tables store portable sync projections.
- RPCs support device registration and leaderboard refresh/query.
- Storage has a private `ai-passages` bucket scaffold, though current mobile AI passage sync primarily writes rows.

Important schema/contracts are documented in `docs/aliyun/supabase-surface-inventory.md` and should remain the source for provider replacement or migration.

## Flutter Mobile Route

- Owner screen/widget: `AuthScreen`, `AccountDrawer`, `MobileRootShell`, `ProfileSettingsScreen`, `TodayShellScreen`, `LeaderboardScreen`.
- SDK/bridge calls:
  - `SupabaseAuthService` wraps Supabase Auth.
  - `AuthSessionManager` owns auth state transitions, local owner reconciliation, restore, and backfill decisions.
  - `SyncClient` owns cloud projection writes/restores.
  - Announcement and leaderboard services own their Supabase table/RPC access.
- Loading/cache/reload behavior:
  - `Supabase.initialize` runs only when config exists.
  - Startup may enter local-only mode if Supabase is not configured, paused, offline, or unreachable.
  - Auth success enters sync eligibility checks before protected cloud work is enabled.
  - Account owner changes invalidate profile/avatar/report/sync projections.
- Orientation/gesture constraints: compact OTP forms and avatar drawer are mobile-specific.
- First implementation slice: signup/login/logout, email OTP verification, password recovery OTP, profile/avatar, local owner isolation, sync restore/backfill, cloud diagnostics.
- Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: desktop account/settings window plus sync status panel.
- Shared APIs to reuse: `AuthSessionManager` phase semantics, Rust local ownership and sync contracts, provider-neutral table/projection shapes.
- Desktop-specific layout: account panel, settings section, and status bar rather than mobile drawer.
- Mobile assumptions to avoid: Android deep links, app-link fallback code, and small-screen OTP layout.
- First parity slice: email/password login, OTP signup/recovery, session restore, local owner isolation, and read-only sync status.
- Current status: planned.

## Sync And Storage

Supabase direct sync currently touches these domains:

- `plan_config` -> `plan_configs`
- `wordbook_preferences` -> `wordbook_preferences`
- `report_snapshot` -> `report_snapshots`
- `study_word_points` -> `study_word_points`
- `wrong_word_entries` -> `wrong_word_entries`
- `ai_passages` -> `ai_passages`
- `leaderboard_stats` through leaderboard RPC/summary flow
- `profiles` for display name and account support data

Rules:

- Uploads must be user-owned and idempotent.
- Study data uses compact aggregates and projections instead of every UI event.
- Reports are daily snapshot upserts.
- Wrong words and AI passages need explicit enqueue/upload when local changes happen.
- Restore must write local state, not only report a successful cloud read.
- Cloud sync remains paused until account ownership and cloud data checks pass.

Migration/provider notes:

- Official Supabase to Volcengine Supabase may preserve many API concepts, but Auth, OTP, RLS, Storage, RPC, Realtime, and Edge Function compatibility must be tested.
- Official Supabase to first-party backend/Aliyun requires replacing client-direct table/RPC writes with authenticated API routes.
- Auth migration is the hardest part. If password hashes cannot be migrated, require OTP/password-reset re-authentication at cutover.

## AI Or Provider Implications

- `ai_passages` is user-owned history and should survive account restore.
- AI provider keys should move to backend/API gateway if production AI calls are server-mediated.
- Supabase Storage should not become the default place for provider secrets or large generated content unless a file/object contract is explicitly designed.
- Volcengine/China-hosted provider choice may improve domestic latency and reachability but introduces a compatibility and migration test matrix.

## Implementation Log

- `2026-04-22`: Supabase adopted for Flutter/Rust mobile cloud rearchitecture with local-first posture.
- `2026-05-01`: Cloud SQL setup and RLS policies run in Supabase Dashboard SQL Editor.
- `2026-05-02`: Diagnostic surface showed account signed in but cloud sync paused until local/cloud checks finished.
- `2026-05-03`: Account switching revealed stale local profile/data leakage; account owner isolation and restore/backfill logic added.
- `2026-05-03`: Cloud restore displayed success while report page stayed empty; restore had to hydrate local report snapshots.
- `2026-05-04`: Leaderboard/report parsing hit legacy question type strings; payload normalization was needed.
- `2026-05-05`: Wrong-word and AI passage local updates did not always sync; explicit domain enqueue/upload became required.
- `2026-05-14`: Supabase database/memory/free resource usage reviewed. Free plan is suitable for testing, not production reliability.
- `2026-05-15`: Email verification and password recovery implemented with Chinese UI/errors.
- `2026-05-16`: Recovery links could open a blank browser and leave `AuthSessionMissingException`; app-link fallback was added, then OTP became primary.
- `2026-05-18`: Email templates should display `{{ .Token }}` for Confirm signup and Reset password.
- `2026-05-29`: Volcengine Supabase discussed. It may provide domestic regions/IPs but should be treated as a migration project, not a config-only flip.
- `2026-06-24`: Supabase Free project pause observed. Paused projects keep data but cloud features fail until resumed.
- `2026-06-25`: Supabase feature ledger consolidated.
- `2026-07-16`: Supabase-aware Flutter release deployment rebuilt a fresh APK, verified the packaged Rust bridge, installed it over wireless adb, and launched `com.wordmobile` successfully.
- `2026-07-16`: The verified APK was resent after wireless adb reconnected on a new device port; reinstall and launcher start both succeeded.
- `2026-07-16`: A release attempt exposed a Windows Gradle daemon crash under near-exhausted system commit memory with the default `-Xmx3G`; the deploy script now caps Gradle memory at `-Xmx2g` and `MaxMetaspaceSize=512m`, after which the build completed successfully.
- `2026-07-17`: The low-memory Supabase-aware release flow completed a fresh APK build, Rust bridge verification, wireless install, and launcher start; the native build emitted only unused-function warnings.
- `2026-07-17`: A subsequent low-memory release completed successfully with a fresh APK, Supabase dart-defines, Rust bridge verification, wireless install, and launcher start; the same four unused-function warnings remained non-blocking.

## Mobile Lessons Learned

- Always check `.env.supabase.local`, Dart defines, remote SQL/migrations, and Dashboard Auth settings before blaming Flutter.
- An installed APK saying "Supabase not configured" usually means it was built without dart-defines.
- Auth provider availability and product sync eligibility are separate states.
- OTP is more robust than deep-link-only recovery on Android.
- Email provider/templates must match the app flow. OTP UI requires `{{ .Token }}` visible in the email.
- Supabase Free can pause due to inactivity; the app should show recoverable network/cloud errors instead of deleting local data.
- Profile/avatar state must be scoped per user id and cleared/swapped on logout/account switch.
- RLS and schema cache issues can look like app bugs; RPCs and table access should be probed directly when new SQL ships.

## Desktop Follow-Up Notes

- Desktop should reuse the OTP auth flow first; browser callback can be added later as convenience.
- Desktop should avoid exposing Supabase internals to ordinary users. Provider-specific diagnostics can live in a developer/support panel.
- Desktop account restore must not hydrate the wrong local slot.
- If moving away from official Supabase, desktop should target the provider-neutral API instead of direct Supabase SDK calls where possible.

## Route Changes

- `2026-05-16`: Signup/password recovery changed to in-app OTP; recovery deep link remains fallback.
- `2026-05-03`: Account switch/local ownership became a hard gate for sync.
- `2026-05-05`: Study sync changed toward compact `study_word_points` plus projections rather than detailed `study_events`.
- `2026-05-29`: Provider route widened to include official Supabase, Volcengine Supabase, and first-party backend options.

## Known Pitfalls

- Do not put service-role keys or access tokens in Flutter, docs examples, commits, or logs.
- Do not assume official Supabase regions include mainland China; verify provider region and DNS/IP from the chosen project.
- Do not assume Volcengine Supabase compatibility without testing Auth OTP, RLS, RPC, Storage, and client SDK behavior.
- Do not rely on Supabase Free for production uptime because inactive projects can pause.
- Do not confuse Dashboard success/manual SQL success with mobile release build correctness; release builds still need dart-defines.
- Do not mutate or delete local data in response to transient Supabase errors.
- On Windows release builds, a Gradle daemon disappearance with a JVM crash log can be caused by system commit pressure rather than source failure; lower the deploy JVM heap before retrying and keep the no-old-APK guard enabled.

## Verification

- Mobile: historical verification includes `flutter test --no-pub test\auth_session_manager_test.dart -r expanded`, `flutter test --no-pub test\sidebar_smoke_test.dart -r expanded`, `flutter test --no-pub test\app_update_service_test.dart -r expanded`, and `flutter analyze --no-pub`.
- Mobile: `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1` completed on 2026-07-16 with Supabase dart-defines loaded, a fresh release APK generated, Rust bridge packaging verified, wireless `adb install -r` successful, and launcher start command sent successfully.
- Mobile: A follow-up wireless `adb install -r` and `adb shell monkey` resend completed successfully on 2026-07-16.
- Mobile: After the Gradle memory adjustment, the release deploy script generated a fresh APK with SHA256 `20F40C57CB8D239554F409C69AE64DCB99030D9513E9CBCBADF3F4848CDB7AF5`, installed it successfully over wireless adb, and sent the launcher command on 2026-07-16.
- Mobile: The release deploy script generated a fresh APK with SHA256 `2F2A5282FA292D54E2BD3BF79B8B00BCF1AD8934E54B8CB19D87D55199441EC4`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-17.
- Mobile: The release deploy script generated a fresh APK with SHA256 `416099A6EDECD2E8D3120E5A9DA94E363593A8E46CE42E5AC8B86126724507CE`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-17.
- Desktop: pending; should validate session restore, OTP auth, account switch isolation, and sync status parity.
- Shared/domain: use schema/RLS smoke checks, sync restore/backfill tests, and provider migration rehearsal with row counts and per-domain sample restores.

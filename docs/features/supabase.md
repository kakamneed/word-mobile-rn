# Feature: Supabase

> Slug: `supabase`
> Status: `mobile_in_progress`
> Updated: `2026-08-17`

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
- `2026-08-28`: Release helper scripts were hardened to set the Flutter/Android SDK, JDK, Gradle home, low-memory Gradle options, and Supabase/word-admin build prerequisites explicitly. Both helpers now run `flutter pub get` before building so a partial Pub cache cannot reach Gradle.
- `2026-07-16`: A release attempt exposed a Windows Gradle daemon crash under near-exhausted system commit memory with the default `-Xmx3G`; the deploy script now caps Gradle memory at `-Xmx2g` and `MaxMetaspaceSize=512m`, after which the build completed successfully.
- `2026-07-17`: The low-memory Supabase-aware release flow completed a fresh APK build, Rust bridge verification, wireless install, and launcher start; the native build emitted only unused-function warnings.
- `2026-07-17`: A subsequent low-memory release completed successfully with a fresh APK, Supabase dart-defines, Rust bridge verification, wireless install, and launcher start; the same four unused-function warnings remained non-blocking.
- `2026-07-18`: The same release flow completed successfully after a long 600.8-second Gradle assemble phase; no daemon crash occurred, and the fresh APK installed and launched over wireless adb.
- `2026-07-18`: A subsequent release completed successfully after a 535.0-second Gradle assemble phase; the fresh APK installed and launched over TLS wireless adb without a daemon crash.
- `2026-07-18`: Another release completed successfully with a fresh APK, Supabase dart-defines, Rust bridge verification, TLS wireless installation, and launcher start; the native build warnings remained unused-function warnings only.
- `2026-07-19`: Another release completed successfully after a 676.4-second Gradle assemble phase; the fresh APK installed and launched over wireless adb without a daemon crash.
- `2026-07-20`: Another Supabase-aware release completed successfully after `flutter_image_compress` dependencies were resolved; the fresh APK installed and launched over wireless adb after a 604.9-second Gradle assemble phase.
- `2026-07-23`: A release built a fresh APK successfully but stopped at installation because the same phone was connected through two adb transports; the APK was then installed and launched by explicit serial, and the deploy script gained `ANDROID_DEVICE_SERIAL` selection plus a multiple-device guard.
- `2026-07-23`: A subsequent release passed the explicit target serial `10.157.72.179:42073`; the fresh APK installed and launched successfully after a 322.9-second Gradle assemble phase.
- `2026-07-26`: A release rebuild completed after a 280.4-second Gradle assemble phase, but the APK SHA256 remained `45065B25F4D8ACE6803B03E998AF868B663F13BBB2E137675F0C4A56BDADD1A4`; the no-old-APK guard correctly refused installation or sending. The device list also changed from one to two transports during the check, so no ambiguous adb action was attempted.
- `2026-07-26`: The verified APK was explicitly reinstalled to `192.168.1.10:37051` after the user-selected retry; streamed install returned `Success` and the `com.wordmobile` launcher returned `Events injected: 1`. The second ADB transport was left untouched.
- `2026-07-27`: A release build initially failed because Android JNI wrappers referenced three newly added exam-analysis bridge functions without importing them. After the minimal import fix and a passing `cargo check -p word-platform-mobile`, the Supabase-aware build generated fresh APK SHA256 `F40B819280D38A87432950C3F6805485C77CEA3FF62124FD3966B0E37E876D4B`; installation to the online TLS transport was rejected twice by the phone, so it was not launched.
- `2026-07-28`: The Supabase-aware release build completed after 391.1 seconds and generated fresh APK SHA256 `A690A9B58BE0E229CF79D8E8E306A3C5E31CB7F4487ACAF070E8F01145665CD4`, with the packaged Rust bridge verified. Explicit installation to `192.168.1.10:41273` failed twice with exit code 1 and no adb diagnostic text despite the device remaining online; launcher verification was not run.
- `2026-07-31`: The Supabase-aware release build completed after 398.0 seconds and generated fresh APK SHA256 `C2A45F3CEA813F50F2D19627FC2C4F49FF2AF7CEB884340D2CCA6AB24B52ABB9`. The packaged Rust bridge was verified, installation to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- `2026-08-02`: The Supabase-aware release build completed after 509.0 seconds and generated fresh APK SHA256 `845D66D63E3F1E9311045AC3B72F3455A3E6208F3E972FCA826F9B70539B95AF`. The packaged Rust bridge was verified, installation to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- `2026-08-03`: The Supabase-aware release build completed after 360.0 seconds and generated fresh APK SHA256 `9B308597D00578F696F27FDDFCBFEED24BAEBB394BFFD91FCF66BEBB2190EEB7`, with the packaged Rust bridge verified. Installation to `192.168.1.10:41795` returned bare adb exit code 1; a retry then reported `device offline`, so the new APK was not installed or launched.
- `2026-08-03`: The fresh APK with SHA256 `9B308597D00578F696F27FDDFCBFEED24BAEBB394BFFD91FCF66BEBB2190EEB7` was resent explicitly to `192.168.1.10:41795`; streamed install returned `Success` and the `com.wordmobile` launcher returned `Events injected: 1`.

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
- Flutter commands can appear to hang before printing anything when the SDK cache lock is outside the current process write scope; verify with `flutter --version --verbose` and run the command with access to `D:\flutter\flutter\bin\cache` before considering an SDK reinstall.
- A missing hosted plugin directory can make Gradle fail late at Flutter plugin-loader startup even when `pubspec.lock` is valid; run `flutter pub get` before `assembleRelease`, and do not use `--no-pub` as the recovery path.
- When more than one adb transport points at the phone, never use an unqualified install; select `ANDROID_DEVICE_SERIAL` explicitly so a freshly built package is sent to the intended device.
- A successful Gradle task does not necessarily produce a new distributable artifact; compare both APK hash and timestamp before install/send, and treat unchanged hash as no new package.
- `INSTALL_FAILED_ABORTED: User rejected permissions` is a phone-side install authorization blocker; preserve the fresh APK and retry the same explicit serial after the phone accepts the prompt.
- An online wireless device can still reject or drop a large streamed APK with bare adb exit code 1; preserve the fresh hash, re-check `adb get-state`, and do not claim launch success without a separate launcher result.
- A transport may transition from `device` to `offline` between build completion and streamed install; retry only after the selected serial returns to `device` and keep the fresh APK hash tied to that attempt.
- A large APK can hang both streamed and non-streamed wireless installation without returning a result; treat timeout as unverified, preserve the fresh hash, and do not report launch success.
- `INSTALL_FAILED_ABORTED: User rejected permissions` can recur even when the selected TLS transport is online; the phone must accept the install authorization before a fresh APK can be sent.

## Verification

- Mobile: historical verification includes `flutter test --no-pub test\auth_session_manager_test.dart -r expanded`, `flutter test --no-pub test\sidebar_smoke_test.dart -r expanded`, `flutter test --no-pub test\app_update_service_test.dart -r expanded`, and `flutter analyze --no-pub`.
- Mobile: `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1` completed on 2026-07-16 with Supabase dart-defines loaded, a fresh release APK generated, Rust bridge packaging verified, wireless `adb install -r` successful, and launcher start command sent successfully.
- Mobile: A follow-up wireless `adb install -r` and `adb shell monkey` resend completed successfully on 2026-07-16.
- Mobile: After the Gradle memory adjustment, the release deploy script generated a fresh APK with SHA256 `20F40C57CB8D239554F409C69AE64DCB99030D9513E9CBCBADF3F4848CDB7AF5`, installed it successfully over wireless adb, and sent the launcher command on 2026-07-16.
- Mobile: The release deploy script generated a fresh APK with SHA256 `2F2A5282FA292D54E2BD3BF79B8B00BCF1AD8934E54B8CB19D87D55199441EC4`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-17.
- Mobile: The release deploy script generated a fresh APK with SHA256 `416099A6EDECD2E8D3120E5A9DA94E363593A8E46CE42E5AC8B86126724507CE`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-17.
- Mobile: The release deploy script generated a fresh APK with SHA256 `C24A12F17595FF374A5DF8C4896727E7EC0D310131AF2AC9AED5E5CDD9DA377D`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-18. Gradle assemble took 600.8 seconds.
- Mobile: The release deploy script generated a fresh APK with SHA256 `DE26A15877DB4BC9ECD1FB97FA9BD3B4107A604F310A07811961B4C068651EDE`, verified the Rust bridge, installed it successfully over TLS wireless adb, and sent the launcher command on 2026-07-18. Gradle assemble took 535.0 seconds.
- Mobile: The release deploy script generated a fresh APK with SHA256 `B3F003229AABAE4B555897FF8044627F7F71F9AB3D89A2A05396C116BF690EBC`, verified the Rust bridge, installed it successfully over TLS wireless adb, and sent the launcher command on 2026-07-18. Gradle assemble took 197.5 seconds.
- Mobile: The release deploy script generated a fresh APK with SHA256 `A49ED7F15A6426828628E10A0804076ACB7E81302CCD9668D815F8DD76617E9E`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-19. Gradle assemble took 676.4 seconds.
- Mobile: The release deploy script generated a fresh APK with SHA256 `A3598724CFDFFBF8992198DB374B07380EAFAD219FE93F244E735F089EBC51F2`, verified the Rust bridge, installed it successfully over wireless adb, and sent the launcher command on 2026-07-20. Gradle assemble took 604.9 seconds.
- Mobile: On 2026-07-23 the release deploy script generated a fresh APK with SHA256 `932400953B8E8987D4B479789E5CACEC867275068E291531EA66DBD74BDB4A5C` and verified the Rust bridge. The unqualified install correctly failed with multiple devices; explicit serial install and launcher start then succeeded. The updated script passed PowerShell syntax and `git diff --check`.
- Mobile: On 2026-07-23 the release deploy script generated a fresh APK with SHA256 `45065B25F4D8ACE6803B03E998AF868B663F13BBB2E137675F0C4A56BDADD1A4`, verified the Rust bridge, installed it successfully to `10.157.72.179:42073`, and sent the launcher command. Gradle assemble took 322.9 seconds.
- Mobile: On 2026-07-26 the release script loaded Supabase dart-defines and completed `flutter pub get` plus `assembleRelease` successfully, but refused install/send because the APK hash was unchanged (`45065B25F4D8ACE6803B03E998AF868B663F13BBB2E137675F0C4A56BDADD1A4`). No install or launch verification was run; the post-build adb check showed two online transports for the same phone.
- Mobile: On 2026-07-26 the existing verified APK with SHA256 `45065B25F4D8ACE6803B03E998AF868B663F13BBB2E137675F0C4A56BDADD1A4` was explicitly installed to `192.168.1.10:37051` with `adb -s`; install returned `Success` and launcher start returned `Events injected: 1`.
- Mobile: On 2026-07-27 `cargo check -p word-platform-mobile` passed after the Android bridge import fix. The release script then generated fresh APK SHA256 `F40B819280D38A87432950C3F6805485C77CEA3FF62124FD3966B0E37E876D4B` and verified the packaged Rust bridge, but both explicit `adb install -r` attempts returned `INSTALL_FAILED_ABORTED: User rejected permissions`; no launch command was successfully run.
- Mobile: On 2026-07-28 the release script generated fresh APK SHA256 `A690A9B58BE0E229CF79D8E8E306A3C5E31CB7F4487ACAF070E8F01145665CD4` and verified the Rust bridge. Two explicit `adb install -r` attempts to `192.168.1.10:41273` returned exit code 1 without diagnostics; `adb get-state` still reported `device`, but no install or launch success was proven.
- Mobile: On 2026-07-31 the release script generated fresh APK SHA256 `C2A45F3CEA813F50F2D19627FC2C4F49FF2AF7CEB884340D2CCA6AB24B52ABB9`, verified the Rust bridge, installed it successfully to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp`, and sent the launcher command successfully. Gradle assemble took 398.0 seconds.
- Mobile: On 2026-08-02 the release script generated fresh APK SHA256 `845D66D63E3F1E9311045AC3B72F3455A3E6208F3E972FCA826F9B70539B95AF`, verified the Rust bridge, installed it successfully to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp`, and sent the launcher command successfully. Gradle assemble took 509.0 seconds.
- Mobile: On 2026-08-03 the release script generated fresh APK SHA256 `9B308597D00578F696F27FDDFCBFEED24BAEBB394BFFD91FCF66BEBB2190EEB7` and verified the Rust bridge. The explicit install to `192.168.1.10:41795` failed with exit code 1, and a retry found that serial offline; no launch verification was run.
- Mobile: On 2026-08-03 a resend of APK SHA256 `9B308597D00578F696F27FDDFCBFEED24BAEBB394BFFD91FCF66BEBB2190EEB7` to `192.168.1.10:41795` returned `Success`; launcher start returned `Events injected: 1`.
- Mobile: On 2026-08-06 the release script generated fresh APK SHA256 `D490714369866C4523C974E1EBEC699C7037DFB09491BD409A108C954654CD04` and verified the Rust bridge. Streamed install to `192.168.1.10:39535` timed out after 180 seconds; `--no-streaming` retry timed out after 240 seconds. The device remained online, but install and launch were not verified.
- Mobile: On 2026-08-10 the release script generated fresh APK SHA256 `5610ED7BDC034759141B1129C5BE33E9D6DBD124228DEBE9AD45EE9C8D983C0F` after a 441.6-second Gradle assemble and verified the Rust bridge. Installation to the only online TLS transport was rejected with `INSTALL_FAILED_ABORTED: User rejected permissions`; no launcher verification was run.
- Mobile: On 2026-08-10 the fresh APK with SHA256 `5610ED7BDC034759141B1129C5BE33E9D6DBD124228DEBE9AD45EE9C8D983C0F` was resent to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp`; streamed install returned `Success` and launcher start returned `Events injected: 1`.
- Mobile: On 2026-08-11 the release script generated fresh APK SHA256 `EC35CBA8D5D40FBF8A9640163ED6ACF86B8066C9BC9FA2502DB4782E9F957D35` after a 475.9-second Gradle assemble and verified the Rust bridge. Installation to `192.168.1.10:42667` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- Mobile: On 2026-08-12 the release script loaded Supabase dart-defines, generated fresh APK SHA256 `515289BF31EE60C34FD27E61EE22CFE93F37934D523D8102DEB0DA02D057A860` after a 419.7-second Gradle assemble, and verified the Rust bridge. Installation to `192.168.1.10:43387` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- Mobile: On 2026-08-14 the release script loaded Supabase dart-defines, generated fresh APK SHA256 `393AC1292B25A87506FF238276923DE7C23297EC0E6CE38764E93C070DDEA352` after a 485.1-second Gradle assemble, and verified the Rust bridge. Installation to `192.168.1.10:38427` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- Mobile: On 2026-08-16 the release script loaded Supabase dart-defines, generated fresh APK SHA256 `BB31D577BE4A945EDDC6F4C2197993419AE349A1DCC2112A3E31622E0E92A4D4` after a 249.1-second Gradle assemble, and verified the Rust bridge. The first direct install was rejected by the phone with `INSTALL_FAILED_ABORTED: User rejected permissions`; the same fresh APK was then installed via the TLS transport with `--no-streaming`, returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- Desktop: pending; should validate session restore, OTP auth, account switch isolation, and sync status parity.
- Shared/domain: use schema/RLS smoke checks, sync restore/backfill tests, and provider migration rehearsal with row counts and per-domain sample restores.
- `2026-08-17` network-regression diagnosis: the user reported that login and cloud sync became unavailable after the Agnes networking changes. The changed `.no_proxy()` calls are confined to Rust `reqwest` AI clients; Flutter Supabase auth/sync uses `supabase_flutter` directly, and no global `HttpOverrides`, proxy, certificate override, or Android Internet-permission removal was found. The existing release APK contains the configured Supabase host and declares `android.permission.INTERNET`.
- `2026-08-17` verification and open gate: the configured Supabase `/auth/v1/settings` and `/rest/v1/profiles` probes both returned HTTP 200 with the anonymous client configuration. `auth_session_manager_test.dart` passed 19/19 and `sync_client_test.dart` passed 2/2. No wireless ADB transport was available, so the phone's exact login/sync exception, DNS/TLS state, session state, and installation of the rebuilt package could not be captured; backend/source checks do not close that device-runtime defect.
- `2026-08-17` host-network root cause: Windows contained an enabled outbound firewall rule named `codex_sandbox_offline_block_outbound` with no program restriction, all profiles enabled, and remote ranges covering nearly every non-loopback IPv4/IPv6 address. This system-wide block produced Winsock `10013` for `adb connect` and can also block Flutter/Supabase traffic from the host. The phone remained reachable by ICMP and advertised the exact `192.168.1.14:40389` ADB TLS service over mDNS, while both `Test-NetConnection` and a restarted ADB server failed at TCP connect, isolating the fault to host outbound policy rather than device addressing or pairing.
- `2026-08-17` firewall repair blocker: an automated attempt to disable the three Codex sandbox firewall rules was rejected by the safety reviewer because it would broadly weaken persistent host policy. No firewall state changed. The minimum repair for the observed non-loopback outage is explicit user authorization to disable only `codex_sandbox_offline_block_outbound`; the two loopback-specific rules are unrelated to the phone and should remain unchanged unless independently proven faulty.
- `2026-08-17` authorized firewall repair: after explicit user authorization, only `codex_sandbox_offline_block_outbound` was disabled through an elevated `netsh` process. The two loopback-specific Codex sandbox rules remained enabled. The phone's `192.168.1.14:40389` port immediately changed from Winsock `10013` to a successful TCP probe, and the paired ADB TLS device subsequently appeared online under its mDNS serial.
- `2026-08-17` Supabase-aware resend: the release deploy script loaded both Supabase dart-defines, rebuilt the Flutter APK in 290.3 seconds, verified the packaged arm64 Rust JNI library, and generated SHA256 `AB7AB97C06713A41487FC54B3A1D4F227685271AEFF27EFB708EE65151DC2F10`. Streamed installation to `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp` returned `Success`, and the launcher injected one event. Device package evidence showed `1.0.3` / version code `4`, update time `2026-08-17 16:57:17`, and a running process. The post-launch diagnostics screen correctly showed transport configured but cloud sync disabled because the active account was `guest local-only`; recent device logs contained no Supabase, DNS, TLS, or socket exception signature. Signed-in login and subsequent sync remain the user-flow acceptance gate.
- `2026-08-17` slow-login root cause and repair: a successful password sign-in still awaited `backfillLocalLearningToCloud`, which enqueues up to 365 days of local projections and flushes pending rows serially. This coupled the login page's submitting state to a multi-minute background synchronization job. `AuthSessionManager` now starts that backfill without awaiting it after cloud-access verification, local-owner reconciliation, and any required first restore have completed, so account safety gates remain blocking while routine upload no longer blocks navigation.
- `2026-08-17` slow-login verification and deployment: the regression test first reproduced the defect by timing out while a fake backfill remained incomplete, then passed after the boundary change. The full auth manager suite passed 20/20, the sync client suite passed 2/2, and targeted Flutter analysis reported no issues. Device access to the configured Supabase host passed DNS/ICMP and TCP 443 checks. The Supabase-configured release build generated SHA256 `0927746D208E5BE5485BA1AE1CB6AC8C9E39171818E7D68A76678D7F936AC502` and verified the packaged arm64 Rust bridge. After two phone-side install rejections, a user-requested resend installed successfully with `--no-streaming`, launcher injection succeeded, and device package evidence showed version `1.0.3` / code `4`, update time `2026-08-17 19:06:34`, and a running process. Actual post-fix login duration remains a device user-flow acceptance gate.
- `2026-08-23` Supabase-aware build: the release script loaded Supabase dart-defines, generated fresh APK SHA256 `029485548F0927CC9251E79CB3135E5808261AFE5AB50497DCA097E122AE6C95` after a 386.4-second Gradle assemble, and verified the Rust bridge. Direct install to `192.168.1.10:37635` hung and was stopped; the same fresh APK then transferred through the TLS transport but returned `INSTALL_FAILED_ABORTED: User rejected permissions`. No launcher verification was run.
- `2026-08-23` resend: the same fresh APK SHA256 `029485548F0927CC9251E79CB3135E5808261AFE5AB50497DCA097E122AE6C95` was installed through the TLS transport with `--no-streaming`, returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- `2026-08-27` Supabase-aware build: Flutter/Gradle completed in 163.1 seconds and the Rust bridge compilation completed with the existing five unused-function warnings, but the release script detected that the APK hash was unchanged and refused to install or send the previous APK. No device installation or launcher verification was run.
- `2026-08-27` rebuild and resend: after removing only the generated APK, the Supabase-aware release script generated fresh APK SHA256 `160F31BC73FAAFBB4122FC80BDBC18700241655D07F0006EB1FDD6D73B0BF459` after a 14.0-second Gradle assemble and verified the Rust bridge. The first streamed install returned exit code 1 without diagnostics; retrying the same APK with `--no-streaming` to `192.168.1.10:41381` returned `Success`, and the `com.wordmobile` launcher returned `Events injected: 1`.
- `2026-08-27` rebuild attempt: the old generated APK was removed and the Supabase-aware release script reached the Flutter/Gradle build, but no APK appeared during the extended wait and the Java process stopped making measurable CPU progress. The build was stopped; no new APK, installation, or launcher verification was produced.
- `2026-08-28` rebuild diagnosis: adb had no online device and mDNS discovered no wireless service. A Supabase-configured build-only retry also failed to produce an APK during extended waiting; the wrapper remained active with high CPU but no observable Flutter/Gradle output, while the output directory contained only the stale `.sha1` marker. C: had about 6.9 GB free and D: about 5.0 GB free. The build was stopped; no new APK, installation, or launcher verification was produced.
- `2026-08-28` root-cause repair and build: the Flutter tool was confirmed healthy when allowed to write its SDK cache; the earlier no-output behavior was caused by cache-lock access being blocked. The first real build then exposed an incomplete Pub cache (`app_links-7.0.0` had no package directory), which was repaired by `flutter pub get`. The hardened Supabase helper completed `assembleRelease` in 309.7 seconds and generated fresh APK SHA256 `9D9579BC9207D35E53E24396FD926AF0C0B225F32E878D5BF3E751AB74F2CE91`, size 118,784,005 bytes, with `lib/arm64-v8a/libword_platform_mobile.so` present. No install, launch, or send verification was run because `adb devices -l` was empty.
- `2026-08-29` Supabase release build: the hardened helper completed dependency restoration and `assembleRelease` in 2504.0 seconds, generated a fresh APK SHA256 `DF5CE34E4DEC8BE862127658D5C2D20AEC21E227DC84D48F1B1FE74CEFC72B16`, size 118,784,005 bytes, and the packaged `lib/arm64-v8a/libword_platform_mobile.so` was present. This was a build-only turn; install, launch, and send were not run.
- `2026-08-29` Supabase release send: the fresh APK with SHA256 `DF5CE34E4DEC8BE862127658D5C2D20AEC21E227DC84D48F1B1FE74CEFC72B16` was installed to the explicitly selected TLS ADB transport `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp` with `--no-streaming`; installation returned `Success`, and `com.wordmobile` launcher start returned `Events injected: 1`. Two online transports for the same phone were present, so the unqualified device path was intentionally not used.

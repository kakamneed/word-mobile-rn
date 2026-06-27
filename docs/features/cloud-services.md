# Feature: Cloud Services

> Slug: `cloud-services`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Cloud services add account identity, cross-device continuity, protected premium actions, operational content, leaderboard/reward surfaces, and AI history recovery without breaking the local-first study experience. Cloud enhances the app; local study must remain usable when auth, network, Supabase, or a future provider is unavailable.

## UX Contract

- Users can study locally when signed out, unverified, offline, or cloud is not configured.
- Wrong-word import, AI passage generation, and reward draw require an active signed-in account.
- Account actions live behind the avatar drawer; signed-in users should not see login/register actions in the primary drawer slots.
- Profile information is separate from app/theme settings. Profile owns display name and avatar.
- Login, signup, email verification, password recovery, sign-out, sync pause, and sync errors use clear Chinese messages.
- Sign-in success returns users to the home shell automatically.
- Sync status is visible for support/debugging but must not block normal local study.
- Signing out must stop showing the previous account's profile, avatar, reports, or cloud-owned projections.

## Shared Domain/Data Contract

Cloud state is split into contracts:

- Auth/account: session, email, user id, verified state, refresh/restore, sign-out, and account-owner transitions.
- Local ownership: Rust/SQLite owns local study truth and isolates guest data from account-owned data.
- Sync projections: cloud stores portable projections of plan config, wordbook preferences, study word points, wrong words, reports, AI passages, profiles, and leaderboard summaries.
- Operational content: announcements and future admin-managed content are cloud-sourced but not local study truth.
- Provider abstraction: Flutter uses service/gateway boundaries; provider-specific calls stay behind auth/sync/announcement/leaderboard services.

| Domain | Current cloud shape | Ownership rule |
| --- | --- | --- |
| Profile | `profiles`, auth metadata display name | User-owned |
| Devices | `devices`, register/revoke RPCs | User-owned |
| Plan | `plan_configs` | User-owned latest plan projection |
| Wordbook | `wordbook_preferences` | User-owned per wordbook |
| Study aggregates | `study_word_points` | User-owned daily per-word aggregate |
| Wrong words | `wrong_word_entries` | User-owned per entry projection |
| Reports | `report_snapshots` | User-owned daily snapshot |
| AI passages | `ai_passages`, optional `ai-passages` bucket | User-owned generated artifacts |
| Leaderboard | `leaderboard_stats`, leaderboard RPCs | Owner write, public/current-user read |
| Announcements | `announcements` | Public read, admin/operator write |

## Flutter Mobile Route

- Owner screen/widget: `MobileRootShell`, `AccountDrawer`, `AuthScreen`, `ProfileSettingsScreen`, `TodayShellScreen`, `LeaderboardScreen`, report/wrong-word/AI flows.
- SDK/bridge calls: `AuthSessionManager`, `SupabaseAuthService`, `WordAdminAuthService`, profile settings service, `SyncClient`, leaderboard/announcement services, Rust local-data-owner and restore/backfill APIs.
- Loading/cache/reload behavior: local runtime starts first; cloud checks are optional and degraded states preserve local study. Account owner changes trigger scoped invalidation and local owner reconciliation.
- Orientation/gesture constraints: phone UI uses avatar drawer and single-column auth forms; OTP is preferred over deep-link-only recovery.
- First implementation slice: Supabase auth, local ownership isolation, restore/backfill, report/wrong-word/AI projections, profile/avatar, announcements, leaderboard, cloud-gated actions.
- Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: desktop account/settings area, sync status surface, and cloud content panels.
- Shared APIs to reuse: Rust local ownership model, sync projection contracts, auth/session state semantics, provider-neutral cloud concepts.
- Desktop-specific layout: persistent account/settings panel or toolbar entry; sync status in status bar or account popover.
- Mobile assumptions to avoid: Android intent filters, mobile mail app deep-link workarounds, compact drawer slot constraints.
- First parity slice: sign in/out/session restore, account ownership isolation, profile display, sync status, read-only restore of projections.
- Current status: planned.

## Sync And Storage

- Local SQLite remains authoritative for in-progress study and immediate UX.
- Auth success does not imply sync eligibility; ownership/bind checks gate product sync.
- Account A and account B must have isolated local slots.
- Cloud restore should bring back reports, study word points, wrong words, AI passages, profile, plan config, and wordbook preferences.
- Study history is compacted into `study_word_points`, not every transient UI event.
- `study_events`, `sync_cursors`, and `sync_dead_letters` are infrastructure/future surfaces, not the primary mobile direct-sync path today.
- Wrong-word and AI passage changes must enqueue/upload when local changes happen, not only after app restart.
- Reports are daily snapshot upserts.

## AI Or Provider Implications

- AI passage generation is account-gated and writes user-owned `ai_passages` rows.
- AI provider keys and relay URLs do not belong in Supabase anon-key config or Flutter feature code.
- If production AI is server-mediated, provider routing belongs behind an authenticated API.
- Generated artifacts should stay small projection rows until large object storage is explicitly needed.

## Implementation Log

- `2026-04-22`: Flutter + Rust + Supabase rearchitecture established local-first account and sync posture.
- `2026-05-01`: Supabase SQL setup and RLS applied manually through Dashboard SQL Editor.
- `2026-05-02`: Diagnostics showed sync paused until local/cloud ownership checks; later sync status surfaced in Today diagnostics.
- `2026-05-03`: Account switching exposed stale account data; local owner reconciliation and guest/account slots became required.
- `2026-05-03`: Report restore had to hydrate local report snapshots, not only mark cloud restore success.
- `2026-05-05`: Wrong-word and AI passage local updates needed explicit cloud enqueue/upload.
- `2026-05-14`: Supabase free resources reviewed; disk was enough for testing, memory and pause behavior are production risks.
- `2026-05-15`: Email verification, password reset, and Chinese auth errors implemented.
- `2026-05-16`: Deep-link recovery blank-page/session issue led to OTP-first mobile auth.
- `2026-05-18`: Supabase email templates should show `{{ .Token }}` for signup and recovery OTP flows.
- `2026-05-29`: Volcengine Supabase discussed as China-hosted compatible option; migration needs rehearsal.
- `2026-06-24`: Supabase Free project pause observed; production should not depend on free-plan uptime.
- `2026-06-25`: Cloud-services ledger consolidated.

## Mobile Lessons Learned

- Supabase not configured, network unavailable, and auth failure must degrade to local study.
- Login success is not sync success; local/cloud ownership checks still run.
- Deep-link-only recovery is fragile on Android; OTP is the safer primary flow.
- Supabase default email is fine for smoke tests but production needs SMTP/provider decisions.
- Error messages must distinguish duplicate email, invalid credentials, unconfirmed email, weak password, expired code, rate limit, and network failure.
- Profile/avatar caches must be scoped by user id.
- Cloud payloads need forward/backward compatibility for legacy question-type strings.

## Desktop Follow-Up Notes

- Desktop should reuse typed auth phases and Rust ownership contracts.
- Desktop can expose wider account/sync status surfaces than mobile.
- Desktop should start with OTP auth parity, then optionally add browser callback.
- Desktop must respect account slots before restore/backfill.
- Desktop should target provider-neutral APIs if the backend moves away from official Supabase.

## Route Changes

- `2026-05-16`: Signup/password recovery switched from link-first to OTP-first; deep links remain fallback.
- `2026-05-03`: Account data ownership isolation became a hard requirement.
- `2026-05-05`: Study sync moved toward compact aggregates/projections instead of detailed UI events.
- `2026-05-29`: Provider route widened to official Supabase, Volcengine Supabase, and first-party backend options.
- `2026-06-25`: This file is the umbrella record; Supabase-specific details live in `docs/features/supabase.md`.

## Known Pitfalls

- Do not commit service-role keys, access tokens, refresh tokens, AI keys, or SMTP secrets.
- Do not validate cloud with a build missing `SUPABASE_URL` and `SUPABASE_ANON_KEY` dart-defines.
- Do not let Flutter feature pages bypass Rust/domain sync ownership.
- Do not clear or overwrite account A cloud/local data when switching to account B or guest.
- Do not rely on Supabase Free staying awake; pause breaks auth, OTP, REST, RPC, and sync until resumed.
- Do not treat report snapshots as primary learning truth.

## Verification

- Mobile: historical checks include `flutter test --no-pub test\auth_session_manager_test.dart -r expanded`, `flutter test --no-pub test\sidebar_smoke_test.dart -r expanded`, `flutter test --no-pub test\app_update_service_test.dart -r expanded`, and `flutter analyze --no-pub` after auth/OTP changes.
- Desktop: pending; first verification should cover session restore, account isolation, profile display, sync status, and read-only cloud restore.
- Shared/domain: sync/restore tests should cover plan config, wordbook preferences, study word points, wrong words, report snapshots, AI passages, and account owner transitions.

# Feature: Sidebar Features

> Slug: `sidebar-features`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

The sidebar is the account and utility surface for Word Mobile. It keeps secondary actions out of the main learning tabs while still making account, profile, leaderboard, settings, onboarding replay, Croc BTI, and update checks reachable from any primary page.

Based on the current conversation history, the sidebar also carries the social/profile layer that grew out of leaderboard, cloud avatar, reward image upload, and image voting work. It should help users reach those flows without turning the Today page into a settings hub.

## UX Contract

- The bottom navigation remains focused on the daily learning loop: Today, Plan, Wrong Words, Reports, and AI.
- The top-right account avatar opens the end drawer.
- The drawer header shows the current account state, display name, and avatar.
- Signed-in users can open: new user guide replay, Croc BTI, profile, leaderboard, settings, update check, and sign out.
- Guest users can open: new user guide replay, Croc BTI, sign in, sign up, and update check.
- Leaderboard stays above Settings.
- Profile owns nickname, local/cloud avatar, image upload entitlement, and local reward-image moderation test controls.
- Leaderboard owns learning leaderboards and image vote leaderboard; image vote cards show uploaded contest images, not account avatars.
- Settings owns theme and color style; highlighted app surfaces should follow the selected theme.
- Drawer labels must be clean Chinese, not mojibake or stale placeholder text.

Current mobile drawer inventory:

| Entry | Signed-in | Guest | Destination |
| --- | --- | --- | --- |
| Header avatar/profile | Yes | Header only | `ProfileSettingsScreen` when signed in |
| New user guide replay | Yes | Yes | `OnboardingFlow(replayMode: true)` |
| Croc BTI learning profile | Yes | Yes | `CrocBtiScreen` |
| Profile | Yes | No | `ProfileSettingsScreen` |
| Leaderboard | Yes | No | `LeaderboardScreen` |
| Settings | Yes | No | `SettingsScreen` |
| Check for updates | Yes | Yes | `showAppUpdateCheckDialog` |
| Sign out | Yes | No | `AuthSessionManager.signOutRetainingLocalData` |
| Sign in | No | Yes | `AuthScreen(signIn)` |
| Sign up | No | Yes | `AuthScreen(signUp)` |

## Shared Domain/Data Contract

The sidebar routes to feature owners; it should not duplicate their domain logic.

- `AuthAccountState` controls signed-in versus guest drawer inventory.
- `LocalProfileSettings` controls local nickname, avatar color, and local avatar image path.
- Supabase `profiles` now carries cloud profile fields including display name and avatar URL/storage path.
- Ordinary profile avatars use the `avatars` bucket and are compressed with the same `webp_q60_540` pipeline as user-uploaded reward images.
- Reward/image-vote uploads use the `reward-images` bucket and `reward_images` table with moderation status.
- `LeaderboardScreen` handles learning rankings, avatar display from profile avatar URL, and image vote leaderboard.
- `SettingsScreen` handles theme/color preferences and should not be folded into profile.
- `MobileRootShell.rootRouteInventoryForTest` is the route inventory guard for mobile root/sidebar entries.

## Flutter Mobile Route

- Owner shell: `MobileRootShell`
- Drawer widget: `AccountDrawer`
- Profile route: `ProfileSettingsScreen`
- Leaderboard route: `LeaderboardScreen`
- Settings route: `SettingsScreen`
- Croc BTI route: `CrocBtiScreen`
- Onboarding route: `OnboardingFlow(replayMode: true)`
- Auth route: `AuthScreen`
- Update route: `showAppUpdateCheckDialog`

Loading/cache/reload behavior:

- Auth changes must call shell callbacks and reload profile settings.
- Auth changes should invalidate or reseed Today, Plan, Wrong Words, Reports, AI, and Study state where relevant.
- Profile changes should refresh drawer/header avatar and display name.
- Leaderboard image vote state and cloud avatar display are owned by leaderboard/profile services, not by the drawer.

Small-screen constraints:

- The drawer is an end drawer opened by a compact avatar button.
- Entries must remain tappable and avoid nested cards.
- Text should be short enough for narrow devices.

Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: desktop account/settings side rail, top menu, or command surface.
- Shared APIs to reuse: auth/session state, profile settings, Supabase profile/avatar sync, leaderboard service, Croc BTI apply-plan flow, update check service.
- Desktop-specific layout: persistent or menu-based navigation rather than a mobile end drawer.
- Mobile assumptions to avoid: floating account avatar placement, bottom-tab coupling, end-drawer gestures, and mobile-only reload seed patterns.
- First parity slice: account/profile, sign in/up/out, Settings, Leaderboard, Croc BTI, onboarding/help replay, update check.
- Current status: planned.

## Sync And Storage

- Profile nickname remains local-first and can sync to `profiles.display_name`.
- Ordinary account avatars upload to `avatars` after compression and update `profiles.avatar_url`, `profiles.avatar_storage_path`, and `profiles.avatar_mime_type`.
- Reward/image-vote uploads are separate from avatars and stay in `reward-images`.
- Pending reward images should not appear in public leaderboards or draw pools until approved.
- Sign-out keeps local learning data and clears cloud session state.
- The sidebar should refresh profile state after profile edits and auth changes.

## AI Or Provider Implications

None directly for the drawer. Image moderation provider choices belong to the reward-image/upload feature, but the sidebar can route to profile/upload controls that surface pending review state.

## Implementation Log

- `2026-04-30`: Account avatar drawer was part of the Supabase mobile implementation path.
- `2026-05-06`: Croc BTI entry was added to side menu and first-run/onboarding flows.
- `2026-05-14`: Leaderboard and Settings entries were established in the drawer, with Leaderboard placed above Settings.
- `2026-05-14`: Profile settings gained local image upload entitlement and reward-image controls for testing the image voting flow.
- `2026-05-14`: User-uploaded reward images were compressed before upload and stored in `reward-images` as pending moderation items.
- `2026-05-14`: Profile avatars were separated from image voting uploads and moved to an `avatars` bucket with `profiles.avatar_url`.
- `2026-06-25`: Rebuilt this sidebar feature record from conversation history and current mobile route inventory.

## Mobile Lessons Learned

- Utility functions belong in the sidebar, not the Today first screen.
- Guest and signed-in drawer inventories must stay distinct.
- Leaderboard and Settings are separate user intents and should not be merged.
- Profile avatars and image-vote contest images are separate storage concepts.
- Image vote leaderboard should show uploaded contest images only, not account avatars.
- Cloud avatar display belongs to normal learning leaderboard/profile, not the image voting cards.
- Drawer visible labels are easy to damage through encoding issues; smoke tests should eventually assert key labels as well as widget keys.

## Desktop Follow-Up Notes

Desktop should keep the same functional grouping but use a desktop-native layout. It should avoid copying the mobile floating avatar/drawer interaction. The first desktop parity slice should focus on account/profile, settings/theme, leaderboard, Croc BTI, onboarding replay/help, update check, and auth actions.

## Route Changes

- `2026-06-25`: Sidebar feature record becomes the canonical ledger for drawer inventory and cross-platform route grouping.
- `2026-06-25`: Mobile root inventory includes Today, Plan, Wrong Words, Reports, AI, Study handoff, Account Drawer, Leaderboard, Settings, Onboarding Replay, Croc BTI, and Profile.

## Known Pitfalls

- Do not route signed-in-only profile/leaderboard/settings entries to guests unless the destination intentionally prompts auth.
- Do not duplicate feature logic inside the drawer.
- Do not mix `avatars` and `reward-images` storage semantics.
- Do not let pending reward images appear in image vote cards.
- Do not use stale subtitles like "reserved entry" after a feature is live.
- Do not allow mojibake Chinese labels in drawer or settings surfaces.

## Verification

- Mobile: `apps/flutter_mobile/test/sidebar_smoke_test.dart` covers root route inventory and signed-in/guest drawer entry presence by stable keys.
- Desktop: pending.
- Shared/domain: delegated feature contracts are verified in their feature records; sidebar-specific verification is route inventory, entry visibility, and state refresh after auth/profile changes.

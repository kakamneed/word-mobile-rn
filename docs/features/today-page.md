# Feature: Today Page

> Slug: `today-page`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Today is the mobile app's daily command center. It should answer, at a glance:

- what the user should do today
- how far each study mode has progressed
- where to adjust the plan
- what AI reading and reward content is available after completing tasks

The page should feel like a working dashboard, not a marketing or explanation screen.

## UX Contract

- The first visible content is the Today task breakdown.
- The old "plan and wordbook" summary card is removed from Today.
- The task breakdown header has an edit action that opens the Plan page.
- Task rows stay visible for plan-backed modes even when the available pool is `0/0`, so the user can understand that the mode exists but is currently unavailable.
- Study-mode progress must reflect Rust-owned persisted state after returning from study and after app restart.
- AI reading remains below the task area and should load independently from the primary Today state.
- The reward card appears below the AI reading card.
- The reward card uses a slot-machine-like interaction: a red pull ball on the left, reward image display on the right.
- Reward images are drawn only after today's tasks are complete.
- Reward draw/save is once per day. After a reward is saved or claimed for the day, app restart must not allow another draw/save.
- Reward images are displayed without filenames or image metadata.
- Saving a reward is a user action through a save button; no automatic gallery save.
- Developer diagnostics can exist but must be collapsed and not dominate the Today flow.

## Shared Domain/Data Contract

Rust remains the authority for Today state and study progress:

- `getTodayHomeState` returns authoritative `TodayHomeState`, including active plan and daily snapshot.
- `today_plan_json` is the Today-facing plan snapshot.
- `today_wordbooks_json` is the Today-facing active wordbook snapshot.
- `today_review_wordbooks_json` may exist separately, but review selection must fall back safely when stale.
- Study progress comes from persisted study sessions/results, not Flutter-local counters.
- Review candidates must come from words learned before today, never today's new-word pool.
- Submit-answer completion responses must preserve `currentQuestion: null` after the final answer.

Reward state is also durable:

- local reward assets are listed by `assets/rewards/manifest.json`
- compressed reward images currently target WebP, quality 60, long edge 540 px
- daily reward draw/claim/save state must be persisted so restart does not reset eligibility
- gallery save is explicit and should surface success/failure without blocking the rest of Today

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- Related study owner: `apps/flutter_mobile/lib/features/study_screen.dart`
- SDK/bridge calls:
  - `TodayClient.getTodayHomeState`
  - `PlanClient.getActivePlan` only as a fallback; Today should prefer `today.activePlan`
  - study start/submit/complete through `StudyClient`
  - reward state/draw/save through `RewardClient`
  - AI reading context/history through AI SDK clients
- Loading/cache/reload behavior:
  - first paint should depend on Today home state only
  - AI reading, reward, announcements, sync, and diagnostics hydrate section-by-section
  - returning from study completion must refresh Today from Rust truth
  - app restart must restore persisted progress rather than starting study at `1/N`
- Orientation/gesture constraints:
  - portrait phone layout
  - bottom navigation remains fixed
  - task rows must fit narrow screens without overlap
  - reward pull interaction should be tap/drag friendly; avoid tiny lever hit targets
- First implementation slice:
  - task breakdown plus plan edit entry
  - AI reading card
  - reward draw/save card
  - section-level loading instead of full-page loading
- Current status:
  - mobile in progress
  - main product shape is implemented, but reward/gallery/device smoke and daily-state edge cases need continued verification

## Tauri Desktop Route

- Owner view/window: desktop home/dashboard view.
- Shared APIs to reuse:
  - `getTodayHomeState`
  - study start/submit/complete/cancel APIs
  - reward state/draw APIs
  - AI passage context/history APIs
- Desktop-specific layout:
  - use a multi-panel dashboard rather than stacked phone cards
  - task breakdown can be a dense table/list with progress bars
  - plan edit opens a side panel or modal instead of bottom-tab navigation
  - AI reading and reward can sit in right-side panels or lower dashboard bands
- Mobile assumptions to avoid:
  - do not copy bottom navigation
  - do not copy phone-only card proportions
  - do not require drag gestures for the reward pull; desktop can use click/press with optional animation
  - do not use mobile gallery-save assumptions; desktop save should use a normal file-save/export flow
- First parity slice:
  - render authoritative Today task breakdown
  - open plan editor from task breakdown
  - start study modes from task rows
  - show AI reading/reward panels using the same Rust state
- Current status: planned.

## Sync And Storage

- Today task targets/progress are local-first and Rust-owned.
- Supabase/cloud state may restore plan, wordbooks, reports, AI reading history, leaderboard, and account metadata, but Flutter must not recompute Today snapshot from cloud data.
- Same-day plan save and apply-to-today are distinct:
  - save plan updates future/default plan intent
  - apply-to-today explicitly mutates today's snapshot
- Study results must persist during and after sessions so:
  - Today progress is correct after returning from study
  - app restart resumes at the correct progress
  - review pool can use historical learned words
- Reward draw/save state must be stored per local date to prevent repeated reward extraction after restart.

## AI Or Provider Implications

- AI reading should not block Today first paint.
- AI reading context depends on today's task completion and wrong-word/review context.
- If AI generation succeeds, Today must refresh or hydrate the AI section without requiring page navigation.
- Provider errors should be contained in the AI section and not turn the entire Today page into an error state.
- Reward image selection is not AI/provider-backed; keep it local unless future cloud reward galleries are introduced.

## Implementation Log

- `2026-04-24`: Today moved back toward the primary task flow after earlier diagnostics-heavy screens.
- `2026-04-29`: Removed the "plan and wordbook" card from Today. Added a plan edit action beside task breakdown.
- `2026-04-29`: Added the daily reward card below AI reading with left pull control and right reward image area.
- `2026-04-29`: Chose local bundled reward assets over remote storage for the initial 300-image library to avoid network latency and account/cloud coupling.
- `2026-04-29`: Reward images were compressed to WebP q60 with long edge 540 px as the current mobile balance of size and quality.
- `2026-04-30`: Reward card changed from auto-save to explicit save button. Image filenames/metadata were removed from the UI.
- `2026-05-01`: Reward lever interaction was simplified to a red pull ball; repeated visual iterations showed full arcade-lever skeuomorphism was too noisy at phone-card size.
- `2026-05-01`: Fixed reward persistence expectations: after daily save/claim, restarting the app must not allow another draw/save for the same day.
- `2026-05-02`: Review mode bug investigated after Today showed `0/0`; review target now falls back safely when review wordbook snapshots are stale or empty.
- `2026-05-02`: Review pool rule clarified: review must sample words learned before today by age distribution, not today's new words.
- `2026-05-04`: Study completion bug root-caused: Rust hint enrichment changed `currentQuestion:null` into a partial object, causing Flutter to parse a malformed final-answer response.
- `2026-05-04`: Final-answer completion path hardened in Rust and Flutter SDK so Today can receive correct post-study progress.
- `2026-05-19`: Today first paint and secondary module hydration were split to reduce visible loading and navigation jank.
- `2026-06-25`: Rebuilt this cross-platform feature ledger from the mobile implementation conversation.

## Mobile Lessons Learned

- Do not recompute Today task targets in Flutter. Prefer `getTodayHomeState` and its `activePlan`.
- Do not let the Plan page's saved plan JSON overwrite Today semantics unless the user explicitly applies it to today.
- A visible `0/0` row can be useful, but it must reflect a real empty pool, not a stale wordbook snapshot bug.
- Review pool selection must not use today's newly learned words.
- Final-answer submit responses are part of the Today loop because completion updates task progress. A malformed final response breaks Today even if study results were persisted.
- Rust enrichers must not mutate `null` JSON fields into partial objects.
- Flutter SDK parsing should treat bridge payloads defensively at the boundary and avoid direct `as String`/`cast<String>` on optional or enriched fields.
- Reward images should not auto-save. Explicit save avoids surprising gallery writes and makes permission failures recoverable.
- Gallery success should be verified by actual media-store visibility, not just a bridge success string.
- Reward state must survive restart, otherwise users can repeatedly draw/save the same day's reward.
- Today performance should use section-level hydration; AI, reward, sync, and diagnostics should not block the core task breakdown.

## Desktop Follow-Up Notes

- Desktop should preserve the same Rust-owned Today contract but use a denser dashboard layout.
- Plan edit should be a side panel or modal connected to the task breakdown edit action.
- Reward interaction should become a click/press animation rather than a phone-style drag target.
- Desktop save reward should use a file-save/export path, not Android gallery semantics.
- Desktop should keep the lesson from mobile: do not let optional enrichers mutate null completion fields.
- Desktop first parity should focus on Today task truth and study launch/completion refresh before decorative reward animation.

## Route Changes

- `2026-04-29`: Removed separate Today plan/wordbook card; Plan becomes accessible through the task breakdown edit action.
- `2026-04-29`: Reward moved under AI reading, not above the primary task breakdown.
- `2026-05-01`: Reward lever changed from complex arcade lever to simplified pull-ball interaction.
- `2026-05-19`: Today moved toward stale-while-revalidate / section hydration rather than one full-page loading gate.
- `2026-06-25`: Feature ledger now treats Today as a parent feature that links Plan, Study, AI Reading, and Reward.

## Known Pitfalls

- Do not expose raw bridge errors or raw JSON in user-facing Today cards.
- Do not block Today first paint on AI reading, reward image loading, announcements, sync, or diagnostics.
- Do not display image filenames or reward IDs on the reward image.
- Do not auto-save reward images.
- Do not use Android gallery APIs as the only success signal without checking whether media appears.
- Do not let app restart reset daily reward eligibility.
- Do not use `getActivePlan`'s saved plan JSON as the primary Today plan when `getTodayHomeState.activePlan` is available.
- Do not rebuild review mode from unlearned/global vocabulary; only use learned-before-today entries.
- Do not allow Rust JSON enrichment helpers to write into `null` response fields.
- Do not let Flutter parse partial `currentQuestion` objects after final submit.

## Verification

- Mobile:
  - `flutter analyze --no-pub` has passed after final-answer and Today-related fixes.
  - Release APK build passed after the final-answer root-cause fix.
  - Manual smoke still needed on device for: Today task breakdown, plan edit navigation, study completion returning to Today, reward draw/save/gallery visibility, reward no-repeat after restart.
- Desktop:
  - pending; no parity implementation yet.
- Shared/domain:
  - `cargo check -p word-platform-mobile` passed after Today/study completion fixes.
  - Rust regression tests covered:
    - review target fallback from stale/empty review selections
    - review pool not using unlearned general vocabulary
    - final submit hint enrichment preserving `currentQuestion:null`

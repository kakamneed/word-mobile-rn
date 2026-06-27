# Feature: Animation Feedback

> Slug: `animation`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Use lightweight motion to make app state changes legible: startup, page loading, pull refresh, reward drawing, study progress, and future graph interactions should feel alive without hiding the real learning workflow.

The animation system is a feedback layer, not a domain feature. It should communicate "work is happening", "this action completed", or "this item is interactive" while keeping task completion, sync, study sessions, and rewards owned by the shared domain layer.

## UX Contract

- Startup and page-level loading use the rolling crocodile animation.
- Pull-to-refresh uses the crawling crocodile in a soft floating popup with a loading label.
- Loading animation must not replace actual data freshness or error handling.
- Pull refresh must only trigger from intentional top-edge pull gestures. Normal scrolling, rebound, and mid-list drag should not call refresh.
- Page refresh should preserve the current page surface; it should not switch the whole page back to a full-screen loading animation.
- Sprite PNGs must have transparent backgrounds so they can sit on any theme surface or popup without visible white boxes.
- Motion should be short, loopable, interruptible, and small enough for low-end Android devices.
- Animation should be optional polish on desktop: preserve state clarity, but avoid mobile-only gesture metaphors where they do not fit.

## Shared Domain/Data Contract

Animation has no shared Rust truth model. Shared clients should depend only on the domain state that drives animation:

- app/bootstrap initializing or ready
- page data loading or refreshing
- study session loading, feedback, and completion
- reward eligibility and claim state
- graph/node selected or dragging state, when graph animation lands

No animation frame, frame index, timing value, or gesture threshold should enter Rust storage, Supabase sync, or shared SDK contracts.

## Flutter Mobile Route

- Owner screen/widget:
  - Shared animation primitives live in `apps/flutter_mobile/lib/widgets/crocodile_frame_animation.dart`.
  - `CrocodileFrameAnimation` renders frame-by-frame PNG sprite loops.
  - `CrocodileLoadingAnimation` wraps the rolling crocodile for startup and page loading.
  - `CrocodileRefreshIndicator` owns custom pull-refresh gesture detection and floating refresh feedback.
  - `CrocodileRefreshPopup` renders the crawling crocodile plus loading label.
- Assets:
  - rolling frames: `apps/flutter_mobile/assets/crocodile/roll/`
  - crawling frames: `apps/flutter_mobile/assets/crocodile/crawl/`
  - registered in `apps/flutter_mobile/pubspec.yaml`.
- SDK/bridge calls:
  - none directly. Screens keep using their existing SDK calls, for example Today bundle reload, plan load, reports load, wrong-word load, AI context load, leaderboard load.
- Loading/cache/reload behavior:
  - Initial page entry can show `CrocodileLoadingAnimation`.
  - Pull refresh must call page reload with `showFullLoading: false` or equivalent so existing content remains visible.
  - Today uses cached bundle state and only shows page loading when no bundle exists.
- Orientation/gesture constraints:
  - Mobile pull refresh is custom overscroll logic, not Flutter's default `RefreshIndicator`, because default trigger behavior caused false positives with normal scrolling and rebound.
  - Refresh should trigger only after a top-origin downward overscroll exceeds the configured threshold.
  - Keep popup away from top app controls and account avatar.
- First implementation slice:
  - rolling startup/page loading animation
  - crawling pull-refresh popup
  - transparent PNG backgrounds
  - shared widget API for page adoption
- Current status:
  - mobile in progress; implemented in Flutter screens, still needs real-device gesture tuning.

## Tauri Desktop Route

- Owner view/window:
  - A future shared desktop UI feedback module should expose loading and refresh indicators for Today, Plan, Reports, Wrong Words, AI, and Study windows/panels.
- Shared APIs to reuse:
  - reuse existing SDK/domain loading states and page reload calls.
  - do not reuse Flutter widget code or mobile gesture thresholds.
- Desktop-specific layout:
  - replace pull-to-refresh with toolbar refresh buttons, command shortcuts, or panel-level reload affordances.
  - use a small inline mascot/status indicator for loading if it does not distract from desktop density.
  - desktop should prefer subtle transitions, progress text, and stable panel placeholders.
- Mobile assumptions to avoid:
  - no top-edge overscroll gesture.
  - no bottom navigation constraints.
  - no mobile popup placement assumptions.
  - do not copy mobile sprite sizes directly; desktop density and window scaling differ.
- First parity slice:
  - desktop should show equivalent startup/page loading state and non-blocking refresh state for Today and Plan first.
- Current status:
  - planned.

## Sync And Storage

No sync or storage changes. Animation assets are packaged client-side. Persist only business outcomes, not animation state.

If future rewards or user profile settings unlock alternate mascot animations, store only the selected animation id or unlocked reward id, not frame timing or transient playback state.

## AI Or Provider Implications

AI image generation was used to create the crocodile motion frames. Once frames are accepted, the app treats them as static packaged assets.

Future AI-generated animation assets should be processed before use:

- split into individual frames
- remove or alpha-key plain backgrounds
- normalize frame bounding boxes when needed
- verify file size and low-end-device playback
- document prompt/asset provenance if the asset becomes product-critical

## Implementation Log

- `2026-04-29`: Today reward card added reward lever/image rolling animation; reward state remains persisted by Rust/local storage so animation cannot create duplicate claims.
- `2026-06-25`: Created the repo-level animation feature record.
- `2026-06-25`: Added crocodile frame assets under Flutter `assets/crocodile/roll/` and `assets/crocodile/crawl/`; registered both folders in `pubspec.yaml`.
- `2026-06-25`: Added `CrocodileFrameAnimation`, `CrocodileLoadingAnimation`, `CrocodileRefreshPopup`, and `CrocodileRefreshIndicator` for Flutter.
- `2026-06-25`: Replaced Flutter page loading indicators across Today, Plan, Wrong Words, Reports, AI, Leaderboard, Study, Auth, Profile Settings, and Croc BTI surfaces where applicable.
- `2026-06-25`: Converted near-white crocodile frame backgrounds to transparent PNG to avoid white rectangles inside refresh popups.
- `2026-06-25`: Moved pull refresh away from default Flutter `RefreshIndicator` after false positives during normal scroll and over-strict follow-up gating. Current custom logic requires top-origin downward overscroll accumulation before calling refresh.

## Mobile Lessons Learned

- Do not use default `RefreshIndicator` when the visual affordance and trigger logic need to be custom. It can show or trigger during rebound/scroll states that do not match product intent.
- Do not tune false positives only by raising thresholds. Separate the gesture source: top-origin drag, downward overscroll, accumulated distance, then refresh.
- Pull refresh must not call a loader that sets the whole page back to `_loading = true`; otherwise Plan and similar pages visually reset during refresh.
- Keep "initial loading" and "refreshing existing data" separate in state and UI.
- Generated sprite sheets often include solid or near-white backgrounds; convert frames to transparent before shipping.
- Asset animation should be shared through one widget API. Per-screen copies drift quickly and make gesture bugs harder to fix.

## Desktop Follow-Up Notes

Desktop should copy the semantic contract, not the mobile mechanism:

- show mascot/status motion only for meaningful waits
- keep content stable during refresh
- use desktop-native refresh controls rather than overscroll
- avoid large decorative animation in dense work surfaces
- expose loading and refresh states in a way that works with keyboard and mouse

Desktop should also review file sizes and image scaling before reusing Flutter PNG assets directly; a vector/Lottie/canvas equivalent may be better if desktop needs sharper scaling.

## Route Changes

- `2026-06-25`: Animation became a parent feature ledger instead of being recorded only under Today/rewards.
- `2026-06-25`: Loading feedback route split into:
  - full-screen or page-level rolling crocodile for initial load
  - floating crawling crocodile popup for pull refresh
  - domain-driven reward animation for reward claiming
- `2026-06-25`: Pull refresh moved from framework-owned trigger logic to product-owned gesture detection.

## Known Pitfalls

- Do not let animation state become business truth.
- Do not show full-screen loading for refresh when stale content is still usable.
- Do not trigger refresh from normal scroll, rebound, or mid-list drag.
- Do not ship generated PNG frames with opaque white backgrounds.
- Do not copy mobile pull-to-refresh into desktop.
- Do not make one-off animation widgets in individual screens when the shared component can cover the case.
- Do not hide network or bridge errors behind an endless mascot loop.

## Verification

- Mobile:
  - `flutter analyze --no-pub` passed after the Flutter animation widget changes in the implementation run.
  - Still needs physical-device gesture smoke testing for Today, Plan, Wrong Words, Reports, AI, and Leaderboard pull refresh.
  - Still needs visual smoke testing for transparent frame rendering on light and dark-ish surfaces.
- Desktop:
  - pending; no Tauri implementation yet.
- Shared/domain:
  - no domain contract change expected.
  - Existing business-state tests remain the source of truth for rewards, study sessions, and page data.
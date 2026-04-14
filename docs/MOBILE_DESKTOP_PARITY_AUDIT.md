# Mobile Desktop Parity Audit

## Scope

This audit compares the current mobile implementation against the desktop reference for the highest-value user-facing flows:

- Today
- Study session
- Plan editing
- Vocabulary / wordbook placement
- Wrong-word review

## Closed Gaps

- Gap: Study feedback state previously showed the next question at the top while still rendering the current answer result below.
  - Closure: Mobile study feedback now keeps the just-answered question on screen and only advances after the explicit next action.

- Gap: Leaving a study mode and returning restarted the round instead of resuming progress.
  - Closure: Native mobile bridge now keeps resumable in-memory session state per mode for the current runtime.

- Gap: Plan editing had no “apply today vs tomorrow” branch.
  - Closure: Mobile quick edit now prompts the user to apply the saved plan to today or leave it for tomorrow semantics.

- Gap: Vocabulary lived in an isolated tab, while the user wanted it merged into the plan area and wrong-word review kept independent.
  - Closure: Wordbook management is now surfaced inside the plan page, and wrong-word review has its own tab.

## Remaining Gaps

- Gap: Mobile still uses a native stub state model rather than the full shared Rust study/runtime truth.
  - Impact: Session persistence is accurate inside the current app runtime, but not yet equivalent to full desktop persistence semantics.

- Gap: Wrong-word detail, reports, and AI review surfaces are still shallower than desktop.
  - Impact: The end-to-end parity focus is now on Today / Study / Plan first, while review-center depth still needs follow-up work.

- Gap: Wordbook updates remain lightweight placeholders.
  - Impact: Information architecture is closer to desktop intent, but vocabulary sync/update semantics are not yet fully parity-complete.

## Decisions

- Keep wrong-word review as an independent destination.
- Keep vocabulary selection embedded in the plan surface instead of a primary bottom-tab destination.
- Continue using desktop semantics as the reference for study progression, plan application, and Today task truthfulness.

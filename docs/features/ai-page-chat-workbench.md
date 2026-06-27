# Feature: AI Page Chat Workbench

> Slug: `ai-page-chat-workbench`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

Turn the AI page into a chat-like workbench where users choose a function near the composer and see results in the answer stream.

## UX Contract

- Remove the old full-page card stack as the primary AI page structure.
- Keep a bottom composer.
- Place compact function chips above the composer.
- Supported first functions: AI passage viewing and wrong-word import.
- Results should appear in the conversation stream.
- Cards, when needed, should live inside the stream rather than as top-level page sections.

## Shared Domain/Data Contract

The workbench uses existing bridge/SDK contracts:

- AI passage context, generation, history, detail
- wrong-word import source picking, analysis, and commit
- style preference save/load

## Flutter Mobile Route

- Owner screen/widget: `AiScreen`.
- SDK/bridge calls: `AiClient` AI passage and wrong-word import methods.
- Loading/cache/reload behavior: generation and import should notify shell callbacks to invalidate Today, AI, and wrong-word data.
- Orientation/gesture constraints: portrait chat layout; keyboard and bottom navigation need careful spacing.
- First implementation slice: mobile structure already changed to workbench style.
- Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: desktop AI workspace.
- Shared APIs to reuse: same bridge/domain APIs behind Tauri commands.
- Desktop-specific layout: conversation stream center, tool/function rail or toolbar, larger composer, optional side inspector.
- Mobile assumptions to avoid: bottom-only controls, small function chips, phone keyboard overlap.
- First parity slice: reproduce AI passage history/view and wrong-word import text flow with desktop layout.
- Current status: not started.

## Sync And Storage

The workbench itself is mostly UI state. Generated passages, wrong-word imports, style preferences, and committed wrong words need persistence/sync through existing contracts.

## AI Or Provider Implications

AI passage mode should support style instructions conversationally and save preferences for future generation. Wrong-word import mode uses image/text recognition and should expose review before commit.

## Implementation Log

- `2026-06-25`: Initial feature record backfilled from current mobile AI workbench state and recent fixes.

## Mobile Lessons Learned

- Text encoding issues can surface as escaped Unicode or mojibake in visible UI; avoid broad string rewrites and verify touched labels.
- AI passage history must be explicitly restored from cloud-backed data, not only local current context.
- Composer and function chips need compact sizing to avoid crowding small screens.

## Desktop Follow-Up Notes

Desktop should use the workbench concept but not copy the phone bottom bar literally. A side tool rail and resizable answer pane may be more natural.

## Route Changes

- `2026-06-25`: Future AI graph or wrong-word graph should be separate feature records rather than hidden inside AI chat workbench.

## Known Pitfalls

- Do not reintroduce top-level management cards.
- Do not make AI passage mode merely a static history viewer; style conversations should affect future generation.
- Do not commit imported wrong words before user review.

## Verification

- Mobile: AI page opens as chat workbench, function chips switch modes, passage history and wrong-word import work.
- Desktop: pending.
- Shared/domain: existing APIs remain usable by both clients.

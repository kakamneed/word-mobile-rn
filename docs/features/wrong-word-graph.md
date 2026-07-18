# Feature: Wrong Word Graph

> Slug: `wrong-word-graph`
> Status: `mobile_in_progress`
> Updated: `2026-07-16`

## Product Intent

Let users explore wrong words as a relationship graph instead of a flat list. The graph should make clusters, repeated contexts, roots, similar spellings, and semantic confusion visible.

## UX Contract

The first experience should feel closer to Obsidian Graph View than to a memory palace.

- Default state shows a field of wrong-word nodes and faint relationship lines.
- Selecting a word brightens the selected node and dims unrelated nodes.
- A small radial wheel appears beside the selected word.
- The wheel can reveal relation layers:
  - white: visually similar words
  - green: synonyms or near-synonyms
  - red: co-occurrence in the same AI passage, exam, task, or wrong-answer context
  - purple: shared root, prefix, suffix, or word family
- The selected relation layer highlights matching nodes and edges.
- Users can drag words from a wrong-word rail into the graph space.
- The graph stores `x/y/z` placement:
  - `x`: semantic or topic position
  - `y`: mastery level
  - `z`: error strength or review urgency

Memory palace is intentionally separate and should later become its own mode with AI image generation.

## Shared Domain/Data Contract

Planned shared contract:

```json
{
  "nodes": [
    {
      "entryId": 123,
      "word": "facilitate",
      "primaryGloss": "浣夸究鍒?,
      "wrongCountToday": 2,
      "wrongCountTotal": 7,
      "masteryScore": 0.42,
      "urgencyScore": 0.84,
      "position": { "x": 0.12, "y": -0.4, "z": 0.9 },
      "isUserPlaced": true
    }
  ],
  "edges": [
    {
      "sourceEntryId": 123,
      "targetEntryId": 456,
      "relation": "coOccurrence",
      "weight": 0.7,
      "sourceRefs": ["aiPassage:2026-06-25"]
    }
  ]
}
```

Relation enum:

- `similarForm`
- `synonym`
- `coOccurrence`
- `rootFamily`

Initial Rust/domain work should expose graph seed data and save user placement. Relationship computation can be incremental.

Canonical contract: `docs/contracts/wrong-word-graph-contract.md`.

## Flutter Mobile Route

- Owner screen/widget: new graph entry under wrong-word or AI area, likely full-screen landscape route.
- SDK/bridge calls: new `getWrongWordGraph`, `saveWrongWordGraphPosition`, possibly `refreshWrongWordGraphRelations`.
- Loading/cache/reload behavior: load graph seed once, update node positions optimistically, refresh relationship edges after wrong-word or AI passage updates.
- Orientation/gesture constraints: force or strongly prefer landscape for graph mode; use a right-side rail for wrong words; support pan/zoom and node drag.
- First implementation slice: landscape shell with right rail, canvas, draggable nodes, persisted user placement.
- Current status: planned.

Rendering recommendation:

- Use Flutter `CustomPainter` and `InteractiveViewer`.
- Store `z` but render it as node size, opacity, shadow, and draw order.
- Avoid true 3D engine in first slice.

## Tauri Desktop Route

- Owner view/window: desktop graph workspace view.
- Shared APIs to reuse: same graph seed and position persistence contract.
- Desktop-specific layout: no forced landscape; use a large resizable canvas, left or right inspector panels, keyboard shortcuts, hover previews, and multi-select.
- Mobile assumptions to avoid: do not copy thumb-sized radial controls exactly; desktop can use radial menu plus side filter panel.
- First parity slice: render the same graph data with selectable nodes, relation filter controls, and persisted positions.
- Current status: planned from stabilized shared graph contract; reuse Rust bridge semantics and implement desktop rendering next.

## Sync And Storage

User placements should sync as user-owned state:

- `entryId`
- `x/y/z`
- `isUserPlaced`
- `updatedAt`
- client/source metadata if needed

Derived edges can be recomputed and do not need full cloud sync unless expensive or user-edited.

## AI Or Provider Implications

AI can later assist with:

- suggesting semantic clusters
- naming graph regions
- explaining why selected words are related
- proposing positions for unplaced words

Do not make AI required for core graph rendering.

## Implementation Log

- `2026-06-25`: Initial plan created from user direction. Mobile should lead with landscape graph plus right wrong-word rail.
- `2026-06-25`: Shared contract drafted in `docs/contracts/wrong-word-graph-contract.md`.
- `2026-06-25`: Mobile Rust bridge seed API added: `getWrongWordGraph` returns wrong-word nodes, empty first-pass edge list, relation legend, coordinate semantics, and viewport hint; `saveWrongWordGraphPosition` persists user `x/y/z` placement in `app_settings` under `wrong_word_graph_positions_json`. Flutter SDK now exposes typed `WrongWordGraph` models for the future landscape canvas.
- `2026-06-27`: Flutter landscape shell added in `WrongWordGraphScreen`: wrong-word notebook hub icon opens a landscape graph, the canvas supports pan/zoom and placeholder nodes, the right rail lists graph nodes, and long-press dragging a rail item onto the canvas persists `x/y/z` through the shared bridge API.
- `2026-06-27`: First focus interaction added: selected nodes get halo emphasis, unrelated nodes dim, and a relation wheel appears with white/green/red/purple affordances. The wheel is visual-only until typed relation edges are introduced.
- `2026-06-27`: Red co-occurrence relation layer started. Rust emits `coOccurrence` edges for canonical wrong words that failed in the same study session, and Flutter renders weighted red lines; AI-passage co-occurrence can be layered later from `ai_passage_history_json`.
- `2026-06-27`: Completed the first full typed relation pass. Rust now emits conservative `rootFamily` purple edges from shared prefix/suffix families, `similarForm` white edges from spelling-distance heuristics, and `synonym` green edges from shared primary glosses. Flutter maps all four relation types to their planned colors.
- `2026-06-27`: Added the first mobile node detail panel inside the graph canvas. Selecting a node now surfaces word, primary gloss, today/total error counts, relation count, and a quick review affordance without leaving graph mode.
- `2026-07-15` - Baseline reconciliation: Mobile graph changes add independent relation-type filters, filter visible edges before layout/rendering, keep directly related labels visible at practical zoom levels, and replace the long node detail list with compact relation/error summaries.
- `2026-07-15` - Problems encountered: The historical UI needed to distinguish relation categories without relying on one dense detail panel. Prior verification status is unknown from the worktree and is rechecked separately.
- `2026-07-16` - Modification points: Exercise unknown/wrong occurrences now refresh idempotent storage `same_article` pairs and project them into red graph `coOccurrence` edges with `sameArticle`, paper/article/question source evidence. The selected-node panel renders source detail rows rather than only relation counts.

## Mobile Lessons Learned

- First version should stay 2.5D for stability.
- Treat z-depth as visual priority rather than real camera depth.
- Node dragging and pan/zoom must not fight each other; use explicit drag handles or long-press if needed.
- Relation filters must affect edges, selected-node context, and label emphasis consistently; filtering only the legend is misleading.

## Desktop Follow-Up Notes

Desktop can expose richer controls without the mobile rail constraint. It should reuse the graph relation contract and avoid reimplementing relationship semantics in UI code. Recommended Tauri first slice: add a graph workspace route, call the same Rust graph read/save APIs through Tauri commands, render with a desktop canvas layer, keep a persistent inspector panel for selected node details, and expose relation toggles as toolbar chips rather than thumb-sized radial controls.

## Route Changes

- `2026-06-25`: Memory palace split out as a later independent mode.
- `2026-07-15`: Relation discovery moved from a selected-node-only wheel/detail list toward persistent category filters that can be adapted to a desktop toolbar.

## Known Pitfalls

- Do not build a generic graph database viewer.
- Do not start with true 3D unless a 2.5D prototype fails.
- Do not bury relation meaning in colors only; provide legend or accessible labels.
- Do not let user placement overwrite derived semantic placement without marking `isUserPlaced`.
- Do not keep labels hidden for nodes directly related to the selection when the current zoom level can support them.
- Do not assign passage, stem, and each choice separate article IDs; that prevents legitimate same-article relationships. Share the article ID and separate occurrence offsets instead.

## Verification

- Mobile: `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart test\wrong_word_graph_screen_test.dart --no-pub` passed all 23 tests on 2026-07-15, including the three graph screen cases for empty-space layout, non-placing rail taps, and readable relation detail.
- Mobile: graph opens in landscape, rail is visible, nodes drag into space, selected node dims unrelated nodes, relation wheel appears.
- Desktop: same graph contract renders in a desktop canvas and preserves positions.
- Shared/domain: edge relation types are stable and tested with sample wrong-word data.
- `2026-07-16`: storage relation refresh/idempotency test passed; the graph widget suite passed 3/3 including visible `sameArticle` paper/article detail. Desktop rendering was not run.

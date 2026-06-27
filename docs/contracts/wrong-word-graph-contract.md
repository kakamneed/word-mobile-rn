# Wrong Word Graph Shared Contract

Status: Draft
Owner: Rust shared core
Created: 2026-06-25
Consumers: Flutter mobile, Tauri desktop

## Purpose

Define the shared wrong-word graph contract before Flutter mobile and Tauri desktop implement their own graph views.

The graph is a study surface, not a generic graph database viewer. Rust/domain code owns graph seed data, relationship semantics, persisted placement, and sync-facing state. Clients own rendering, gestures, viewport, and platform-specific controls.

## Contract Family

| Contract | Initial bridge/API name | Scope | Side effects | Notes |
|---|---|---|---|---|
| Graph seed read | `getWrongWordGraph` | mobile + desktop | Read-only | Returns nodes, edges, relation metadata, and viewport hints |
| Node placement save | `saveWrongWordGraphPosition` | mobile + desktop | Persists user placement | Optimistic client update is allowed |
| Relation refresh | `refreshWrongWordGraphRelations` | optional later | May recompute derived edges | First version can compute on read |

## Coordinate Model

Coordinates are normalized graph-space values, not pixels.

- `x`: semantic/topic position.
- `y`: mastery level.
- `z`: error strength or review urgency.

Clients may render `z` as size, opacity, draw order, shadow, or depth. The first implementation should not require a real 3D engine.

## Node DTO

```json
{
  "entryId": 123,
  "entryKind": "word",
  "sourceEntryId": "kaoyan_123",
  "word": "facilitate",
  "primaryGloss": "make easier",
  "meanings": ["make easier", "promote"],
  "wrongCountToday": 2,
  "wrongCountTotal": 7,
  "lastWrongAt": "2026-06-25T09:30:00+08:00",
  "priorityScore": 8.4,
  "masteryScore": 0.42,
  "urgencyScore": 0.84,
  "position": {
    "x": 0.12,
    "y": -0.4,
    "z": 0.9
  },
  "isUserPlaced": true,
  "positionUpdatedAt": "2026-06-25T12:00:00+08:00",
  "sources": ["wrongWord", "aiPassage"]
}
```

### Required Node Fields

- `entryId`
- `entryKind`
- `word`
- `wrongCountToday`
- `wrongCountTotal`
- `priorityScore`
- `masteryScore`
- `urgencyScore`
- `position`
- `isUserPlaced`

### Entry Kinds

- `word`: normal vocabulary entry.
- `imported`: imported wrong word that may not have a canonical entry yet.

Imported words may use negative `entryId` values in existing mobile bridge payloads. The graph contract should preserve that compatibility until a stable imported-word id model replaces it.

## Edge DTO

```json
{
  "edgeId": "coOccurrence:123:456:aiPassage:2026-06-25",
  "sourceEntryId": 123,
  "targetEntryId": 456,
  "relation": "coOccurrence",
  "weight": 0.7,
  "sourceRefs": ["aiPassage:2026-06-25"],
  "label": "same AI passage",
  "isUserPinned": false
}
```

### Required Edge Fields

- `edgeId`
- `sourceEntryId`
- `targetEntryId`
- `relation`
- `weight`
- `sourceRefs`

## Relation Enum

| Relation | Color | Meaning | First-version source |
|---|---|---|---|
| `similarForm` | white | visually or spelling-similar words | derived by conservative spelling-distance heuristics |
| `synonym` | green | near-synonym or overlapping Chinese meanings | derived from shared primary gloss, low-confidence |
| `coOccurrence` | red | words appearing in the same AI passage, exam, task, session, or wrong-answer context | implemented from shared failed study sessions first; AI passage context can be added later |
| `rootFamily` | purple | shared root, prefix, suffix, or word family | implemented from conservative shared prefix/suffix families first |

Unknown relation values must be ignored by clients instead of crashing.

## Graph Response DTO

```json
{
  "version": 1,
  "generatedAt": "2026-06-25T12:00:00+08:00",
  "nodes": [],
  "edges": [],
  "relationLegend": [
    {
      "relation": "coOccurrence",
      "label": "same wrong context",
      "color": "#D95555"
    }
  ],
  "viewportHint": {
    "preferredOrientation": "landscape",
    "rightRail": true,
    "initialFocusEntryId": null
  }
}
```

## Placement Save Request

```json
{
  "entryId": 123,
  "entryKind": "word",
  "position": {
    "x": 0.12,
    "y": -0.4,
    "z": 0.9
  },
  "isUserPlaced": true,
  "clientUpdatedAt": "2026-06-25T12:00:00+08:00"
}
```

## Placement Save Response

```json
{
  "entryId": 123,
  "position": {
    "x": 0.12,
    "y": -0.4,
    "z": 0.9
  },
  "isUserPlaced": true,
  "positionUpdatedAt": "2026-06-25T12:00:01+08:00"
}
```

## Persistence Semantics

User placement is user-owned state and should be persisted separately from derived graph edges.

Recommended local persistence:

- `entry_id`
- `entry_kind`
- `source_entry_id`
- `x`
- `y`
- `z`
- `is_user_placed`
- `updated_at`

Derived edges may be recomputed and do not need full cloud sync in the first version.

## Sync Semantics

The first sync candidate is node placement. Derived relationships should be recomputed from shared learning data unless a future version allows user-pinned edges or manual clusters.

If syncing placement:

- use last-write-wins by `updated_at` for the same user and entry
- keep `isUserPlaced` so user placement can override automatic layout
- do not sync transient viewport state

## Mobile Rendering Contract

Flutter mobile can assume:

- graph mode prefers landscape
- a right wrong-word rail is allowed
- pan/zoom canvas is client-owned
- drag-to-place writes placement through `saveWrongWordGraphPosition`
- selected node focus is client state
- relation filters are client state

## Desktop Rendering Contract

Tauri desktop can assume:

- no forced orientation
- graph can use a large resizable canvas
- relation filters may live in a persistent side panel
- hover previews and keyboard shortcuts are desktop-only affordances
- it must consume the same node/edge/placement contract as Flutter

## Validation Rules

- `weight` must be between 0 and 1.
- `position.x`, `position.y`, and `position.z` must be finite numbers.
- Missing `position` should be filled by deterministic auto-layout.
- Clients must ignore unknown relation values.
- Clients must not persist viewport pixels as graph coordinates.

## First Implementation Order

1. Build `getWrongWordGraph` from existing wrong-word list data with deterministic auto-layout and empty edges.
2. Add local placement persistence and `saveWrongWordGraphPosition`.
3. Add `coOccurrence` edges from AI passage history and/or study session context.
4. Add `rootFamily` edges.
5. Add `similarForm` and `synonym` edges.

## Known Non-Goals

- No true 3D engine requirement.
- No memory palace mode in this contract.
- No AI dependency for the first graph render.
- No generic graph query language.

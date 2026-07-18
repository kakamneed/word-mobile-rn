# Feature: <Name>

> Slug: `<feature-slug>`
> Status: `planned | mobile_in_progress | mobile_shipped | desktop_in_progress | parity_complete`
> Updated: `YYYY-MM-DD`

## Product Intent

What user problem this feature solves.

## UX Contract

What the user should be able to see, do, and understand.

## Shared Domain/Data Contract

Shared Rust/domain/API models, bridge methods, persistence, sync payloads, or cloud tables used by both clients.

## Flutter Mobile Route

- Owner screen/widget:
- SDK/bridge calls:
- Loading/cache/reload behavior:
- Orientation/gesture constraints:
- First implementation slice:
- Current status:

## Tauri Desktop Route

- Owner view/window:
- Shared APIs to reuse:
- Desktop-specific layout:
- Mobile assumptions to avoid:
- First parity slice:
- Current status:

## Sync And Storage

Local persistence, Supabase/cloud sync, conflict rules, cache invalidation, and offline behavior.

## AI Or Provider Implications

Prompts, tools, provider routing, model fallbacks, latency, token limits, and failure modes.

## Implementation Log

- `YYYY-MM-DD` - Modification points: Files, contracts, behavior, UX, routes, or plans changed this turn.
- `YYYY-MM-DD` - Problems encountered: Symptom, cause when known, workaround or resolution, and remaining risk. Omit when no problem occurred.
- `YYYY-MM-DD` - Baseline reconciliation: Historical worktree changes mapped to this capability. State provenance limits and never imply unrun verification passed.

## Mobile Lessons Learned

Issues, decisions, and fixes discovered while implementing mobile.

## Desktop Follow-Up Notes

How desktop should adapt the mobile experience without copying mobile-only constraints.

## Route Changes

Document changes to scope, architecture, contracts, or UX.

## Known Pitfalls

Things future agents should avoid. Include failed approaches, platform-specific traps, unresolved blockers, and assumptions that still need verification.

## Verification

- Mobile: Command or focused check and observed result. Write `not run` with the reason when applicable.
- Desktop: Command or focused check and observed result. Write `not run` with the reason when applicable.
- Shared/domain: Command or focused check and observed result. Write `not run` with the reason when applicable.

# Repository Agent Rules

## Cross-Platform Feature Ledger

On every turn in this repository, invoke the `cross-platform-feature-ledger` skill as a mandatory checkpoint before the final response.

- At turn start, inspect `git status --short`, relevant diffs, existing feature ledgers, and `docs/features/_reconciliation.md` for feature changes that predate the current conversation but remain undocumented.
- Backfill unrecorded historical changes into the matching product-capability ledger and map them in `docs/features/_reconciliation.md` before doing normal incremental logging.
- Mark uncertain history as a baseline reconciliation; do not infer authorship, exact timing, intent, or successful verification from a dirty worktree alone.
- When the turn changes, plans, diagnoses, verifies, or discovers a reusable issue in a user-facing feature or shared contract, create or update the matching `docs/features/<feature-slug>.md` record before responding.
- Record concrete modification points in `Implementation Log`.
- Record encountered problems, causes, workarounds, failed attempts, blockers, or unresolved risks in `Known Pitfalls` or the relevant platform lessons section.
- Record only verification that actually ran, including failures and explicit reasons for checks not run.
- Reuse the product capability's existing ledger; do not create one record per chat turn or small bug.
- If the turn produces no durable feature information, run the checkpoint but do not add filler ledger entries.
- Exclude temporary patches, screenshots, APKs/build output, and unrelated tooling from feature ledgers; record deliberate exclusions in the reconciliation index when needed.

The final response must briefly state which feature ledger files were updated, or state that the checkpoint found no durable feature update.

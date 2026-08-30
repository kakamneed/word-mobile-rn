# Phase 15: WASM-safe Mobile Domain Export for Unified PC - Research

**Researched:** 2026-07-28
**Domain:** Rust domain extraction, versioned JSON protocol, WebAssembly packaging, cross-target equivalence
**Confidence:** HIGH for repository boundaries and required behavior; MEDIUM for the not-yet-installed WASM/browser toolchain

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Treat Flutter-backed mobile behavior and its current Rust tests as truth.
- Extract the smallest coherent pure-domain surface instead of forcing storage-backed application crates to compile for WASM.
- Keep native repositories, SQLite transactions, paths, clocks requiring platform access, sync, and lifecycle in adapters.
- Use deterministic inputs for clocks/randomness where fixture equivalence depends on them.
- Publish structured JSON envelopes with explicit protocol and source versions.
- Preserve all existing mobile call paths during extraction; compatibility adapters are acceptable when they keep behavior unchanged.
- Lock native canonical outputs first, then require byte-normalized or semantically identical WASM outputs.
- Cover the NewWord/Review distinction, question units, meanings, question rendering data, resume, wrong-word identity, and report/local-day contracts.
- Run focused Rust and Flutter regression tests for every moved rule boundary.
- Require actual Chromium, Firefox, and WebKit loading/execution checks plus size/startup/serialization measurements before declaring the artifact browser-ready.

### Claude's Discretion

- Exact names of the new pure model/protocol crates and generated package.
- Exact fixture directory layout and semantic JSON normalization implementation.
- Whether the WASM adapter exports one `execute` function or a small set of versioned entry points, provided all paths use the same pure Rust handlers.
- Concrete artifact size and timing budgets, which should be baselined first and then locked before completion.

### Deferred Ideas (OUT OF SCOPE)

- Sharing storage engines across browser and native PC targets.
- Cloud synchronization through the WASM package.
- UI component reuse between Flutter and React.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ARCH-04 | Canonical domain models compile for `wasm32` without `rusqlite`, filesystem, network, Flutter, or Tauri. | The current dependency graph proves `study-core -> storage-core -> rusqlite`; split pure models from persistence and make the WASM crate depend only on pure crates. |
| ARCH-05 | Native mobile and WASM produce equivalent canonical study, plan, progress, wrong-word, and report results. | Use one pure dispatcher and a checked-in fixture corpus executed by both a native runner and the generated WASM package. |
| ARCH-06 | Additive versioned JSON protocol, structured errors, reproducible source identity. | Define explicit envelopes, compatibility rules, error codes, and an artifact manifest containing protocol/schema/source/build identity. |
| ARCH-07 | Preserve current Flutter-to-Rust behavior and leave SQLite/platform adapters outside the package. | Keep `app-core` and `platform-mobile` call paths as compatibility adapters and gate extraction with focused Rust plus Flutter tests. |
</phase_requirements>

## Summary

The product already has a useful pure-logic nucleus in `crates/study-core`, but its crate boundary is not actually WASM-safe: `word-study-core` depends on `word-storage-core`, which unconditionally depends on `rusqlite`. Domain DTOs and SQLite repositories currently live in the same storage crate. The correct first move is therefore a dependency inversion, not an `app-core` WASM build and not a `rusqlite` feature flag. Extract the serde-only model types needed by study, plan/progress, wrong-word, and report calculations into a new pure models crate; make both `storage-core` and the pure domain crates depend on it.

The current rule implementations also contain nondeterministic/platform inputs at their edges. `study-domain-runner` creates session IDs and timestamps with `chrono::Utc::now()`, while wrong-word recency and report gap filling can read `chrono::Local::now()`. These values must enter through versioned requests (`nowUtc`, `localDay`, `sessionSeed` or an explicit `sessionId`) so native and WASM execute the same function over the same data. The fixture corpus must be captured from current mobile-backed Rust behavior before moving code, including the exact NewWord type-major order and Review one-question behavior.

**Primary recommendation:** Create a pure `domain-models -> domain-core -> domain-protocol` chain, expose one shared `execute_v1` dispatcher to native and WASM adapters, and make semantic native/WASM fixture equality plus existing mobile regression tests the completion gate.

## Project Constraints

- The cross-platform feature ledger is mandatory; planning/execution must update `docs/features/learning.md` and `docs/features/_reconciliation.md` when the shared route or verification evidence changes.
- Do not infer successful verification from the current dirty worktree. Record only commands actually run.
- Current Flutter-backed Rust behavior is authoritative when historical desktop, React Native, or mini-program code disagrees.
- Keep offline-first mobile SQLite behavior intact. Phase 15 exports calculations and contracts, not persistence ownership.

## Standard Stack

### Core

| Library/tool | Version | Purpose | Why Standard Here |
|--------------|---------|---------|-------------------|
| Rust workspace | nightly `1.94.0` currently installed | Native and `wasm32-unknown-unknown` builds | Existing implementation language and workspace toolchain. Pin CI/toolchain instead of silently following local nightly. |
| `serde` | `1.0.228` in current lockfile | Typed protocol models | Already used by every relevant crate; preserves current camelCase DTO contracts. |
| `serde_json` | `1.0.149` in current lockfile | Request/result envelopes and fixtures | Already used by the native runner and bridge. |
| `thiserror` | `1.0.69` in current lockfile | Internal typed error taxonomy | Existing pattern; map typed errors to stable public error codes. |
| `chrono` | `0.4.44` in current lockfile | Parse/format supplied timestamps and local days | Keep parsing pure; never call `Utc::now()` or `Local::now()` inside exported calculations. |
| `wasm-bindgen` | `0.2` line, exact lock in Wave 0 | Thin JS/WASM adapter | Officially targets `wasm32-unknown-unknown`; keep it target-specific and outside domain-core. |

### Supporting

| Library/tool | Version | Purpose | When to Use |
|--------------|---------|---------|-------------|
| `wasm-bindgen-test` / `wasm-pack` | exact matching versions resolved in Wave 0 | WASM unit/browser execution | The official guide recommends `wasm-pack` for browser test runner setup. |
| `@playwright/test` | `1.62.0` verified from npm registry on 2026-07-28 | Chromium, Firefox, WebKit artifact smoke and measurement | Use a minimal harness that imports the generated package and executes canonical fixtures in all three projects. |
| Existing Flutter test stack | repository-pinned | Mobile bridge/UI regression | Run only focused contract/display tests per task, then the broader mobile gate at phase completion. |

**Installation (Wave 0, versions must be committed/locked):**

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm.cmd install --save-dev @playwright/test@1.62.0
npx.cmd playwright install chromium firefox webkit
```

Do not install globally during planning. The target, `wasm-pack`, and Playwright browsers are currently absent or unproven on this machine.

## Architecture Patterns

### Recommended Project Structure

```text
crates/
  domain-models/          # serde-only DTOs/enums; no repository modules
  domain-core/            # extracted deterministic study/progress/wrong/report rules
  domain-protocol/        # versioned envelopes, compatibility and execute_v1 dispatcher
  domain-wasm/            # wasm-bindgen adapter; JSON string/bytes in and out
  study-core/             # temporary compatibility facade or reduced module
  storage-core/           # rusqlite repositories, depends on domain-models
  app-core/               # native orchestration, SQLite transactions, resume persistence
  platform-mobile/        # Flutter bridge and platform lifecycle
  study-domain-runner/    # native CLI adapter calling domain-protocol
fixtures/domain/v1/
  requests/
  expected/
  manifest.json
tests/domain-browser/     # minimal generated-package Playwright harness
```

### Pattern 1: Dependency Inversion at the Model Boundary

`storage-core/src/models/*` currently mixes reusable types (`SessionMode`, `StudyQuestion`, requests/results) with a crate that brings `rusqlite`. Move only the needed serde types into `domain-models`. `storage-core` re-exports them temporarily if necessary, so `app-core`, `platform-mobile`, and Flutter JSON shapes do not change in the same task.

Do not solve this with `rusqlite = { optional = true }` while leaving domain ownership in `storage-core`. Feature combinations are easy to regress, obscure the intended architecture, and still make the exported contract conceptually persistence-owned.

### Pattern 2: Pure Dispatcher, Thin Target Adapters

```rust
pub fn execute_v1(request_json: &str) -> String {
    let response: Envelope<ResultPayload> = dispatch(parse(request_json));
    serde_json::to_string(&response).expect("serializable protocol response")
}
```

Both `study-domain-runner` and `domain-wasm` call the same dispatcher. The WASM layer should contain only `#[wasm_bindgen]` exports and conversion glue; it must not select rules or rewrite payloads.

### Pattern 3: Deterministic Execution Context

Every request that can depend on time/order receives an explicit context:

```json
{
  "protocolVersion": 1,
  "command": "buildStudySession",
  "requestId": "fixture-newword-001",
  "source": { "commit": "<git-sha>", "schemaVersion": 1 },
  "context": {
    "nowUtc": "2026-07-28T00:00:00Z",
    "localDay": "2026-07-28",
    "sessionId": "sess_fixture_001",
    "orderingSeed": "fixture-newword-001"
  },
  "payload": {}
}
```

Current question ordering is deterministic from `session_id`; retain that behavior by passing the same ID/seed. Remove `Utc::now()` from `study-domain-runner` handlers and avoid `Local::now()` in exported report/recency functions.

### Pattern 4: Additive Protocol Compatibility

- Top-level response always returns `protocolVersion`, `requestId`, `source`, and exactly one of `result` or `error`.
- Unknown top-level or payload fields are accepted within protocol v1.
- Missing required fields, unknown commands, unsupported major versions, and invalid fixture data return stable error codes, not stderr-only strings or JS exceptions.
- Breaking field/meaning changes require protocol v2; additive optional fields remain v1.
- The generated artifact includes a machine-readable manifest with git commit, dirty flag, Cargo.lock hash, protocol/schema versions, target, optimized WASM SHA-256, raw/gzip sizes, and build command.

### Pattern 5: Golden Master First, Semantic Cross-Target Comparison

Before moving each rule, run the current native implementation with fixed inputs and check in its expected JSON. After extraction, compare parsed JSON values, not raw object key order. Preserve array order because it is product behavior. Separately test deterministic serialization if consumers cache/hash bytes.

Required fixture groups:

- NewWord: two or more words, four type-major rounds, `4 * wordCount` questions, stable meaning set, phonetics/examples and feedback-only translations.
- Review/mixed/wrong/root: one question per selected entry according to current mode weights; never inherit the NewWord loop.
- Choice integrity: unique non-empty per-entry distractors, non-A correct labels, no stray label text in option content, canonical correct meaning.
- Progress/summary: question units, carry-over totals, duplicate-submit cap, distinct word totals.
- Resume: answered history/current question/index/total and question-engine compatibility survive snapshot round-trip.
- Wrong/report: real `entrySourceId`, no `unknown`, explicit local day, zero-answer filtering and mode normalization.

### Anti-Patterns to Avoid

- **Compile `app-core` wholesale:** it directly depends on `rusqlite` and owns lifecycle/transactions.
- **Duplicate TypeScript rules:** Word Net must call the artifact; TS may validate envelopes only.
- **Generate time/randomness inside domain code:** it prevents native/WASM fixture equivalence.
- **Use `serde_json::Value` as the public contract:** existing report/wrong-word services should gain typed inputs/outputs before export.
- **Treat a successful `cargo build --target wasm32` as completion:** it proves neither parity nor browser loading.
- **Move persistence semantics into WASM:** resume projection inputs/outputs may be shared, but SQLite/IndexedDB storage remains adapter-owned.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rust/JS ABI | Manual pointer/memory protocol | `wasm-bindgen` | Handles JS glue and WASM memory conversions. |
| Cross-browser automation | Browser-specific scripts/drivers | Playwright projects | One test suite drives Chromium, Firefox, and WebKit. |
| JSON parsing/shape conversion | String slicing or ad hoc maps | Typed serde envelopes | Stable errors and compatibility are testable. |
| Hashing source/artifacts | Custom checksum format | Git SHA plus SHA-256 produced by standard tools | Consumers can reproduce and pin identity. |
| Time zone inference | Browser/native local clock reads in core | Explicit ISO UTC timestamp and `YYYY-MM-DD` local day inputs | Avoids native/WASM and timezone drift. |

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | SQLite `app_settings` holds `active_study_session_<mode>` JSON snapshots; `study_sessions`, `study_results`, entries/meanings/examples, question preps, accepted meanings, and mastered state feed current behavior. | No database migration in Phase 15. Keep native repositories. Add fixture/snapshot compatibility tests and preserve question-engine version behavior when DTOs move. |
| Live service config | No external service config is required by the pure learning export. Word Net's artifact pin will live in its own repo later. | None in mobile runtime; emit manifest for downstream pinning. |
| OS-registered state | Android/iOS bridge registrations load the native `word_platform_mobile` library; no WASM registration exists. | Do not replace or rename current bridge symbols. WASM is an additional artifact. |
| Secrets/env vars | Supabase/AI environment values exist for mobile features but are outside the exported domain. | Domain/WASM crates must not reference them; no secret migration. |
| Build artifacts | Native `.so`/static libraries and release APKs coexist with future `.wasm`/JS/package output. | Generate WASM into a dedicated reproducible directory, exclude transient output, publish only manifest-pinned artifacts. Never use an old artifact as parity proof. |

## Common Pitfalls

### Storage Models Masquerading as Pure Domain

**What goes wrong:** `study-core` appears pure by source behavior but `cargo tree -p word-study-core` includes `rusqlite` and `libsqlite3-sys` through `storage-core`.

**Prevention:** Make `cargo tree -p <wasm crate>` and a forbidden-dependency scan explicit tests. Fail on `rusqlite`, `reqwest`, `jni`, `objc`, Tauri, filesystem, or platform-mobile dependencies.

### Native Runner Is Not Yet Deterministic

**What goes wrong:** `study-domain-runner` creates session IDs and answer/completion timestamps from `Utc::now()`.

**Prevention:** Require explicit context values in fixture requests; compatibility CLI defaults may supply current time only outside fixture mode.

### Resume Is Orchestration Plus Pure State

**What goes wrong:** Moving `start_study_session` wholesale drags SQLite snapshots and process-global active session handling into WASM, or omitting resume loses the user's current position.

**Prevention:** Export typed pure `SessionState` transitions/snapshot validation; leave loading/saving and transaction boundaries in native/PC storage adapters.

### Reports and Wrong Words Hide Platform Reads

**What goes wrong:** `reports_service::build_recent_days` and wrong-word recency can fall back to `chrono::Local::now()`; public values are loosely typed JSON.

**Prevention:** Add typed pure functions that require `localDay`; keep compatibility wrappers native-only.

### Canonical Fixtures Accidentally Bless Stale Behavior

**What goes wrong:** Fixtures are generated from a stale artifact, legacy React Native stub, or old desktop implementation.

**Prevention:** Record the mobile git commit and dirty state, trace each fixture through current Rust plus Flutter bridge contract, and review required behaviors against `docs/features/learning.md` before locking expected output.

### WebKit Evidence Is Mistaken for Safari Certification

**What goes wrong:** Playwright WebKit on Windows/Linux is reported as real Safari proof.

**Prevention:** Call it Playwright WebKit evidence. Safari-specific validation remains a separate macOS gate if later required.

## Code Examples

### Target-Specific WASM Adapter Dependency

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
```

Source: official wasm-bindgen target guide. The domain crates themselves should not need this dependency.

### Three-Browser Artifact Gate

```ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'firefox', use: { browserName: 'firefox' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
});
```

Source: Playwright official browser documentation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust/Cargo | native extraction/tests | Yes | nightly `1.94.0` | Pin a project toolchain before implementation. |
| `wasm32-unknown-unknown` target | WASM compile | No | - | Wave 0 `rustup target add`; blocking until installed. |
| `wasm-pack` | package/browser Rust tests | No | - | Direct `cargo build` can diagnose boundaries but cannot satisfy browser gate. |
| Node/npm | Playwright harness | Yes | Node `24.14.0`, npm `11.9.0` | None needed. |
| Playwright package/browsers | Chromium/Firefox/WebKit proof | Not proven | registry latest verified `1.62.0` | Install repo-locally in Wave 0; network/cache permission may require an approved install. |
| Flutter toolchain | mobile regression | Existing project tooling | repository/environment managed | Use the repository's explicit Flutter executable if PATH is ambiguous. |

**Missing dependencies with no completion fallback:** `wasm32-unknown-unknown`, `wasm-pack`, and Playwright browser binaries. Plans must install/probe these in Wave 0 and must not mark browser readiness before all three projects run.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Native unit/integration | Rust built-in test harness across domain, runner, app-core, storage-core |
| WASM unit | `wasm-bindgen-test` / `wasm-pack test` |
| Cross-browser | Playwright Test `1.62.0`, projects: Chromium, Firefox, WebKit |
| Mobile compatibility | Existing Flutter tests plus Rust `app-core`/`platform-mobile` focused tests |
| Quick native command | `cargo test -p word-study-core -p word-study-domain-runner` |
| Full Rust command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| ARCH-04 | Pure export has no platform/storage dependencies and compiles to WASM | compile + dependency contract | `cargo check -p word-domain-wasm --target wasm32-unknown-unknown` plus forbidden-dependency script | No - Wave 0/1 |
| ARCH-05 | Native and WASM return semantically identical canonical fixtures | golden integration | native fixture runner, WASM fixture runner, JSON semantic diff | No - Wave 0/2 |
| ARCH-05 | NewWord type-major four rounds; Review one per entry; canonical meanings/render data | domain unit/golden | `cargo test -p word-domain-core fixture_` | Partial: current `question_builder.rs`; canonical corpus missing |
| ARCH-05 | Resume state, wrong identity, report local-day semantics | state/projection golden | `cargo test -p word-domain-core resume_ wrong_ report_` | Partial in `study_facade.rs` and services; pure tests missing |
| ARCH-06 | Version negotiation, additive fields, structured errors, source manifest | protocol contract | `cargo test -p word-domain-protocol` and manifest validation script | No - Wave 0/2 |
| ARCH-06 | Generated package loads and executes fixtures in three browsers | browser integration | `npm.cmd run test:domain-wasm -- --project=chromium --project=firefox --project=webkit` | No - Wave 0/3 |
| ARCH-07 | Flutter bridge shapes and mobile behavior unchanged | regression | focused Rust bridge/app tests plus Flutter study/report/wrong tests | Partial: existing tests listed below |

### Existing High-Value Regression Anchors

- `crates/study-core/src/question_builder.rs`: NewWord round order, weighted non-loop modes, choice/distractor/meaning rendering.
- `crates/study-core/src/answer_evaluator.rs`: correct choice text/label authority and accepted meaning behavior.
- `crates/app-core/src/facade/study_facade.rs`: answer/submit, resume history, completion cleanup, non-A labels, mastered-entry pruning.
- `crates/app-core/src/services/reports_service.rs` and `wrong_words_service.rs`: report aggregation and priority/recency.
- `crates/app-core/tests/baseline_runner.rs`: broader current native behavior.
- `apps/flutter_mobile/test/study_client_test.dart`, `study_question_display_test.dart`, `today_task_breakdown_test.dart`, `reports_screen_test.dart`, `wrong_words_screen_test.dart`.

### Sampling Rate

- **Per extraction task:** focused unit tests for the moved module plus native fixture subset.
- **Per wave:** `cargo test -p word-domain-core -p word-domain-protocol -p word-study-core -p word-study-domain-runner` and semantic native/WASM fixture diff.
- **Mobile adapter wave:** focused `app-core`/`platform-mobile` Rust tests and the named Flutter tests.
- **Phase gate:** `cargo test --workspace`, all canonical fixture comparisons, optimized artifact/manifest validation, and Playwright green in Chromium, Firefox, and WebKit.

### Wave 0 Gaps

- [ ] Install/pin `wasm32-unknown-unknown`, `wasm-pack`, matching wasm-bindgen packages, and repo-local Playwright/browser binaries.
- [ ] Create pure-crate dependency contract test/forbidden-dependency scan.
- [ ] Capture native canonical fixture requests/expected outputs before extraction.
- [ ] Add semantic JSON comparator that preserves array-order checks.
- [ ] Add protocol compatibility/error/manifest tests.
- [ ] Add minimal browser harness and size/startup/serialization measurement script.
- [ ] Record baseline timings and sizes, then lock explicit non-regression budgets before final wave.

## Planning Decomposition Recommendation

1. **Plan 15-01 - Freeze truth and create Wave 0 gates:** toolchain probes, fixture manifest, current native golden outputs, forbidden dependency test, measurement harness.
2. **Plan 15-02 - Extract pure models and study domain:** invert `storage-core` dependency, preserve re-exports/adapters, remove clock/session generation from pure paths, prove NewWord/Review and answer fixtures.
3. **Plan 15-03 - Extract state/projection contracts and protocol:** typed resume transition, wrong-word/report calculations with explicit local day, v1 envelopes/errors/source manifest, native runner conversion.
4. **Plan 15-04 - Package WASM and prove equivalence:** wasm-bindgen adapter, reproducible optimized artifact, semantic native/WASM corpus comparison, Chromium/Firefox/WebKit gates and measurements.
5. **Plan 15-05 - Mobile regression and downstream handoff:** full Rust/focused Flutter regressions, ledger updates, publish artifact identity and Word Net consumption instructions without implementing Word Net.

Do not move mobile adapters before the golden baseline is committed. Each extraction plan should leave compatibility re-exports so callers migrate separately and failures identify the moved boundary.

## Open Questions

1. **Which repository owns the Playwright harness?**
   - Recommendation: keep a minimal artifact-only harness in mobile Phase 15 so export readiness is independently provable; Word Net will add its own integration tests later.
2. **What size/startup limits apply?**
   - Recommendation: Wave 0/first optimized package records raw/gzip WASM size, JS glue size, first import time, first command time, and repeat command time. Lock budgets before the final implementation wave rather than inventing numbers now.
3. **How much plan/Today logic is pure enough today?**
   - Current evidence is strongest for study rules; planner should inventory `today_facade`/plan calculations and export only deterministic calculations required by the specified fixtures. Do not drag repository selection or daily snapshot writes into the package.

## Sources

### Primary (HIGH confidence)

- Repository `Cargo.toml` files and `cargo tree -p word-study-core` on 2026-07-28 - current dependency boundary and versions.
- `crates/study-core/src/question_builder.rs`, `answer_evaluator.rs`, `session_definition.rs` - canonical current study rules.
- `crates/app-core/src/facade/study_facade.rs` and `crates/storage-core/src/persistence/study_repo.rs` - resume/persistence boundary.
- `crates/app-core/src/services/reports_service.rs` and `wrong_words_service.rs` - current time-sensitive projections.
- https://wasm-bindgen.github.io/wasm-bindgen/reference/rust-targets.html - official `wasm32-unknown-unknown` and target-specific dependency guidance.
- https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/browsers.html - official browser test/wasm-pack guidance.
- https://playwright.dev/docs/browsers - official Chromium/Firefox/WebKit project and browser-install guidance.
- https://registry.npmjs.org/@playwright/test/latest - `@playwright/test` `1.62.0` verification on 2026-07-28.

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH for existing Rust dependencies; MEDIUM for new WASM tooling until installed and locked.
- Architecture: HIGH - based on current crate graph and explicit phase constraints.
- Required fixtures: HIGH - grounded in current mobile code, tests, PRD, context, and learning ledger.
- Browser execution: MEDIUM - official tooling supports the matrix, but this machine has not installed or run it.

**Research date:** 2026-07-28
**Valid until:** 2026-08-27 for architecture; re-verify tool versions at execution.

# Phase 2: Cross-Platform Contract and Desktop Refactor

## Goal

Lock down the shared contract boundary that both desktop and mobile will depend on. Refactor desktop to consume the Phase 1 shared core through adapter layers. Prove desktop behavior remains correct after extraction.

## Success Criteria

1. DTO shapes are explicit, versioned, and tested
2. Desktop uses the same contracts that mobile will use
3. Core extraction does not regress desktop study-loop correctness (bootstrap, today, study, wrong-words, reports)

## Phase Boundary

- **In scope:** Contract definitions, facade layer, TypeScript generation, desktop refactor for learning loop
- **Out of scope:** React Native bridge, mobile UI, full desktop parity for non-core surfaces

## Dependencies

- Phase 1 shared core crates (app-core, study-core, content-core, storage-core)
- Desktop reference at `../word-desktop-tauri`

## Verification Approach

- Contract tests for serialization round-trips
- Desktop integration tests for learning loop
- Regression test suite comparing pre/post refactor behavior

---

## Wave 1: Contract Stabilization

### Plan 02-01: Document Cross-Platform Contract

**Goal:** Create CONTRACT.md with explicit DTO shapes and serialization rules.

**Files to create:**
- `docs/CONTRACT.md` - Contract documentation
- `crates/contracts/src/lib.rs` - Contract export module

**Key content for CONTRACT.md:**

```markdown
# Word Mobile Cross-Platform Contract

## Version: 0.1.0

### Bootstrap Contract

#### Request
None (GET style)

#### Response: BootstrapState
- app_ready: boolean
- first_run_required: boolean
- database_status: "ready" | "init_failed"
- snapshot_status: "ready" | "missing_required" | "empty"
- connectivity_status: "offline" | "online"
- ai_config_status: "configured" | "missing"
- settings_entry_available: boolean
- blocking_reason: string | null

### Today Home Contract

#### Request
None (GET style)

#### Response: TodayHomeState
- today_date: string (ISO 8601)
- active_plan: PlanSummary | null
- today_snapshot: DailySnapshot | null
- wordbooks: WordbookSummary[]

### Study Session Contract

#### Start Session Request
- mode: "new_word" | "review" | "mixed_test" | "wrong_word_reinforcement"
- wordbook_id: number | null
- entry_source_ids: string[]

#### Start Session Response
- session: StudySession
- current_question: StudyQuestion
- progress: { current: number, total: number }

#### Submit Answer Request
- question_id: string
- response: string
- response_time_ms: number

#### Submit Answer Response
- result: StudyResult
- is_complete: boolean
- next_question?: StudyQuestion
- summary?: SessionSummary
- next_action?: string
```

**Acceptance criteria:**
- CONTRACT.md exists with all DTO shapes documented
- All field types are explicit
- Serialization format (camelCase) documented

---

### Plan 02-02: Create Facade Layer in app-core

**Goal:** Create high-level facade API for platform consumers.

**Files to create:**
- `crates/app-core/src/facade.rs` - Facade module
- `crates/app-core/src/facade/bootstrap_facade.rs` - Bootstrap operations
- `crates/app-core/src/facade/today_facade.rs` - Today home operations
- `crates/app-core/src/facade/study_facade.rs` - Study session operations

**Key content for bootstrap_facade.rs:**
```rust
/// Bootstrap the application.
pub fn bootstrap(
    runtime: &dyn PlatformRuntime,
    db_path: &Path,
) -> Result<BootstrapState, BootstrapError> {
    let conn = persistence::initialize_database(db_path)?;
    evaluate_bootstrap(runtime, &conn)
}
```

**Key content for today_facade.rs:**
```rust
/// Get today's home state.
pub fn get_today_home_state(conn: &Connection) -> Result<TodayHomeState, StorageError> {
    // Implementation
}
```

**Key content for study_facade.rs:**
```rust
/// Start a study session.
pub fn start_study_session(
    conn: &Connection,
    request: StartSessionRequest,
) -> Result<StartSessionResponse, StudyError> {
    // Implementation
}

/// Submit a study answer.
pub fn submit_study_answer(
    session_id: &str,
    request: SubmitAnswerRequest,
) -> Result<SubmitAnswerResponse, StudyError> {
    // Implementation
}

/// Complete a study session.
pub fn complete_study_session(
    conn: &Connection,
    session_id: &str,
) -> Result<CompleteSessionResponse, StudyError> {
    // Implementation
}
```

**Acceptance criteria:**
- All facades compile
- No direct Tauri dependencies
- API is ergonomic for both desktop and mobile

---

### Plan 02-03: Add Today Home DTOs to storage-core

**Goal:** Create TodayHomeState and related DTOs that don't exist yet.

**Files to create:**
- `crates/storage-core/src/models/today_home_state.rs` - Today home state
- `crates/storage-core/src/models/daily_snapshot.rs` - Daily snapshot
- `crates/storage-core/src/models/plan_summary.rs` - Plan summary

**Acceptance criteria:**
- All new DTOs implement Serialize/Deserialize
- All DTOs use camelCase naming
- Unit tests for serialization round-trips

---

## Wave 2: TypeScript Contract Generation

### Plan 02-04: Create TypeScript Type Generator

**Goal:** Generate TypeScript types from Rust DTOs.

**Files to create:**
- `scripts/generate-ts-types.rs` - Type generator script
- `packages/contracts/tsconfig.json` - TypeScript package config

**Generated files:**
- `packages/contracts/src/bootstrap.ts` - Bootstrap types
- `packages/contracts/src/today.ts` - Today home types
- `packages/contracts/src/study.ts` - Study session types
- `packages/contracts/src/common.ts` - Shared types

**Acceptance criteria:**
- Generator script runs successfully
- Generated types match Rust DTOs exactly
- TypeScript package compiles without errors

---

### Plan 02-05: Create Contract Test Suite

**Goal:** Verify contract serialization round-trips correctly.

**Files to create:**
- `crates/storage-core/tests/contract_tests.rs` - Serialization tests

**Test coverage:**
- Each DTO serializes to expected JSON shape
- Each DTO deserializes from JSON correctly
- Unknown fields are handled appropriately
- camelCase serialization is correct

**Acceptance criteria:**
- All tests pass
- 100% DTO coverage

---

## Wave 3: Desktop Refactor (Deferred to Desktop Repo)

### Plan 02-06: Create Tauri Platform Adapter (in desktop repo)

**Goal:** Implement PlatformRuntime for Tauri.

**File (in desktop repo):**
- `apps/desktop/src-tauri/src/platform_adapter.rs`

**Acceptance criteria:**
- Adapter implements all PlatformRuntime methods
- Error handling maps Tauri errors to PlatformError

---

### Plan 02-07: Refactor Desktop Commands (in desktop repo)

**Goal:** Thin out command handlers to use shared core facades.

**Files to modify (in desktop repo):**
- `apps/desktop/src-tauri/src/commands/bootstrap.rs`
- `apps/desktop/src-tauri/src/commands/today.rs`
- `apps/desktop/src-tauri/src/commands/study.rs`
- `apps/desktop/src-tauri/src/main.rs`

**Acceptance criteria:**
- Commands use shared core facades
- Desktop compiles with shared core
- Bootstrap flow works end-to-end

---

### Plan 02-08: Add Desktop Regression Tests (in desktop repo)

**Goal:** Ensure desktop behavior hasn't regressed.

**Files to create (in desktop repo):**
- `apps/desktop/src-tauri/tests/integration_tests.rs`

**Test coverage:**
- Bootstrap returns correct state
- Today home loads plan and snapshot
- Study session starts and completes
- Wrong words are tracked
- Reports aggregate correctly

**Acceptance criteria:**
- All integration tests pass
- No regressions in learning loop

---

## Summary

| Wave | Plans | Goal |
|------|-------|------|
| 1 | 02-01 to 02-03 | Contract stabilization: docs, facade, DTOs |
| 2 | 02-04 to 02-05 | TypeScript generation and testing |
| 3 | 02-06 to 02-08 | Desktop refactor (in desktop repo) |

## Notes

- Plans 02-06 through 02-08 must be executed in the desktop repository
- Phase 2 can be considered "structurally complete" in this repo after Wave 2
- Full Phase 2 completion requires desktop repo work

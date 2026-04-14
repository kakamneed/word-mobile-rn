# Phase 1: Shared Rust Core Extraction Baseline

## Goal

Extract the reusable Rust business core from the desktop Tauri app into platform-neutral crates that can be shared between desktop and mobile shells. Desktop remains compilable and testable throughout the extraction.

## Success Criteria

1. Tauri command handlers become thin adapters (demonstrated by `bootstrap.rs` already being thin, `study.rs` being refactored)
2. Shared crates compile independently of Tauri concerns
3. Core domain logic has tests covering study sessions, plans, snapshots, and vocabulary import
4. Platform-specific concerns (paths, storage bootstrap) are abstracted behind a runtime boundary
5. Desktop app continues to compile and pass key behavior chains

## Phase Boundary

- **In scope:** Crate structure, platform abstraction, service extraction from commands
- **Out of scope:** React Native bridge, mobile app bootstrap, fine-grained crate decomposition

## Dependencies

- Desktop reference code at `../word-desktop-tauri` (behavior reference)
- Phase 1 context decisions already captured in `01-CONTEXT.md`

## Verification Approach

- Unit tests for extracted services
- Integration test: desktop compiles and bootstrap flow works
- Cross-platform compile check for extracted crates (no Tauri dependency)

---

## Wave 1: Foundation - Crate Skeleton and Platform Abstraction

### Plan 01-01: Create Shared Core Repository Structure

**Goal:** Initialize the shared Rust crate structure with four medium-granularity crates.

**Files to create:**
- `crates/app-core/Cargo.toml` - App lifecycle, bootstrap orchestration
- `crates/app-core/src/lib.rs` - Main exports
- `crates/study-core/Cargo.toml` - Study session domain logic
- `crates/study-core/src/lib.rs` - Main exports
- `crates/content-core/Cargo.toml` - Vocabulary, wordbooks, content import
- `crates/content-core/src/lib.rs` - Main exports
- `crates/storage-core/Cargo.toml` - Persistence, database, models
- `crates/storage-core/src/lib.rs` - Main exports
- `Cargo.toml` - Workspace root with crate members

**Key content:**
- Workspace configuration with all four crates
- Minimal dependencies (serde, chrono, rusqlite, thiserror)
- No Tauri dependencies in any crate
- Version 0.1.0 for all crates

**Acceptance criteria:**
- `cargo check` passes at workspace root
- No compiler errors in any crate
- Crates are independent (no circular deps)

---

### Plan 01-02: Define Platform Runtime Abstraction

**Goal:** Create coarse `PlatformRuntime` trait that abstracts Tauri-specific dependencies.

**Files to create:**
- `crates/app-core/src/platform.rs` - PlatformRuntime trait definition

**Key content:**
```rust
/// Coarse platform abstraction for runtime services.
/// Mobile and desktop implement this trait for their respective platforms.
pub trait PlatformRuntime: Send + Sync {
    /// Returns the path to the application data directory.
    fn app_data_dir(&self) -> Result<std::path::PathBuf, PlatformError>;

    /// Returns the path to the application config directory.
    fn app_config_dir(&self) -> Result<std::path::PathBuf, PlatformError>;

    /// Returns the path to the application log directory.
    fn app_log_dir(&self) -> Result<std::path::PathBuf, PlatformError>;

    /// Returns the path to a bundled resource file.
    fn bundled_resource_path(&self, relative_path: &str) -> Result<std::path::PathBuf, PlatformError>;

    /// Returns the current time (abstracted for testability).
    fn now(&self) -> chrono::DateTime<chrono::Utc>;

    /// Opens an external URL or path using platform mechanisms.
    fn open_external(&self, url: &str) -> Result<(), PlatformError>;
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("Failed to resolve path: {0}")]
    PathResolution(String),
    #[error("IO error: {0}")]
    Io(String),
}
```

**Acceptance criteria:**
- Trait compiles without errors
- Documentation explains each method's purpose
- Error type is `thiserror`-based for ergonomics

---

### Plan 01-03: Extract Data Models to storage-core

**Goal:** Move all domain models from desktop into `storage-core` as the shared source of truth.

**Files to create:**
- `crates/storage-core/src/models/mod.rs` - Model module exports
- `crates/storage-core/src/models/bootstrap_state.rs` - BootstrapState struct
- `crates/storage-core/src/models/plan_template.rs` - PlanTemplate struct
- `crates/storage-core/src/models/study_session.rs` - StudySession, SessionMode
- `crates/storage-core/src/models/study_question.rs` - StudyQuestion, QuestionType
- `crates/storage-core/src/models/study_answer.rs` - StudyAnswer
- `crates/storage-core/src/models/study_result.rs` - StudyResult
- `crates/storage-core/src/models/standardized_entry.rs` - StandardizedEntry, MeaningZh, EntryExample
- `crates/storage-core/src/models/wordbook.rs` - Wordbook
- `crates/storage-core/src/models/wordbook_entry.rs` - WordbookEntry
- `crates/storage-core/src/models/wrong_word_state.rs` - WrongWordState
- `crates/storage-core/src/models/settings.rs` - Settings models

**Key adaptations:**
- Remove any Tauri-specific derives or annotations
- Ensure all structs implement `Serialize, Deserialize` from serde
- Use `#[derive(Clone, Debug)]` for all models
- Keep field names identical to desktop for compatibility

**Acceptance criteria:**
- All models compile with `cargo check -p storage-core`
- Unit test: each model can be serialized/deserialized to JSON
- No Tauri dependencies in any model

---

## Wave 2: Service Extraction

### Plan 01-04: Extract Storage Service Layer

**Goal:** Move persistence logic from desktop `persistence/` module into `storage-core`.

**Files to create:**
- `crates/storage-core/src/persistence/mod.rs` - Public exports
- `crates/storage-core/src/persistence/connection.rs` - Database connection management
- `crates/storage-core/src/persistence/schema.rs` - Database schema (migrated from desktop)
- `crates/storage-core/src/persistence/entry_repo.rs` - Entry CRUD operations
- `crates/storage-core/src/persistence/wordbook_repo.rs` - Wordbook CRUD operations
- `crates/storage-core/src/persistence/plan_repo.rs` - Plan template CRUD operations
- `crates/storage-core/src/persistence/study_repo.rs` - Study session/results persistence

**Key adaptations:**
- Remove `tauri::AppHandle` dependencies
- Accept `Path` or `Connection` as parameters instead
- Return `Result<T, StorageError>` with `thiserror` error types
- Keep SQL queries identical to desktop for compatibility

**Acceptance criteria:**
- `cargo test -p storage-core` passes
- Unit tests for each repository function
- Mock connection tests for query logic

---

### Plan 01-05: Extract Bootstrap Service to app-core

**Goal:** Migrate `bootstrap_service.rs` logic into `app-core` with platform abstraction.

**Files to create:**
- `crates/app-core/src/bootstrap.rs` - Bootstrap evaluation logic
- `crates/app-core/src/bootstrap/evaluator.rs` - `evaluate_bootstrap` function

**Key adaptations:**
- Change signature from `evaluate_bootstrap(app: &tauri::AppHandle)` to:
  `evaluate_bootstrap(runtime: &dyn PlatformRuntime, conn: &Connection) -> BootstrapState`
- Remove direct Tauri path calls, use `runtime.app_data_dir()` etc.
- Database connection passed in (caller manages connection lifecycle)

**Acceptance criteria:**
- Function compiles and has no Tauri dependencies
- Unit test with mock PlatformRuntime implementation
- BootstrapState correctly evaluates all conditions

---

### Plan 01-06: Extract Study Services to study-core

**Goal:** Migrate study domain services from desktop into `study-core`.

**Files to create:**
- `crates/study-core/src/lib.rs` - Public exports
- `crates/study-core/src/question_builder.rs` - QuestionBuilder (from desktop)
- `crates/study-core/src/answer_evaluator.rs` - AnswerEvaluator (from desktop)
- `crates/study-core/src/session_definition.rs` - SessionDefinition service
- `crates/study-core/src/session_summary.rs` - SessionSummaryService (from desktop)
- `crates/study-core/src/state_transition.rs` - Study state transitions

**Key adaptations:**
- Pure domain logic, no persistence calls
- Accept data structures as parameters, return results
- No Tauri dependencies
- Testable with unit tests

**Acceptance criteria:**
- All services compile with `cargo check -p study-core`
- Unit tests for question generation, answer evaluation, session summary
- No persistence or platform dependencies in study-core

---

### Plan 01-07: Extract Content Services to content-core

**Goal:** Migrate vocabulary and content management services.

**Files to create:**
- `crates/content-core/src/lib.rs` - Public exports
- `crates/content-core/src/snapshot.rs` - Snapshot checking (from snapshot_service.rs)
- `crates/content-core/src/vocabulary_import.rs` - Vocabulary import logic
- `crates/content-core/src/wordbook_derive.rs` - Wordbook derivation logic
- `crates/content-core/src/vocabulary_status.rs` - Vocabulary status evaluation

**Key adaptations:**
- Accept `&Path` for file operations instead of Tauri app_handle
- Return results that caller can persist
- No direct database calls (higher level orchestrates)

**Acceptance criteria:**
- All services compile with `cargo check -p content-core`
- Snapshot checking works with path parameter
- Unit tests for vocabulary import logic

---

## Wave 3: Desktop Adapter and Integration

### Plan 01-08: Create Tauri Platform Runtime Implementation

**Goal:** Implement `PlatformRuntime` trait for Tauri in the desktop app.

**Files to create (in desktop repo, not shared core):**
- `apps/desktop/src-tauri/src/platform_impl.rs` - TauriPlatformRuntime struct

**Key content:**
```rust
use word_core::platform::PlatformRuntime;

pub struct TauriPlatformRuntime {
    app_handle: tauri::AppHandle,
}

impl TauriPlatformRuntime {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl PlatformRuntime for TauriPlatformRuntime {
    // Delegate to Tauri APIs
}
```

**Acceptance criteria:**
- Implementation compiles in desktop app
- All trait methods implemented
- Error handling maps Tauri errors to PlatformError

---

### Plan 01-09: Refactor Desktop Commands to Use Shared Core

**Goal:** Thin out command handlers to use shared core services.

**Files to modify (in desktop repo):**
- `apps/desktop/src-tauri/src/commands/bootstrap.rs` - Use `app_core::bootstrap`
- `apps/desktop/src-tauri/src/commands/study.rs` - Extract domain logic to study-core calls
- `apps/desktop/src-tauri/src/commands/plan.rs` - Use storage-core repos
- `apps/desktop/src-tauri/src/commands/vocabulary.rs` - Use content-core services
- `apps/desktop/src-tauri/src/main.rs` - Add shared core dependencies

**Cargo.toml additions (desktop):**
```toml
[dependencies]
word-app-core = { path = "../../../word-mobile-rn/crates/app-core" }
word-study-core = { path = "../../../word-mobile-rn/crates/study-core" }
word-content-core = { path = "../../../word-mobile-rn/crates/content-core" }
word-storage-core = { path = "../../../word-mobile-rn/crates/storage-core" }
```

**Acceptance criteria:**
- Desktop app compiles with shared core dependencies
- Bootstrap command uses shared core
- At least one study command refactored as proof of concept

---

### Plan 01-10: Add Cross-Platform Compilation Verification

**Goal:** Ensure shared crates compile without Tauri on multiple targets.

**Files to create:**
- `.github/workflows/ci.yml` - CI workflow (or local verification script)
- `scripts/verify-cross-platform.sh` - Cross-compile check script

**Verification steps:**
```bash
# Check no Tauri in shared crates
cargo check -p word-app-core
cargo check -p word-study-core
cargo check -p word-content-core
cargo check -p word-storage-core

# Run tests
cargo test -p word-storage-core
cargo test -p word-study-core
cargo test -p word-content-core
cargo test -p word-app-core
```

**Acceptance criteria:**
- All crates compile independently
- All unit tests pass
- CI script documents the verification steps

---

## Summary

| Wave | Plans | Goal |
|------|-------|------|
| 1 | 01-01 to 01-03 | Foundation: crate structure, platform abstraction, models |
| 2 | 01-04 to 01-07 | Service extraction: storage, bootstrap, study, content |
| 3 | 01-08 to 01-10 | Desktop integration: adapter, command refactor, verification |

## Verification Checklist

- [ ] All 4 crates compile independently
- [ ] PlatformRuntime trait is implemented for Tauri
- [ ] Desktop app compiles with shared core
- [ ] Bootstrap flow works end-to-end
- [ ] Unit tests cover extracted services
- [ ] No Tauri dependencies leak into shared crates

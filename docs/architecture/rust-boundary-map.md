# Rust Module Boundary Map

Status: Frozen (Slice 2)
Owner: Architecture

## Current crate structure

```
word-mobile-rn/
  crates/
    app-core/         -- Facades, bootstrap, platform abstraction, domain services
    study-core/       -- Pure study domain: question building, answer evaluation, session summary
    content-core/     -- Vocabulary import, snapshot validation (mostly stubs)
    storage-core/     -- Models, repositories, schema, persistence
    platform-mobile/  -- FFI/JNI/ObjC exports, bridge functions, AI HTTP calls, path resolution
```

## Responsibility matrix

| Responsibility | Current crate | Ideal crate | Status |
|---|---|---|---|
| Question generation | study-core | study-core | CORRECT |
| Answer evaluation + normalization | study-core | study-core | CORRECT |
| Session summary computation | study-core (delegates to storage-core model) | study-core | MINOR: logic lives in storage-core `SessionSummary::from_results()` |
| Session state machine | app-core/study_facade | study-core or app-core | MIXED: facade holds global ACTIVE_SESSIONS + lifecycle rules |
| Today snapshot derivation | app-core/today_facade | app-core | CORRECT |
| Daily progress recommendation | app-core/today_facade | app-core | CORRECT |
| Wrong-word priority scoring | app-core/services (moved) | app-core | MOVED from bridge.rs |
| Wrong-word filtering/sorting | app-core/services (moved) | app-core | MOVED from bridge.rs |
| Reports aggregation | app-core/services (moved) | app-core | MOVED from bridge.rs |
| Streak calculation | app-core/services (moved) | app-core | MOVED from bridge.rs |
| Plan management + growth rules | platform-mobile/bridge.rs | app-core | STILL IN BRIDGE |
| Wordbook catalog + activation | platform-mobile/bridge.rs | app-core + content-core | STILL IN BRIDGE |
| AI passage generation | platform-mobile/bridge.rs | app-core + platform-mobile | STILL IN BRIDGE |
| AI provider HTTP calls | platform-mobile/bridge.rs | platform-mobile | ACCEPTABLE (platform concern) |
| Schema + migrations | storage-core | storage-core | CORRECT |
| Repository pattern | storage-core | storage-core | CORRECT |
| DTO models | storage-core | storage-core | ACCEPTABLE (centralized models) |
| Bootstrap orchestration | app-core | app-core | CORRECT |
| Platform runtime trait | app-core | app-core | CORRECT |
| Mobile path resolution | platform-mobile | platform-mobile | CORRECT |
| FFI/JNI/ObjC exports | platform-mobile | platform-mobile | CORRECT |
| Direct SQL in bridge | platform-mobile/bridge.rs | storage-core repositories | STILL IN BRIDGE |

## What was moved in Slice 2

### From platform-mobile/bridge.rs to app-core/services/

| Function | Lines moved | New module |
|---|---|---|
| `build_wrong_words()` | ~40 | `services::wrong_words_service` |
| `recency_score()` | ~10 | `services::wrong_words_service` |
| `compare_wrong_word_entries()` | ~35 | `services::wrong_words_service` |
| `build_reports_overview()` | ~165 | `services::reports_service` |
| `build_recent_days()` | ~35 | `services::reports_service` |
| `build_all_days_series()` | ~28 | `services::reports_service` |
| `build_mode_breakdown()` | ~24 | `services::reports_service` |
| `build_mode_series()` | ~22 | `services::reports_service` |
| `build_streak_info()` | ~40 | `services::reports_service` |

**Total: ~400 lines of domain logic extracted.**

### What remains in bridge.rs for future slices

| Concern | Approximate lines | Target slice |
|---|---|---|
| Plan management + growth rules | ~250 | Slice 4+ |
| AI passage generation + parsing | ~350 | Slice 7+ |
| AI provider config + HTTP calls | ~250 | Slice 7+ |
| Direct SQL / persistence bypass | ~120 | Slice 2+ (ongoing) |
| Hardcoded wordbook catalog | ~30 | Slice 4+ |
| Pure bridge plumbing | ~200 | Stays in platform-mobile |

## Dependency flow (current)

```
platform-mobile --> app-core --> study-core
                    |                |
                    v                v
                storage-core <-------
```

## What does NOT belong in shared contracts

- Android/iOS path strings
- JNI/ObjC symbol names
- Raw native exception text
- Secure-storage backend identity
- Provider failover implementation
- JSON-string bridge encoding details
- Hardcoded API keys (security issue)

## Known security issue

`bridge.rs` lines 668-675 contain hardcoded API keys:
- `DEFAULT_PRIMARY_AI_KEY` (Anthropic)
- `DEFAULT_BACKUP_AI_KEY` (OpenAI)

These MUST be moved to environment variables or secure storage before any public release.

## Test coverage for extracted services

| Service | Tests | Location |
|---|---|---|
| wrong_words_service | 3 | `app-core/src/services/wrong_words_service.rs` |
| reports_service | 4 | `app-core/src/services/reports_service.rs` |
| Total new service tests | 7 | All passing |

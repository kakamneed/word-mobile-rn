# Bridge Error Model

Status: Draft
Owner: Flutter bridge layer + native adapter layer
Phase: Slice 4 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how bridge and runtime failures should be categorized before they reach Flutter feature code.

The goal is to keep platform and transport failures distinct from domain failures.

## Error categories

### `domain_error`

Used when:

- a business rule is violated
- a session transition is invalid
- an operation is semantically rejected by Rust domain logic

Examples:

- invalid study state transition
- answer submitted for a missing question
- unsupported plan mutation under current domain rules

### `runtime_error`

Used when:

- Rust runtime cannot initialize
- storage or migration prerequisites fail
- bundled resources are missing

Examples:

- database init failure
- seed resource not available
- bootstrap cannot complete

### `platform_error`

Used when:

- Android/iOS adapter logic fails
- native secure storage fails
- platform sandbox path cannot be prepared

Examples:

- keychain/keystore access failure
- application support directory failure
- JNI/ObjC adapter setup failure

### `protocol_error`

Used when:

- the bridge contract cannot be decoded
- required fields are missing
- an unexpected null or malformed payload is returned

Examples:

- JSON decode failure
- missing required enum field
- payload shape drift between Rust and Flutter

### `unsupported_error`

Used when:

- the requested operation is not implemented on the current platform or build

Examples:

- feature stub still active on a target platform
- symbol not linked in the current build

## Error mapping path

```text
Rust/native failure
  -> native adapter classification
    -> bridge error envelope
      -> Flutter SDK error mapping
        -> feature/UI presentation
```

## Slice 4 first-pass decisions

- every failure must map to exactly one category
- bridge returns a stable error envelope instead of ad hoc platform strings
- Flutter SDK throws normalized typed exceptions in v1
- startup-blocking failures are routed differently from in-flow feature failures
- retryability must come from structured fields, not message parsing

## Error envelope fields

Suggested minimum bridge envelope:

- `category`
- `code`
- `message`
- `retryable`
- `details` optional

Example:

```json
{
  "category": "runtime_error",
  "code": "DATABASE_INIT_FAILED",
  "message": "Database initialization failed.",
  "retryable": true
}
```

Rules:

- `message` is user-safe and stable enough for normal app handling
- adapter may log richer diagnostics internally
- `details` is optional and should not become a contract dump bucket

## Startup vs in-flow handling

### Startup-blocking failures

Typical categories:

- `runtime_error`
- `platform_error`
- `protocol_error` during bootstrap decode

Expected UI posture:

- route to app-level startup error state
- offer retry if `retryable = true`
- do not render a fake ready app shell

### In-flow feature failures

Typical categories:

- `domain_error`
- `runtime_error`
- `protocol_error`
- `unsupported_error`

Expected UI posture:

- keep the rest of the app stable
- show feature-local error state where appropriate
- avoid collapsing feature failure into app-wide crash

## UI presentation rules

- UI should present user-safe descriptions, not raw native stack traces.
- Retry affordance should depend on `retryable`, not guessed from message text.
- Startup-blocking failures should route differently from in-flow feature failures.
- Unsupported feature errors should not be presented as generic crashes.
- Domain errors should not be mislabeled as transport failures.

## Logging rules

- Internal logs may retain a richer native diagnostic payload.
- User-facing error surfaces must not expose:
  - secret values
  - token material
  - full file paths unless intentionally diagnostic
  - raw native stack frames in normal UX

## Mapping examples

| Source failure | Category | Suggested code |
|---|---|---|
| Rust returns invalid state transition | `domain_error` | `INVALID_SESSION_STATE` |
| Rust runtime init returns null/empty failure | `runtime_error` | `RUST_RUNTIME_INIT_FAILED` |
| Native secure storage access fails | `platform_error` | `SECURE_STORAGE_UNAVAILABLE` |
| Flutter decode of DTO fails | `protocol_error` | `BRIDGE_DECODE_FAILED` |
| Platform symbol not linked | `unsupported_error` | `BRIDGE_SYMBOL_NOT_LINKED` |

## Retryability guidance

### Usually retryable

- temporary path/bootstrap preparation failure
- storage temporarily unavailable
- transient runtime initialization failure

### Usually not retryable without user or developer action

- invalid contract shape
- symbol not linked
- unsupported feature stub

### Depends on domain semantics

- some domain errors are final
- some domain errors may become valid after user state changes

## Downstream handoff

- Slice 5 can reuse startup-blocking vs in-flow boundaries for auth/runtime behavior
- Slice 6 can build Today/Plan/Study feature UX without guessing failure classes
- Slice 7 can extend the same error model to reports/wrong words/AI surfaces

## Open questions

- Whether protocol errors should immediately fail CI in debug builds
- Whether some startup failures should carry a diagnostics token for support workflows
- Whether error-code generation should later be derived from Rust source metadata

## Exit criteria

- Each bridge failure path maps into exactly one category.
- Flutter SDK can distinguish retryable runtime failures from domain failures.
- Startup and feature-local handling are both explicit.
- Feature teams do not need to inspect native adapter internals to decide UI behavior.

# Phase 3: React Native Bootstrap and Mobile Runtime - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase proves that the mobile app can boot as a real React Native product shell, call into the shared Rust core through a real native bridge, initialize and migrate SQLite inside mobile sandbox paths, install formal bundled seed vocabulary offline, and block startup with explicit recovery paths when runtime prerequisites fail. The phase also validates the runtime deeply enough to launch one real study session from the mobile side. It does not deliver the full mobile study UI or later mobile product surfaces.

</domain>

<decisions>
## Implementation Decisions

### React Native native integration
- **D-01:** Phase 3 uses a real React Native native bridge from the start rather than mocks or placeholder JS-only adapters.
- **D-02:** The bridge must be capable of calling shared-core entrypoints for bootstrap, settings, today-home, database initialization, and a real study-launch path during this phase.
- **D-03:** Temporary fake mobile runtime responses are explicitly out of scope because they would not validate the core mobile-runtime risks this phase exists to de-risk.

### Offline seed baseline
- **D-04:** The mobile app package must include formal bundled seed resources rather than only a tiny repair seed.
- **D-05:** First launch must install or expose the bundled formal seed baseline inside the mobile runtime sandbox so users can proceed offline without waiting for remote content.
- **D-06:** Seed handling should preserve the desktop app's core policy: validated local baseline first, remote update later.

### Failure and recovery behavior
- **D-07:** Runtime initialization failures must block startup into a clear error state rather than silently degrading into a partially usable shell.
- **D-08:** Blocking states must present explicit recovery actions or diagnostics for issues such as path resolution failure, SQLite init failure, seed-missing failure, or runtime migration failure.
- **D-09:** Silent destructive rebuild behavior is not acceptable as the default recovery path in this phase.

### Phase 3 validation scope
- **D-10:** Phase 3 is not complete with only bootstrap/status reads; it must validate one real study-launch path from the mobile side as part of runtime viability.
- **D-11:** The acceptance baseline is: cold start, SQLite init and persistence, bundled formal seed availability, bootstrap/settings/today-home reads, and one actual study-session launch.
- **D-12:** Full study-session UI completion remains out of scope for this phase and belongs to Phase 5.

### the agent's Discretion
- Exact RN native module implementation style and file layout
- Exact startup error-state UI wording and structure
- Exact smoke-test harness shape for validating the study launch
- Exact packaging layout for bundled seed assets inside iOS and Android project structure

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile direction, constraints, and offline-first guardrails
- `.planning/REQUIREMENTS.md` - Phase 3 must satisfy `MOB-01`, `MOB-04`, and `MOB-05`, while supporting later `MOB-02`
- `.planning/ROADMAP.md` - official Phase 3 scope and success criteria
- `.planning/STATE.md` - current planning status and prior locked decisions
- `.planning/phases/01-shared-rust-core-extraction-baseline/01-CONTEXT.md` - prior locked architecture and migration rules
- `.planning/phases/02-cross-platform-contract-and-desktop-refactor/02-CONTEXT.md` - prior locked contract and facade rules

### Desktop runtime reference implementation
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/bootstrap_service.rs` - current blocking bootstrap sequence and startup readiness policy
- `../word-desktop-tauri/apps/desktop/src-tauri/src/persistence/app_paths.rs` - current runtime-path and bundled-resource resolution behavior
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/runtime_storage_service.rs` - controlled runtime-root preparation and migration behavior
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/local_seed_service.rs` - formal seed installation and database repair policy
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/snapshot_service.rs` - bundled snapshot existence check used by bootstrap
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/settings_service.rs` - settings read/write baseline for early mobile bridge coverage
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs` - reference for the minimum real study-launch path Phase 3 must prove

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/bootstrap_service.rs`: already defines a strong blocking bootstrap sequence that can be mapped into the mobile runtime facade
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/local_seed_service.rs`: already expresses the desired "formal bundled baseline first" rule and can anchor mobile seed logic
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/settings_service.rs`: gives a small, clean early bridge target for validating read/write contract flow
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs`: contains the minimal real study-launch path that can be used to prove runtime viability without delivering the full mobile study experience yet

### Established Patterns
- The desktop runtime blocks startup when initialization fails in important ways instead of silently entering a degraded state
- Runtime paths, bundled resource resolution, and local seed repair are currently platform-aware and must be re-expressed behind the shared `PlatformServices/AppRuntime` boundary
- The product already treats formal local seed data as essential to offline-first behavior
- Settings, bootstrap, and today-home are the safest first real bridge surfaces, but a true runtime proof also needs one study-launch path

### Integration Points
- The mobile runtime bridge should attach at the shared facade layer locked in Phase 2
- Platform-mobile path resolution must replace direct Tauri `AppHandle` path calls for database, vocab cache, logs, and bundled resources
- Seed install and validation must happen early enough in startup to support offline bootstrap and today-home readiness
- The final runtime smoke path should exercise: app shell launch -> bootstrap -> settings/today-home read -> study session launch

</code_context>

<specifics>
## Specific Ideas

- The user explicitly chose the heavier but more truthful path of using a real RN native bridge in Phase 3 instead of a temporary fake shell.
- The user wants formal bundled seed data, not a tiny repair payload, inside the mobile app package from the start.
- The user wants failures to be visible and blocking rather than silently patched over.
- The user wants Phase 3 to prove enough runtime integrity to launch one real study session, even though full study UI belongs later.

</specifics>

<deferred>
## Deferred Ideas

- Full mobile study-session UI and answer loop - belongs to Phase 5
- Full mobile onboarding and today-home product experience - belongs to Phase 4
- Long-lived sync, notifications, and mobile release operations - later phases
- Broader runtime recovery automation beyond explicit blocking recovery actions - can evolve after Phase 3 proves baseline correctness

</deferred>

---

*Phase: 03-react-native-bootstrap-and-mobile-runtime*
*Context gathered: 2026-04-09*

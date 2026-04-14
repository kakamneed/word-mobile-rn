# Phase 8: Release Hardening and Store Readiness - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase hardens the mobile app for real-world distribution by focusing on runtime resilience, upgrade visibility, release validation, and store submission readiness. It must make the app trustworthy under cold start, restart, interruption, offline, and degraded-runtime conditions, and it must prepare the non-code packaging and store-facing assets required for release. It does not add new learning features.

</domain>

<decisions>
## Implementation Decisions

### Release update strategy
- **D-01:** Mobile release updates should present users with a clear update notice plus version details rather than only a bare binary "update available" prompt.
- **D-02:** The update surface should show at least the installed version, latest version, and release notes or summary before sending users to the store or update target.
- **D-03:** Mobile update UX should guide users clearly, but it should not attempt an overly complex in-app update workflow that fights the platform store model.

### Exception recovery and user visibility
- **D-04:** Key runtime exceptions must be explicitly visible to users and paired with recovery actions or diagnostics.
- **D-05:** Silent recovery is not an acceptable default for the key categories of release-hardening failures in this phase.
- **D-06:** The app should preserve the already-established product principle that important runtime failures are understandable and actionable rather than hidden behind vague degradation.

### Pre-release validation strength
- **D-07:** Phase 8 uses a strong validation gate rather than a basic or moderate smoke-only gate.
- **D-08:** Release readiness must require verification across cold start, restart, background/foreground interruption, offline behavior, upgrade or migration recovery, and fallback scenarios.
- **D-09:** The release gate must treat these scenarios as mandatory pre-release checks rather than nice-to-have QA extras.

### Store-readiness scope
- **D-10:** Phase 8 should bring the app to full store-readiness rather than only technical package generation.
- **D-11:** Store readiness includes the technical build plus store-facing materials such as icons, permissions or privacy explanation, release notes, and submission-ready presentation assets.
- **D-12:** The phase should close with the app meaningfully prepared for real submission, not just locally buildable.

### the agent's Discretion
- Exact shape and placement of the in-app update notice
- Exact recovery UI hierarchy for different hard failure categories
- Exact validation checklist formatting and automation split between code and manual QA
- Exact set of store materials grouped as mandatory versus supporting

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile-first and offline-first constraints
- `.planning/REQUIREMENTS.md` - Phase 8 must satisfy release-hardening concerns implied by runtime, storage, and non-blocking AI principles
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 8
- `.planning/STATE.md` - current planning status and prior locked decisions
- `.planning/phases/03-react-native-bootstrap-and-mobile-runtime/03-CONTEXT.md` - prior decisions on blocking runtime failures and strong runtime validation
- `.planning/phases/07-reports-wrong-words-and-ai-surfaces/07-CONTEXT.md` - latest information architecture decisions that now need release-grade stability

### Desktop reference implementation
- `../word-desktop-tauri/apps/desktop/src/lib/release-update-client.ts` - version-check UX and status semantics reference
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/release_manifest_service.rs` - release-manifest fetch and status semantics reference
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/bootstrap_service.rs` - blocking bootstrap severity policy reference
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/runtime_storage_service.rs` - runtime migration and storage-preparation behavior reference

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/lib/release-update-client.ts`: already defines a release-update state model and user-facing version/status semantics that can inform mobile's store-based update prompt
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/release_manifest_service.rs`: shows how release metadata, latest version, and notes are already modeled in the product
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/bootstrap_service.rs`: already encodes the desired blocking-failure startup philosophy
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/runtime_storage_service.rs`: already captures migration-sensitive runtime preparation logic and failure modes that should inform mobile hardening and QA

### Established Patterns
- The product already prefers explicit visibility for important runtime failures
- Update semantics already include version metadata and user-facing notes, not just a raw yes/no check
- Runtime migrations and fallback behaviors are considered product-critical, not hidden implementation details
- Offline viability remains a first-class constraint even at release time

### Integration Points
- Mobile hardening should sit on top of the runtime guarantees already built in Phases 3 through 7
- The update surface belongs in the mobile settings or app-maintenance domain, but its status model should stay consistent with the shared release concept
- Release validation must touch startup, study, Today, vocabulary, review center, and runtime recovery paths
- Store-readiness work must cover both technical packaging and user-facing submission materials

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants update prompting to include version details and explanatory context, not just a raw update badge.
- The user wants key exceptions to remain visible and actionable rather than silently repaired.
- The user explicitly chose the strongest release validation bar among the available options.
- The user wants this phase to end at full store-readiness, not just successful local release builds.

</specifics>

<deferred>
## Deferred Ideas

- Overly elaborate in-app update systems that exceed normal store expectations
- New product features unrelated to hardening and release readiness
- Any attempt to weaken runtime failure visibility in favor of silent behavior
- Post-launch analytics or support workflows that go beyond submission readiness

</deferred>

---

*Phase: 08-release-hardening-and-store-readiness*
*Context gathered: 2026-04-09*

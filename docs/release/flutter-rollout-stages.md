# Flutter Rollout Stages

Status: Draft
Owner: Release management + mobile platform team
Phase: Slice 9 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the staged rollout path for replacing the React Native mobile shell with the Flutter shell.

## Core principle

A runnable Flutter build is not enough to replace RN.

Promotion between rollout stages must depend on:

- functional parity where promised
- local continuity safety
- acceptable auth/session behavior
- acceptable sync behavior for the scoped release

## Slice 9 first-pass decisions

- rollout happens in explicit stages, not one irreversible jump
- RN may overlap temporarily for release safety, not as a permanent equal front
- promotion decisions come from gates and signals, not team intuition alone
- rollback remains a first-class product path throughout rollout

## Stage model

### Stage 1: Internal verification

Audience:

- core engineering team
- local device smoke validation

Required checks:

- bootstrap
- today
- plan
- study
- wrong words
- reports
- non-blocking AI behavior

Exit criteria:

- no blocker-level failures in the core loop

### Stage 2: Limited beta

Audience:

- small internal/beta user group

Focus:

- real-device variability
- auth/session restore
- local continuity
- upgrade behavior

Exit criteria:

- core loop remains stable across real-user usage

### Stage 3: Staged rollout

Audience:

- controlled production percentage

Focus:

- crash trends
- bootstrap failure rate
- upgrade migration results
- rollback readiness

Exit criteria:

- metrics remain within agreed threshold

### Stage 4: Mainline replacement

Audience:

- default production population

Focus:

- Flutter becomes the primary mobile client
- RN remains only for rollback/support window if needed

Exit criteria:

- release gates fully satisfied

### Stage 5: RN decommission

Audience:

- post-cutover production state

Focus:

- retire RN as an actively shipped client
- close rollback window

Exit criteria:

- no remaining production dependency on RN client rollout

## Promotion gate categories

- functional gate
- continuity gate
- sync gate
- operational gate

## Gate highlights

### Functional gate

- bootstrap to today works
- plan read/save/apply works
- study complete/cancel/resume works
- wrong words and reports match baseline
- AI remains non-blocking

### Continuity gate

- local SQLite migration succeeds
- restart restore succeeds
- logout keep-local policy behaves as defined
- guest/sign-in transitions behave as defined

### Sync gate

- outbox persists across restart
- duplicate delivery does not double-count
- pull/apply does not corrupt local truth
- sync failure does not block local study

### Operational gate

- crash-safe error surfaces exist
- runtime diagnostics are visible enough for support
- version/build/update information is visible

## Anti-patterns

- promoting based on "looks good on dev build"
- skipping staged rollout because internal QA passed
- keeping RN and Flutter as equal long-term feature fronts

## Exit criteria

- Rollout stages are explicit enough to coordinate execution and release decisions.
- Promotion gates are concrete enough to drive go/no-go calls.

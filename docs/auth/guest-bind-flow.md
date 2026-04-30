# Guest Bind Flow

Status: Draft
Owner: Mobile auth layer + sync layer
Phase: Slice 5 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how a guest/local-only user becomes attached to a cloud account without losing local study history.

## Goal

Support account introduction without forcing one unsafe default:

- auto-overwrite cloud with local
- auto-overwrite local with cloud
- or silently mixing both without domain-specific merge rules

## Slice 5 default decisions

- guest can study fully before login
- login does not equal destructive bind
- cloud empty -> local becomes first sync source
- cloud already populated -> enter explicit bind/merge decision flow
- bind interruption must not corrupt local runtime

## Entry states

### Guest with local-only data

- no cloud identity
- local history exists
- can study fully offline

### Existing cloud account

- remote plan/preferences/history may already exist

### Empty cloud account

- auth exists but product data is effectively empty

## Pre-bind checks

Before any bind decision is finalized, the app should know:

- whether secure auth session is valid
- whether local runtime contains meaningful syncable product data
- whether cloud contains meaningful syncable product data
- whether an in-progress study session exists locally

Important rule:

- in-progress session truth remains local; bind should not reinterpret it as already-synced final history

## Bind state machine

| State | Meaning |
|---|---|
| `guest_local_only` | no account attached |
| `login_success_pending_bind_check` | auth exists, local/cloud relationship not yet resolved |
| `bind_safe_auto_attach` | cloud empty, local can become initial source |
| `bind_needs_merge_decision` | both local and cloud contain meaningful data |
| `bind_completed` | account attached with safe next-step sync posture |
| `bind_aborted_keep_local` | user leaves account attach flow without data loss |

## Bind paths

### Path A: Guest -> empty cloud account

Recommended default:

- bind succeeds
- local data becomes first cloud-sync source
- no destructive overwrite required

Operational result:

- local runtime remains authoritative
- future sync may upload syncable domains
- no forced user confirmation is needed if cloud is effectively empty

### Path B: Guest -> existing cloud account

Recommended default:

- do not auto-merge blindly
- enter explicit bind/merge decision flow
- use domain-specific merge rules from [merge-strategy.md](/d:/projects/word-mobile-rn/docs/supabase/merge-strategy.md)

Operational result:

- account is authenticated
- sync is not allowed to guess destructive winners
- local study continuity remains available while decision is pending

### Path C: Guest aborts bind

Recommended default:

- remain local-only
- no local data loss
- no partial account attachment artifacts left behind

Operational result:

- clear temporary bind-pending markers if they would confuse future startup
- keep auth/login state only if product explicitly wants "signed in but not bound yet"; otherwise return cleanly to local-only

## Domain handling during bind

| Domain | Empty cloud account | Existing cloud account |
|---|---|---|
| plan configs | upload local later | compare versions / explicit merge decision |
| wordbook preferences | upload local later | low-risk merge by policy |
| study history | local becomes first event source | append/merge by event rules, not overwrite |
| wrong-word state | rebuild from uploaded source truth | rebuild after merge, do not trust row overwrite |
| reports | rebuild from merged study truth | rebuild from merged study truth |
| AI history | upload owned local artifacts later | merge by stable ids where possible |

## Required decisions in the flow

- whether bind is automatic after login or requires confirmation
- whether a summary screen shows what exists locally vs in cloud
- whether some domains can merge automatically while others require user confirmation

Recommended first-pass answer:

- automatic only for empty-cloud-account path
- existing-cloud-account path should show explicit "local + cloud both have data" messaging
- user confirmation is needed only for meaningful authored-data conflicts, not for every low-risk preference row

## Local markers

Slice 5 may need local metadata markers such as:

- bind pending status
- last seen authenticated `user_id`
- last bind outcome
- migration or merge required flag

Rules:

- these markers are runtime coordination metadata, not replacements for domain truth
- marker corruption must not destroy study history

## Interruption and retry safety

- app kill during bind must return to a typed recoverable state
- failed network step must not partially delete local truth
- failed merge preparation must pause cloud attach, not break guest mode
- retrying bind should be idempotent at the flow level where possible

## Anti-patterns

- auto-uploading all local data into a pre-existing cloud account without user awareness
- deleting local data immediately after first successful login
- resolving every domain by timestamp alone
- making "signed in" imply "safe to start sync immediately"

## Minimum UX requirements

- user understands whether they are:
  - linking to an empty account
  - attaching to an existing account
  - staying local-only
- user understands whether local data will be kept
- bind interruption should not corrupt local runtime
- failure state distinguishes auth success from bind decision still pending

## Exit criteria

- Bind paths are explicit.
- Existing-cloud-account behavior is defined.
- Local/cloud conflict posture is explicit.
- Local data preservation is guaranteed unless the product explicitly says otherwise.

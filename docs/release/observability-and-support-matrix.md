# Observability And Support Matrix

Status: Draft
Owner: Release management + mobile platform + support
Phase: Slice 9 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the minimum observability and support signals needed to operate the Flutter rollout safely.

## Slice 9 first-pass decisions

- release decisions need runtime, auth, sync, and support visibility
- continuity failures matter, not only crash metrics
- support must diagnose failure class without raw secret access

## Signal categories

### Runtime health

- crash rate by version/build
- bootstrap success/failure rate
- startup error category breakdown

### Core loop health

- today load success rate
- study start success rate
- study submit success rate
- study complete success rate
- resume-after-restart success rate

### Auth health

- session restore failure rate
- token refresh failure rate
- logout-related error rate

### Sync health

- pending upload growth
- sync success rate
- dead-letter rate
- duplicate-protection anomalies

## User-visible diagnostics

The app should make it easy to expose at least:

- installed version/build
- high-level runtime status
- last successful sync time when sync is enabled
- high-level last sync error code when useful

## Support use cases

- identify whether the issue is bootstrap/runtime, auth, sync, or study-flow related
- confirm the user build/version
- confirm whether the device is failing before or after local runtime bootstrap

## Anti-patterns

- silent critical runtime failures
- support workflows requiring raw secret/token access
- observability that exists only for crashes but not for continuity failures

## Exit criteria

- The rollout has enough runtime, auth, sync, and support visibility to make go/no-go and rollback decisions.
- Support can distinguish major failure classes without privileged client-secret access.

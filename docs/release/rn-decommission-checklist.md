# RN Decommission Checklist

Status: Draft
Owner: Mobile platform team
Phase: Slice 9 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the checklist for retiring React Native as the active mobile client after Flutter becomes the primary shell.

## Principle

RN should not remain an indefinite parallel front once Flutter becomes the forward path.

## Checklist

### Product readiness

- [ ] Flutter core learning loop is production-acceptable
- [ ] continuity/restart behavior is production-acceptable
- [ ] auth/account behavior is production-acceptable
- [ ] sync behavior is acceptable for scoped rollout stage

### Release readiness

- [ ] rollback window has closed or is explicitly narrowed
- [ ] rollout metrics are stable
- [ ] no high-severity RN-only fallback dependency remains

### Support readiness

- [ ] support docs point to Flutter build line
- [ ] diagnostics and observability point to Flutter runtime
- [ ] on-call/release owners know RN is no longer the active client path

### Engineering readiness

- [ ] new mobile feature work no longer lands in RN except emergency support if still needed
- [ ] contract drift between RN and Flutter is no longer growing
- [ ] RN-specific release workflow is no longer required for normal delivery

## Anti-patterns

- keeping RN alive "just in case" without clear ownership
- continuing dual feature development in RN and Flutter
- declaring RN decommissioned before rollback/support window is understood

## Exit criteria

- The team can clearly answer when RN stops being a first-class shipped client.
- Decommission depends on explicit readiness, not vague confidence.

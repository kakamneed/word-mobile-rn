# Flutter Diagnostics Surface

Status: Active draft
Owner: Mobile runtime / support
Phase: Slice 8 of `2026-04-22-flutter-app-supabase-implementation`

## Purpose

This document lists the diagnostics that must be visible before internal
rollout. Diagnostics should help classify failures without exposing secrets.

## Current Visible Surfaces

### Bootstrap

- Rust bridge loaded/initialized through bridge status.
- Bootstrap state distinguishes app ready, first-run required, and blocking
  reason.
- Startup error route displays a code/message.

### Account

- `notConfigured`
- `guestLocalOnly`
- `signedInActive`
- `signedInExpired`
- `signedOutRetainedLocal`
- `accountDeletedOrRevoked`
- `error`

Flutter feature code must not display raw Supabase tokens, refresh tokens, or
provider secrets.

### Sync

- sync enabled
- transport configured
- account sync state
- pending count
- pending domains
- last successful sync timestamp
- last high-level error code

Flutter can read these values but must not mutate outbox, cursor, or dead-letter
rows directly.

## Missing Before Wider Rollout

- App version/build number surfaced in-app.
- Database schema version surfaced in diagnostics.
- Last bridge initialization failure persisted for support capture.
- Supabase local/production environment label shown without exposing keys.
- Cloud sync transport result codes once push/pull exists.

## Secret Safety

Never include these in diagnostics:

- Supabase anon/service role keys
- access tokens
- refresh tokens
- AI provider tokens
- full user answer payloads
- raw sync outbox payloads

# Mobile Bundle Resources

This directory is the planned source-of-truth location for mobile bundle resources
that must be copied into the iOS app bundle during build.

## Expected layout

- `vocab-snapshot/vocab-snapshot.jsonl`

## Current status

- Android seed assets are still sourced from `apps/mobile/android/app/src/main/assets/`
- The iOS build now copies `seed-vocab` and `seed-medical` from Android assets into the app bundle
- `vocab-snapshot/vocab-snapshot.jsonl` is still missing and must be supplied here before the iOS bootstrap path can pass the required snapshot check

# Neutral Export Format

Date: 2026-05-05

Neutral exports are the bridge between Supabase, Aliyun PostgreSQL, local
PostgreSQL, and any future cloud PostgreSQL environment. They should be boring,
inspectable, and restorable without Aliyun-only tooling.

## Directory shape

```text
export-YYYYMMDD-HHMMSS/
  manifest.json
  tables/
    users.jsonl
    profiles.jsonl
    devices.jsonl
    plan_configs.jsonl
    wordbook_preferences.jsonl
    study_word_points.jsonl
    wrong_word_entries.jsonl
    ai_passages.jsonl
    report_snapshots.jsonl
    leaderboard_stats.jsonl
    announcements.jsonl
  checksums/
    row-counts.json
    table-sha256.json
  objects/
    object-manifest.jsonl
```

`objects/object-manifest.jsonl` is required only if object storage is introduced
or migrated.

## Manifest

`manifest.json`:

```json
{
  "formatVersion": 1,
  "source": "supabase",
  "createdAt": "2026-05-05T09:00:00Z",
  "schema": "aliyun-initial",
  "tables": [
    "users",
    "profiles"
  ],
  "notes": []
}
```

## Table files

Each table file is JSON Lines:

```jsonl
{"id":"uuid","email":"user@example.com","created_at":"2026-05-05T09:00:00Z"}
{"id":"uuid","email":"other@example.com","created_at":"2026-05-05T09:01:00Z"}
```

Rules:

- One row per line.
- UTF-8.
- No trailing commas.
- Timestamps use ISO-8601 strings.
- UUIDs remain strings.
- JSONB columns remain nested JSON objects.
- Empty optional fields may be `null`.

## Auth migration note

Supabase Auth password hashes may not be portable into the first-party auth
model. The neutral export should preserve stable user IDs, email, profile data,
and metadata, but the cutover may still need a password reset or re-login path.

## Integrity checks

Minimum checks:

- Row counts by table.
- SHA-256 per table file.
- Foreign key relationship checks after import.
- Selected user-owned aggregate counts:
  - profile per user,
  - plan config per user,
  - report snapshots per user,
  - wrong-word entries per user,
  - study word points per user.

## Import order

Recommended import order:

1. `users`
2. `profiles`
3. `devices`
4. `plan_configs`
5. `wordbook_preferences`
6. `study_word_points`
7. `wrong_word_entries`
8. `ai_passages`
9. `report_snapshots`
10. `leaderboard_stats`
11. `announcements`
12. `object_manifests`

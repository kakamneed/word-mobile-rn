# Aliyun Preflight Checklist

Date: 2026-05-07

## Current Freeze

- Freeze dump: `backups/word_admin_dev_2026-05-07_aliyun_freeze.dump`
- Pre-cleanup dump: `backups/word_admin_dev_2026-05-07_before_aliyun_freeze_cleanup.dump`
- Local database URL: `postgres://word_admin@127.0.0.1:55432/word_admin_dev`
- Backend project: `D:\projects\word-admin`
- Mobile project: `D:\projects\word-mobile-rn`

## Frozen Data Shape

Current production candidate data:

- Users: 2
- Study word points: 418
- Study attempts: 418
- Study correct: 328
- Study wrong: 83
- Max study point attempt count: 1
- Max study point wrong count: 1
- Wrong word entries: 78
- Wrong word total errors: 139
- Max wrong word error count: 5
- Report snapshots: 0
- AI passages: 8
- Test users: 0

Report snapshots are intentionally empty in the freeze. The app should rebuild reports from restored local study data, avoiding migration of previously polluted aggregate report snapshots.

## Verified Checks

Run locally before buying or deploying the ECS:

```powershell
cd D:\projects\word-admin
npm.cmd test
$env:DATABASE_URL='postgres://word_admin@127.0.0.1:55432/word_admin_dev'
npm.cmd run test:postgres
```

Run mobile bridge check:

```powershell
cd D:\projects\word-mobile-rn
cargo check -p word-platform-mobile
```

Expected current status:

- Static backend checks pass.
- Postgres smoke check passes.
- Mobile bridge check passes.
- Backend `/health` returns `storeMode: postgres`.

## Security Gates Already Added

- JSON request body limit: 1 MB.
- Sync array limits:
  - wordbook preferences: 20
  - study word points: 500
  - wrong word entries: 500
- AI passage and report snapshot payload limit: 512 KB.
- Study point validation:
  - `attemptCount <= 20`
  - `wrongCount <= 5`
  - response time between 0 and 1 hour
  - `correctCount + wrongCount <= attemptCount`
- Wrong word projection is capped at 5 errors per word.
- Cloud restore does not expand aggregate attempts into many local `study_results` rows.

## Do Not Migrate

Do not migrate these local-only artifacts:

- `D:\projects\word-admin\.pgdata`
- `D:\projects\word-admin\node_modules`
- `D:\projects\word-admin\.npm-cache`
- local `.pglog`
- mobile build outputs
- temporary smoke users

Use the dump file and source code instead.

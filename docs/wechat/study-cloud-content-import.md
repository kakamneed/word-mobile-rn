# Study Cloud Content Import

Date: 2026-05-21
Status: Cloud schema and seed-vocab content imported

## Purpose

The Mini Program Study backend is now a light frontend flow:

- Taro sends mode/wordbook/source ids.
- `server/miniprogram-api` sources word payloads.
- Rust `word-study-core` builds questions, evaluates answers, and builds summary.
- The adapter writes `study_events`, `report_snapshots`, and `wrong_word_entries`.

Production vocabulary content now lives in `public.study_entry_payloads`.
Normal word payloads and root/affix payloads share this table.

## Cloud Schema

Preferred normal path:

```powershell
scripts\supabase-local.cmd db push
```

If the CLI asks for login first, provide a Supabase access token or run:

```powershell
scripts\supabase-local.cmd login
scripts\supabase-local.cmd db push
```

2026-05-21 execution note: `db push` was blocked by an older pending reward
image migration whose existing remote function has a different return type. To
avoid changing older migration semantics, only
`supabase/migrations/202605200002_miniprogram_study_entry_payloads.sql` was
applied through `supabase db query --linked -f ...`, then migration history was
repaired for version `202605200002`.

## Export Payloads

From an existing local Word Mobile SQLite database:

```powershell
node server\miniprogram-api\scripts\export-study-entry-payloads.mjs --source=D:\path\word-mobile.sqlite --outputJson=server\miniprogram-api\out\study-entry-payloads.json --outputSql=server\miniprogram-api\out\study-entry-payloads.sql
```

From Flutter/mobile bundled assets, which is the preferred source for Mini
Program production import:

```powershell
node server\miniprogram-api\scripts\export-study-entry-payloads.mjs --source=apps\flutter_mobile\build\app\intermediates\assets\release\mergeReleaseAssets --outputJson=server\miniprogram-api\out\study-entry-payloads.json --outputSql=server\miniprogram-api\out\study-entry-payloads.sql
```

From the same kajweb seed-vocab book JSON used by the desktop/mobile seed
import flow, when Flutter build assets are unavailable:

```powershell
node server\miniprogram-api\scripts\export-study-entry-payloads.mjs --source=D:\projects\word-desktop-tauri\apps\desktop\src-tauri\seed-vocab\book --outputJson=server\miniprogram-api\out\study-entry-payloads.json --outputSql=server\miniprogram-api\out\study-entry-payloads.sql
```

Root/affix content follows Flutter `crates/platform-mobile/src/bridge.rs`:

- shared root/affix cards are parsed from bundled `seed-vocab/book/*.json`
  `remMethod` values and filtered with the same allowlist/reliability rules;
- medical root/affix cards prefer bundled
  `seed-medical/medical-root-affix.txt`, then fall back to the Rust built-in
  medical cards when file rows are not reliable;
- backend `rootAffix` mode reads `root_affix_shared_*` for CET/KaoYan
  wordbooks, `root_affix_medical_*` for the medical wordbook, and both when no
  wordbook is scoped.

If the machine has no `sqlite3` CLI, export/provide equivalent JSON in the same
shape as:

```text
server/miniprogram-api/test-fixtures/study-entry-payloads.json
```

Then run:

```powershell
node server\miniprogram-api\scripts\export-study-entry-payloads.mjs --source=server\miniprogram-api\test-fixtures\study-entry-payloads.json
```

## Import Payloads

After schema exists:

```powershell
cd server\miniprogram-api
node scripts\import-study-entry-payloads.mjs --input=out\study-entry-payloads.json
node scripts\cloud-study-payload-check.mjs
```

`import-study-entry-payloads.mjs` reads `SUPABASE_URL` and
`SUPABASE_SERVICE_ROLE_KEY` from `.env.supabase.local` by default.

2026-05-21 result:

- Source: `apps\flutter_mobile\build\app\intermediates\assets\release\mergeReleaseAssets`
- Exported/imported rows: `11563`
- Root/affix rows: `16` shared + `10` medical
- First generated source id: `CET4_3_1`
- Cloud read smoke: `node scripts\cloud-study-payload-check.mjs` returned
  sample rows plus root/affix counts from `public.study_entry_payloads`.

## Smoke

Local automated smoke:

```powershell
cd server\miniprogram-api
node scripts\check-all.mjs
```

Cloud content smoke:

```powershell
cd server\miniprogram-api
node scripts\cloud-study-payload-check.mjs
```

Manual Mini Program smoke still needs WeChat Developer Tools:

- Run the Taro dev/build flow.
- Open `apps/wechat_miniprogram/dist`.
- Use HTTP SDK mode against the first-party API.
- Start Study from Today and answer through a full session.
- Confirm Reports and Wrong Words update.

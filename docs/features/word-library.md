# Feature: Word Library

> Slug: `word-library`
> Status: `mobile_in_progress`
> Updated: `2026-07-16`

## Product Intent

词库是学习、复习、错题、AI 短文、词根词缀和后续桌面端词书管理的共同数据底座。它不只是 headword + 中文释义，而要提供足够干净、稳定、可复用的学习材料，让不同题型直接读取同一套正确答案、备选项、例句、词性和词根词缀信息，减少运行时临场生成导致的慢加载和不一致。

这轮用户反馈集中在四类问题：释义污染、选择题备选重复或误判、输入题判定边界、词根词缀质量。目标是“求精不求多”：词书内容宁可少一点，也要干净、相关、可解释，并能被移动端和桌面端一致复用。

## UX Contract

- 用户在任意场景看到同一个单词时，中文释义必须一致、干净，不允许混入 `<`、`>`、孤立选项字母 `A/B/C/D`、乱码或重复释义。
- AI 短文、学习题目、选项、错题详情和报告中使用的释义，都必须来自同一份清洗后的 entry payload。
- 选择题的正确选项必须和该题实际 `correct_choice_label` 绑定，不能出现用户点了视觉上的正确释义却被判错的情况。
- 每个普通词条应尽量在词书扩展数据中预置多个同词性、同词书、语义相近但不等价的备选项。移动端实际出题时从预置候选中抽 3 个，再和正确答案随机分配到 A/B/C/D。
- 同一次学习中，备选项应尽量不重复；当词书已经提供足够候选时，不应反复落回同几个全局 distractor。
- 输入题判定要接受有意义的部分答案，例如 `润滑油` 对 `动物脂，油脂；润滑油` 可判为基本正确；但不能把 `的` 这种停用词/孤立虚词判为基本正确。
- 词根词缀题只展示高质量条目：词根/词缀不能太生僻，至少应能找到两个相关例词；例词释义要短、去重、能体现该词根/词缀含义。像 `ear` 对 `gear/shear`、`exe` 对 `execute/execution` 这类意义无关或只是字符串片段的条目应过滤。
- 医学英语词库如果以词组为主，普通学习模式可以按词组学习；词根词缀模式必须避免把无法形成可靠词根/词缀关系的医学词组直接送入 root-affix 流程。

## Shared Domain/Data Contract

Shared Rust/domain/storage contracts:

- `StartSessionEntryPayload` now carries:
  - `cn_choice_distractors: Vec<String>`
  - `en_choice_distractors: Vec<String>`
- `WordForQuestion` mirrors those fields so `QuestionBuilder` can build choices without loading a large global distractor pool.
- SQLite schema version `15` adds `entry_question_preps`:
  - `entry_id INTEGER PRIMARY KEY`
  - `cn_choice_distractors_json TEXT NOT NULL DEFAULT '[]'`
  - `en_choice_distractors_json TEXT NOT NULL DEFAULT '[]'`
  - `updated_at TEXT`
- Seed vocabulary maintenance rebuilds question preps after import or maintenance. Candidate generation should prefer same wordbook + same part of speech, exclude the target entry, skip duplicate/identical meaning keys, cap each side around seven candidates, and serialize the result into `entry_question_preps`.
- Seed vocabulary entries may include `content.word.content.realExamSentence.sentences[]` generated from bundled exam-paper assets. Each item uses `sContent` for the original English exam sentence, `sCn` for a human-readable exam source label, and `source` for stable provenance such as exam, paper, section, question, and choice identifiers.
- Real-exam example enrichment must not cross exam families: CET4 books only use `cet4`, CET6 books only use `cet6`, Kaoyan books only use `kaoyan-english-1/2`, and medical books remain empty until a medical exam corpus exists.
- Runtime question construction uses precomputed pools first. If a payload has enough required distractors for the current question types, mobile should avoid loading the old 96-entry global fallback pool.
- Fallback global distractors remain a compatibility path for older DBs, incomplete preps, user-imported words, or sparse wordbooks.
- User-accepted disputed meanings are user data, not seed vocabulary, but must merge into entry payloads so accepted answers remain visible in later study, wrong words, and detail pages.
- Root/affix cards are represented as study payloads with `SessionMode::RootAffix`, but they should not use choice distractor preps; their quality gate is source extraction/filtering.

Affected code paths:

- `crates/storage-core/src/models/study_requests.rs`
- `crates/storage-core/src/persistence/schema.rs`
- `crates/study-core/src/question_builder.rs`
- `crates/platform-mobile/src/bridge.rs`
- `crates/app-core/src/facade/study_facade.rs`

## Flutter Mobile Route

- Owner screen/widget: Study screen, Today entry points, Plan wordbook selection, Wrong word detail, AI passage screen, root/affix study flow.
- SDK/bridge calls: `start_study_session`, study submit/accept-dispute calls, wordbook/plan state calls, AI passage context calls, wrong-word detail calls.
- Loading/cache/reload behavior:
  - On app DB initialization or seed maintenance, rebuild `entry_question_preps` once per maintenance key.
  - During session hydration, load selected entry payloads with precomputed choice candidates.
  - Only load global distractor payloads when required question types lack enough precomputed candidates.
  - Root/affix mode should load from filtered root-affix resources, not regular word distractors.
- Orientation/gesture constraints: mobile study UI is vertical and small-screen first; long Chinese glosses must be shortened upstream where possible, not solved only with UI wrapping.
- First implementation slice: mobile Rust bridge and study-core now support precomputed distractors and targeted fallback.
- Current status: mobile in progress; core path compiles and targeted tests pass, but generated seed assets still need one more data-quality pass and release build verification.

## Tauri Desktop Route

- Owner view/window: desktop word library management view, study session view, root/affix review view, and a future vocabulary QA/admin view.
- Shared APIs to reuse: Rust storage schema, `StartSessionEntryPayload`, `QuestionBuilder`, seed maintenance/rebuild logic, answer evaluator, wrong-word detail payloads.
- Desktop-specific layout:
  - Desktop should expose richer inspection tools: table/grid wordbook browser, per-entry meanings, precomputed CN/EN distractor lists, examples, root/affix source cards, and validation warnings.
  - Desktop can show batch QA states that mobile should not carry in the learning UI.
- Mobile assumptions to avoid:
  - Do not hide data-quality problems behind small-screen truncation.
  - Do not reimplement choice generation in frontend TypeScript/Rust UI layer; reuse shared Rust/domain logic.
  - Do not assume one active phone-sized study flow; desktop may compare wordbooks and inspect multiple entries at once.
- First parity slice: desktop can start with read-only wordbook browser + single-entry detail showing meanings, examples, and precomputed distractors, then reuse the same study session contract for quiz parity.
- Current status: planned.

## Sync And Storage

Seed vocab and generated `entry_question_preps` are app/content data. They can be rebuilt from bundled resources and should not sync as user-owned cloud state.

User-owned data includes learning progress, wrong words, user accepted meanings/disputes, hints, mastered entries, and cloud restore snapshots. User accepted meanings must be merged at read time so they appear consistently without mutating seed meanings.

Migration concerns:

- DBs before schema `15` need `entry_question_preps` created lazily during schema apply.
- Existing installs need a seed maintenance key bump so question preps are built after app update.
- If seed vocabulary is updated or cleaned again, maintenance key should bump to force rebuild.
- Sparse custom/imported word sets should continue to work with fallback distractors.

## AI Or Provider Implications

AI passage generation relies on cleaned `word`, `primary_gloss`, `part_of_speech`, and `entry_id`. If seed meanings contain `<`, dangling option letters, duplicate glosses, or root-affix fragments, AI passages will reproduce those errors. AI should not repair the vocabulary database at generation time; the seed/payload layer should provide clean data.

Wrong-word image/text import can add candidate words, but imported user words may lack rich precomputed distractors. Those should either use fallback distractors or later pass through an enrichment pipeline.

## Implementation Log

- `2026-08-12` - The Kaoyan derivational-family builder now separates the lexical `real/reality/really` branch from `realize/realise/realization` even when a source dictionary exposes all members in one broad `同根` block. Regenerating the 2010-2026 frequency assets changed the `real` family to 47 occurrences across `real`, `reality`, and `really`; the packaged portable family index uses the same boundary.
- `2026-08-10` - Real-exam example coverage repair: `scripts/enrich-seed-vocab-real-exam-examples.mjs` now reuses each entry's audited `realExamFrequency.surfaceForms` and source scope when matching and ranking examples. For Kaoyan positive-frequency entries, English I 2010-2026 source sentences outrank older English I and English II matches; `segment` now starts with its 2013 English I translation sentence followed by the three 2010 English I passage sentences.
- `2026-08-10` - Coverage audit: 2,109 of 2,134 positive Kaoyan English I frequency entries have an in-scope sentence. The remaining 25 are recorded in `docs/seed-vocab-real-exam-examples-report.json`: 24 occur only as cloze choices, often incorrect distractors, and `crew` is the brand token in `J.Crew`. No sentence is fabricated for those source fragments.
- `2026-04/05`: User reported `assimilate` gloss showing a stray `<` in AI passages, study questions, and options, indicating seed vocabulary pollution rather than a single UI bug.
- `2026-04/05`: User reported option text pollution such as `基金会 A`, and repeated distractor pools. Requirement clarified: options should be more random, same POS where possible, and not just three repeated global candidates.
- `2026-04/05`: User reported input-answer boundary: meaningful partial answer like `润滑油` should be accepted, but single function words like `的` should not.
- `2026-05`: User reported `explosive` could have two close correct gloss groups, causing visual correct choice to be marked wrong. Contract: rendered correct option and backend correct label must come from the same accepted meaning set.
- `2026-05`: User reported root/affix low-quality cards: one-example roots, duplicate example meanings, irrelevant string fragments, and overly obscure roots. Contract tightened to at least two relevant examples and shorter deduped example glosses.
- `2026-05`: User reported medical wordbook entering root-affix mode can crash. Root/affix flow must guard sparse/phrase-heavy wordbooks and never assume every wordbook can generate root-affix cards.
- `2026-06-25`: Introduced expanded study payload fields for precomputed CN/EN choice distractors and SQLite `entry_question_preps` cache.
- `2026-06-25`: Updated `QuestionBuilder` to consume precomputed distractors first, then fall back to same-POS/global distractor pools only when needed.
- `2026-06-25`: Updated mobile bridge hydration so sessions skip global 96-entry distractor loading when selected payloads already contain enough required precomputed candidates.
- `2026-06-25`: Added targeted tests for precomputed distractor usage and same-wordbook/same-POS prep generation.
- `2026-07-15` - Baseline reconciliation: Added seed-vocabulary distractor repair and audit tooling for CET4, CET6, Kaoyan, and medical books. The contract requires seven aligned CN/EN/source distractors, strict same-POS selection, and no semantic overlap with the target or sibling options.
- `2026-07-15` - Baseline reconciliation: Regenerated the four seed books with versioned question-prep metadata and emitted repair/audit reports. The working tree proves the data changed but not who ran the repair or whether the original conflict set was preserved.
- `2026-07-15` - Problems encountered: Same part of speech is insufficient for valid distractors; synonyms, overlapping gloss fragments, duplicate source shapes, and semantically equivalent sibling options can still make multiple answers appear correct.
- `2026-07-15` - Modification points: Added `scripts/repair-seed-vocab-choice-conflicts-strict.mjs` and regenerated CET4, CET6, Kaoyan, and medical seed vocab question preps with stricter semantic filtering. The stricter pass rejects same/contained meaning keys, token containment, Han n-gram overlap, rare Han-character overlap, option-vs-option semantic conflicts, and small-POS sparse pools; medical CT/MRI-like phrase entries are normalized so noun targets do not receive adjective-only distractors.
- `2026-07-15` - Modification points: Enriched CET4, CET6, Kaoyan, and medical seed vocab books with real-exam examples matched from bundled CET/Kaoyan paper assets via `scripts/enrich-seed-vocab-real-exam-examples.mjs`; added `scripts/check-seed-vocab-real-exam-examples.mjs` and `docs/seed-vocab-real-exam-examples-report.json`.
- `2026-07-15` - Problems encountered: A naive sentence-by-entry regex scan timed out. The matcher now builds an inverted index of headword forms and inflections before scanning passage/stem/choice sentence tokens.
- `2026-07-15` - Modification points: Tightened real-exam example enrichment so each wordbook only consumes its own exam-family corpus. This removed cross-family examples such as Kaoyan entries using CET sentences and cleared medical examples until a medical source corpus is available.
- `2026-07-27` - Modification points: Changed real-exam example ranking in `scripts/enrich-seed-vocab-real-exam-examples.mjs` so passage/original paper text strongly outranks stems and choices. Added `scripts/repair-seed-vocab-real-exam-meanings.mjs` and repaired clear exam-context extended meanings for `mobile`, `stock`, `cancel`, `explosive`, and `collapse`.
- `2026-07-27` - Problems encountered: Some correct real-exam sentences used extended senses missing from the seed gloss, such as `mobile phone/apps`, `stock market`/`to stock`, `cancel student debt`, `explosive situation`, and institutional `collapse`. These need seed-level meaning repair rather than UI-only filtering.
- `2026-07-16` - Diagnosis: `KaoYan_3.json` still contains abbreviation-like headwords such as `a.` and `vs.`. These are vocabulary data-quality issues: they can carry legitimate-looking meanings and distractors, but they should not enter normal Study target selection.
- `2026-07-16` - Diagnosis: The `1.0.2+3` APK contains the four raw seed books without precomputed choice distractors. On first-run maintenance, `rebuild_seed_question_preps` ranks same-book/same-POS candidates by `Reverse(seed_text_overlap_score)`, so meanings with the greatest textual overlap are selected first. This reproduces the reported `curb`, `displace`, `insert`, and `oven` choices and is the direct cause of ambiguous answers in that APK/runtime path.
- `2026-07-16` - Modification points: Added `scripts/list-seed-vocab-choice-conflicts.mjs` and focused Node tests. The inventory can audit Git `HEAD`, the current worktree, or reproduce APK runtime ranking directly from an APK without mutating seed data.
- `2026-07-16` - Inventory result: The `1.0.2+3` APK runtime simulation scanned 11,537 entries and found 7,875 book-entry rows (5,655 distinct normalized headwords) with 26,550 conflicting prepared choices. Directly auditing the current strict-repair JSON fields finds zero conflicts, but the current startup maintenance still overwrites those fields through `rebuild_seed_question_preps`; simulating that current runtime path finds 7,907 book-entry rows (5,663 distinct normalized headwords) with 26,634 conflicting choices.
- `2026-07-16` - Modification points: `crates/platform-mobile/src/bridge.rs` now uses maintenance key `seed_vocabulary_maintenance_v5_safe_question_preps`, runs the legacy generator only as a compatibility fallback, and then reapplies the audited bundle `cnChoiceDistractors` / `enChoiceDistractors` for all four seed books. Existing v4 databases therefore replace stale ambiguous `entry_question_preps` on their next Today startup, while fresh imports preserve the same audited candidates after import.
- `2026-07-16` - Modification points: `ensure_seed_vocabulary_available_for_today` no longer returns merely because entries exist. It enters the maintenance-key guarded import path so existing installations receive v5 once; subsequent Today loads still return immediately after reading the completed v5 marker.
- `2026-07-16` - Modification points: `scripts/list-seed-vocab-choice-conflicts.mjs` now models the effective runtime contract: complete bundled precomputed choices are retained, while the old overlap-ranked generator is used only for entries without at least three precomputed CN and EN candidates.
- `2026-07-16` - Result: The effective current runtime inventory now scans all 11,537 entries with zero problem words and zero conflicting choices. The strict audit also reports zero target-CN, target-EN, sibling-option, source-shape, source-count, and POS conflicts in every book.
- `2026-07-16` - Problems encountered: The first parallel full platform test attempt failed at Windows link time with `LNK1104` because the test executable could not be overwritten. No residual test process was present; rerunning the platform suite alone succeeded with all 39 tests, so no product failure was hidden or skipped.

## Mobile Lessons Learned

- A visible glyph bug in one page often means the shared seed payload is dirty. Fixing only the AI passage renderer or Study UI would leave the same bad gloss elsewhere.
- Choice questions need a stable data contract: correct answer text, accepted meanings, rendered option text, and `correct_choice_label` must be generated together.
- Runtime distractor generation from a broad global pool is both slower and less predictable. Precomputing several candidates per entry gives better UX and reduces Today/Study startup work.
- High randomness and “no repeats in one round” are compatible if the wordbook stores a larger candidate list, and the runtime samples a subset while tracking used distractors.
- Root/affix extraction from arbitrary mnemonic text is risky. Shared substrings are not necessarily roots; relevance must be semantic, not only string-based.
- Medical English wordbooks may be phrase-heavy. Root/affix mode needs explicit availability checks and graceful empty-state/fallback behavior.
- Keep source word, source meaning, POS, and book aligned by distractor index so audits can explain and reproduce every option.
- Same POS is only a first-pass filter. Distractor repair must also reject near-synonyms and overlapping Chinese fragments, otherwise users can still see multiple visually correct options.
- Real exam examples should live in the existing `realExamSentence` field because the mobile bridge already imports that field into `entry_examples`; adding a parallel field would not affect study questions without more runtime work.
- Prefer original exam passage sentences over stems and options. Stems/options remain valid fallback examples, but they should not displace a good source-passage sentence for the same word.

## Desktop Follow-Up Notes

Desktop should turn these mobile lessons into tooling, not just parity:

- Add a wordbook QA view that flags polluted meanings, duplicate glosses, missing examples, short/irrelevant root fragments, single-example root cards, and entries with fewer than three CN/EN distractors.
- Let maintainers inspect and regenerate `entry_question_preps` for a wordbook.
- Show source wordbook, POS, meaning group, precomputed CN distractors, precomputed EN distractors, and examples in one entry-detail view.
- Add root/affix review tooling that explains why a candidate was accepted or filtered.

## Route Changes

- `2026-06-25`: Word library scope expanded from seed vocabulary storage to a richer “study entry payload” contract containing precomputed quiz materials.
- `2026-06-25`: Distractor generation route changed from mostly runtime/global-pool selection to seed-maintenance precomputation plus runtime sampling/fallback.
- `2026-06-25`: Root/affix quality moved into the word-library contract instead of treating it as only a Study UI issue.

## Known Pitfalls

- `2026-08-12` - Dictionary `同根` blocks are not always precise enough to union transitively. A single broad block can collapse distinct derivational branches, so known branch boundaries must be applied before union-find grouping and mirrored by runtime projection for existing installs.
- A counted paper occurrence is not always a usable example sentence. Cloze distractors are legitimate frequency occurrences but must not be inserted into the passage as if they were correct answers; brand-token matches such as `J.Crew -> crew` also require audit rather than fabricated examples.
- Do not clean vocabulary only at render time; polluted data must be corrected before it reaches shared payloads.
- Do not let option labels (`A/B/C/D`) become part of meaning text.
- Do not compare user choice by displayed text from one gloss group while storing a different correct-answer gloss group.
- Do not use generic global distractors when entry-specific precomputed distractors are available.
- Do not use root/affix cards from arbitrary shared substrings unless examples actually support the meaning.
- Do not let mobile-only truncation hide long or duplicated glosses; clean and shorten source payloads.
- Do not sync generated seed caches as if they were user data.
- Do not treat a zero-conflict generated report as proof of coverage unless every required book, entry, and seven-option source tuple was audited.
- Do not write raw exam text into seed vocabulary without cleaning Unicode replacement characters; `check-seed-vocab-question-preps` treats `\uFFFD` as a blocking data-quality failure.
- Do not improve coverage by borrowing examples from another exam family; source relevance is part of the wordbook contract, so lower coverage is preferable to misleading provenance.
- Do not keep a real-exam example when the corresponding entry gloss lacks the exam-context sense; either add a short reviewed extension to the seed meaning or choose a different example.
- Do not extend the older corrupted repair script for stricter semantic repair; use the strict repair path and keep punctuation/tokenization rules in ASCII or Unicode escapes to avoid Windows encoding damage.
- Do not count a seed entry as study-ready just because it has a headword and Chinese meaning. Headword token quality must reject dotted abbreviations and single-letter tokens before study payloads are built.
- Do not rank distractors by descending Chinese meaning overlap. That strategy systematically chooses synonyms, contained meanings, and identical answer components, making multiple options defensible.
- Do not use a post-repair zero report to describe what an installed older APK generates. Audit the APK's raw books through the runtime prep algorithm, because installed SQLite question preps may predate current seed JSON fields.
- Before maintenance v5, repaired seed `cnChoiceDistractors` did not survive startup because `rebuild_seed_question_preps` overwrote them. Preserve the v5 ordering and regression test.
- A maintenance fix must change the maintenance key. Reusing v4 would leave existing installations marked complete and preserve their old ambiguous SQLite preps.
- Bundle precomputed candidates must be applied after fallback generation. Applying them before `rebuild_seed_question_preps` silently recreates the original ambiguity bug.

## Verification

- Shared/data (`2026-08-12`): `node --test scripts\kaoyan-english-1-vocab-frequency.test.mjs` passed 8/8, including an explicit `real` versus `realize` branch test. Regeneration completed over 17 papers from 2010-2026 and retained 6,326 appeared vocabulary headwords and 39,377 matched occurrences; scoped diff inspection showed one changed line in `KaoYan_3.json`, and the packaged family index reports `real/reality/really` at 47 combined occurrences.
- `2026-08-10`: The real-exam generator completed with Kaoyan `2794/3728` entries and `12,897` examples; positive-frequency in-scope coverage is `2109/2134`. Its focused Node suite passed 2/2, the existing frequency suite passed 3/3, and the real-exam checker reported zero over-limit, empty, missing-source, target-missing, duplicate, or cross-exam examples. Question-prep, graph-relation, choice-conflict, and exam-import checks also passed.
- Choice conflict inventory: `node --test scripts\list-seed-vocab-choice-conflicts.test.mjs` passed 3 tests on 2026-07-16, covering the four reported ambiguity patterns, false-positive guards, and runtime overlap ranking.
- Current runtime inventory: `node scripts\list-seed-vocab-choice-conflicts.mjs --source=worktree-runtime` scanned 11,537 entries on 2026-07-16 and reported 7,907 problem rows / 26,634 conflicting choices across all four books.
- APK runtime inventory: `node scripts\list-seed-vocab-choice-conflicts.mjs --source=apk-runtime --apk=releases/word-mobile-1.0.2+3.apk` scanned 11,537 entries on 2026-07-16 and reported 7,875 problem rows / 26,550 conflicting choices across all four books.
- Git baseline inventory: `node scripts\list-seed-vocab-choice-conflicts.mjs --source=head` scanned 11,537 entries on 2026-07-16 and reported 5,089 problem rows / 17,446 conflicting choices.
- Current static-field inventory: `node scripts\list-seed-vocab-choice-conflicts.mjs --source=worktree` scanned 11,537 entries on 2026-07-16 and reported zero conflicts.
- Effective runtime inventory after v5 fix: `node scripts\list-seed-vocab-choice-conflicts.mjs --source=worktree-runtime` scanned all 11,537 entries on 2026-07-16 and reported `problemWords=0` and `conflictingChoices=0` for CET4, CET6, Kaoyan, and medical books.
- Strict seed audit after v5 fix: `node scripts\audit-seed-vocab-choice-conflicts.mjs` passed on 2026-07-16 with zero target-CN, target-EN, sibling-option, missing-source, source-shape, and POS conflicts across all 11,537 entries.
- Runtime inventory tests: `node --test scripts\list-seed-vocab-choice-conflicts.test.mjs` passed all 4 tests on 2026-07-16, including preservation of complete bundled safe candidates.
- Platform regression: `cargo test -p word-platform-mobile --lib -- --test-threads=1` passed all 39 tests on 2026-07-16, including `seed_maintenance_replaces_existing_generated_preps_with_bundled_safe_preps`.
- Formatting: `rustfmt --edition 2021 --check crates\platform-mobile\src\bridge.rs` passed on 2026-07-16.
- Compilation: `cargo check -p word-platform-mobile` passed on 2026-07-16 with four pre-existing dead-code warnings in the dirty worktree.
- Final report assertion: parsed `docs/seed-vocab-choice-conflict-inventory-worktree-runtime.json` and `docs/seed-vocab-choice-conflicts-report.json`; confirmed 11,537 runtime entries, zero problem words, zero conflicting choices, and zero non-entry audit counters.

- Seed audit: `node scripts\audit-seed-vocab-choice-conflicts.mjs` passed on 2026-07-15 with zero target-CN, target-EN, internal-option, missing-source, source-shape, or POS conflicts across 2,607 CET4, 2,345 CET6, 3,728 Kaoyan, and 2,857 medical entries.
- Strict repair: `node scripts\repair-seed-vocab-choice-conflicts-strict.mjs` passed on 2026-07-15 and rebuilt 11,537 entries across the four bundled seed books with `incomplete=0`.
- Strict audit: `node scripts\audit-seed-vocab-choice-conflicts.mjs` passed again on 2026-07-15 after the strict repair with zero target-CN, target-EN, internal-option, missing-source, source-shape, or POS conflicts.
- Real exam example audit: `node scripts\check-seed-vocab-real-exam-examples.mjs` passed on 2026-07-15 with source-family isolation. Coverage is CET4 `2066/2607` with `10,744` examples from `cet4`, CET6 `1808/2345` with `8,896` examples from `cet6`, Kaoyan `2697/3728` with `11,894` examples from `kaoyan-english-1/2`, and medical `0/2857` with no examples because no medical exam corpus is bundled. `crossExam=0` for every book.
- Real exam meaning repair: `node scripts\repair-seed-vocab-real-exam-meanings.mjs` passed on 2026-07-27 with 7 reviewed patches and 0 misses. `node scripts\check-seed-vocab-real-exam-examples.mjs`, `node scripts\check-seed-vocab-question-preps.mjs`, `node scripts\check-seed-vocab-graph-relations.mjs`, `node scripts\audit-seed-vocab-choice-conflicts.mjs`, `node scripts\check-exam-paper-import.mjs`, and `cargo check` passed afterward. `cargo check` reported four pre-existing unused-code warnings in `crates/platform-mobile/src/bridge.rs`.
- Mobile: targeted Rust bridge tests pass for same-wordbook/same-POS question prep generation.
- Desktop: pending; first parity should verify desktop can inspect the same payload fields without reimplementing quiz generation.
- Shared/domain:
  - `node -e ...` JSON inspection on 2026-07-16 found `KaoYan_3.json` top-level abbreviation-like entries `resumé`, `vs.`, and `a.`; `resumé` is valid, while `vs.` and `a.` are not normal study targets.
  - `cargo fmt`
  - `cargo test -p word-study-core precomputed_wordbook_distractors`
  - `cargo test -p word-platform-mobile question_preps_are_generated_from_same_wordbook_and_part_of_speech`
  - `cargo check` passed with one existing warning: `today_target_seed_from_plan_value` is unused.

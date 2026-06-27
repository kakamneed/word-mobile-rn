# Feature: Word Library

> Slug: `word-library`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

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

## Mobile Lessons Learned

- A visible glyph bug in one page often means the shared seed payload is dirty. Fixing only the AI passage renderer or Study UI would leave the same bad gloss elsewhere.
- Choice questions need a stable data contract: correct answer text, accepted meanings, rendered option text, and `correct_choice_label` must be generated together.
- Runtime distractor generation from a broad global pool is both slower and less predictable. Precomputing several candidates per entry gives better UX and reduces Today/Study startup work.
- High randomness and “no repeats in one round” are compatible if the wordbook stores a larger candidate list, and the runtime samples a subset while tracking used distractors.
- Root/affix extraction from arbitrary mnemonic text is risky. Shared substrings are not necessarily roots; relevance must be semantic, not only string-based.
- Medical English wordbooks may be phrase-heavy. Root/affix mode needs explicit availability checks and graceful empty-state/fallback behavior.

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

- Do not clean vocabulary only at render time; polluted data must be corrected before it reaches shared payloads.
- Do not let option labels (`A/B/C/D`) become part of meaning text.
- Do not compare user choice by displayed text from one gloss group while storing a different correct-answer gloss group.
- Do not use generic global distractors when entry-specific precomputed distractors are available.
- Do not use root/affix cards from arbitrary shared substrings unless examples actually support the meaning.
- Do not let mobile-only truncation hide long or duplicated glosses; clean and shorten source payloads.
- Do not sync generated seed caches as if they were user data.

## Verification

- Mobile: targeted Rust bridge tests pass for same-wordbook/same-POS question prep generation.
- Desktop: pending; first parity should verify desktop can inspect the same payload fields without reimplementing quiz generation.
- Shared/domain:
  - `cargo fmt`
  - `cargo test -p word-study-core precomputed_wordbook_distractors`
  - `cargo test -p word-platform-mobile question_preps_are_generated_from_same_wordbook_and_part_of_speech`
  - `cargo check` passed with one existing warning: `today_target_seed_from_plan_value` is unused.
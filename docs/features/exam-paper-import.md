# Feature: Exam Paper Import

> Slug: `exam-paper-import`
> Status: `mobile_in_progress`
> Updated: `2026-08-02`

## Product Intent

Provide normalized exam-paper assets for mobile study flows, including kaoyan English, CET-4, and CET-6 papers with question text, choices, answers, explanations, and source metadata where available.

## UX Contract

Users should eventually browse complete paper sets by exam, year, month, and set, and see whether a question has a verified answer or explanation.

## Shared Domain/Data Contract

Imported files use `schemaVersion: 1` JSON under `apps/flutter_mobile/assets/exam-papers/`, with a manifest plus per-exam files. Each paper owns sections; each section owns normalized questions with `id`, `number`, `stem`, `choices`, optional `answer`, optional `explanation`, source path, and optional `answerSource`.

## Flutter Mobile Route

- Owner screen/widget: future exam-paper study surface under Flutter mobile learning/study flows.
- SDK/bridge calls: none yet; current slice ships static assets via Flutter asset bundling.
- Loading/cache/reload behavior: asset manifest should be the first entry point.
- Orientation/gesture constraints: not yet defined.
- First implementation slice: normalize imported public datasets into one JSON shape.
- Current status: importer and static assets are in progress.

## Tauri Desktop Route

- Owner view/window: future desktop study or paper-practice view.
- Shared APIs to reuse: the normalized JSON contract should be reused before introducing platform-specific parsers.
- Desktop-specific layout: desktop can expose denser table/filter views for year, exam type, and answer coverage.
- Mobile assumptions to avoid: do not assume bundled static assets are the only delivery mechanism; desktop may stream or cache larger datasets.
- First parity slice: read manifest, list exams, and open a paper with section/question navigation.
- Current status: not implemented.

## Sync And Storage

Current imported assets are static and offline. Upstream copyright and source stability remain unverified, so syncing user progress should stay separate from imported content provenance.

## AI Or Provider Implications

No model/provider dependency in the current import path. Future answer-quality checks could use AI only as a secondary validator, never as the authoritative source of official answers.

## Implementation Log

- `2026-08-12` - Placeholder-stem audit: a manifest-wide scan found 4,047 `Question N` values among 6,600 normalized questions, but that raw total includes legitimate blank/selection/writing rows. Restricting the audit to kaoyan `text1`-`text5` standard reading found 127 unresolved English I stems and 65 English II stems before repair; CET Section C standard reading also contains roughly 850 placeholders. The defect is therefore corpus-wide and originates in incomplete upstream stem data or failed source merging, not Flutter text rendering.
- `2026-08-12` - Asset modification points: restored the two missing 2008 English I Text 2 stems (`In the first paragraph, the author discusses` and `Which of the following best summarizes the main idea of the text?`) from the contemporaneous full-paper PDF, with a second independent New Oriental transcription used as a cross-check. Added a focused importer regression so these two stems cannot silently regress to `Question 1`/`Question 5`.
- `2026-08-12` - Remaining risk: after the verified two-question repair, 125 English I and 65 English II standard-reading placeholders remain. The existing explanation-overview repair returns zero for the 2008 pair because those explanations contain option summaries and locations but no English stem. Remaining items must be restored from a verifiable full-paper or structured source; choice text and explanation prose are insufficient authority for inventing a stem.
- `2026-08-12` - Verification: `node --test scripts\\import-exam-papers.test.mjs` passed 6/6; `node scripts\\check-exam-paper-import.mjs` passed with 149 papers, 6,600 questions, 6,420 answers, and unchanged capability totals. A post-repair scoped audit reported English I standard-reading unresolved `125` across 56 sections and English II unresolved `65` across 17 sections.

- `2026-07-15` - Modification points: Added normalized import assets and scripts for GitHub-sourced kaoyan English, CET-4, and CET-6 datasets; Flutter asset bundling now includes `apps/flutter_mobile/assets/exam-papers/`.
- `2026-07-15` - Problems encountered: Initial GitHub sources had mixed answer coverage. `XixiGod7/kaoyan-english` had structured questions but not answers; `TsekaLuk/Kaoyan-English1-Papers` only yielded one trusted kaoyan answer block; `Drhm1224/cet6-all-in-one` could not be cloned normally on Windows because the repo contains an invalid NTFS path, so selected answer files had to be downloaded individually.
- `2026-07-15` - Discovery: `https://zhenti.kaoyansou.cn/` registers an English subject on the main API, but `/api/question/subject/2` returns an empty list. The usable public English endpoints are on `https://english.kaoyansou.cn/api`, especially `/paper/filters`, `/paper/list`, and `/paper/{id}`.
- `2026-07-15` - Discovery: `https://english.kaoyansou.cn/api/paper/{id}` returns section-level content with answer and explanation fields inside `contentJson`; `/api/question/answers/{id}` is not a paper-id answer endpoint and returned HTTP 500 for a paper id.
- `2026-07-15` - Modification points: Extended `scripts/import-exam-papers.mjs` with `--kaoyansouEnglishDir`, normalized kaoyansou API snapshots into the same schema, grouped API section records into full papers, added `kaoyan-english-2.json`, regenerated all exam-paper assets, and updated `docs/exam-paper-import-report.json`.
- `2026-07-15` - Problems encountered: kaoyansou uses different payload shapes by section type. Objective reading/cloze questions live under `contentJson.data.tm`, while listening questions live under the top-level `tiMuJson`; writing/translation sections may have passage content but no objective question array. The importer preserves empty-question sections instead of fabricating answers.
- `2026-07-15` - Modification points: Added post-dedupe pruning for papers with `questions > 0` and `answerCount == 0`, regenerated assets from 164 to 137 papers, and added `droppedZeroAnswerPapers` to `docs/exam-paper-import-report.json`.
- `2026-07-15` - Problems encountered: 27 generated papers had no answer-bearing questions. Most were old CET6 GitHub TXT papers from 2016-2018 with no answer source; the rest were kaoyansou CET4/CET6 set variants containing only writing/translation placeholder questions with no objective answers.
- `2026-07-15` - Discovery: The dropped old GitHub CET6 papers were not replaced by kaoyansou records. The kaoyansou snapshot contains CET6 from 2018-12 onward, but not 2016, 2017, or 2018-06, so those dropped IDs remain absent from generated assets.
- `2026-07-15` - Modification points: Added per-question `kind` classification (`objective`, `translation`, `writing`, `subjective`), limited `tiMuJson` parsing to listening sections, filled translation answers from kaoyansou `tm[].daan.zhengque` or `conts[].econt`, and regenerated assets to 149 papers / 6600 questions.
- `2026-07-15` - Problems encountered: Non-listening kaoyansou records also contain top-level `tiMuJson`, but those payloads duplicate explanation/detail records and created thousands of no-choice placeholder questions. Only listening should read `tiMuJson`; other sections should use `contentJson.data.tm`.
- `2026-07-15` - Verification discovery: The current answer-presence contract is `Boolean(question.answer)` and is independent of choice labels. The regenerated assets contain 6,420 answer-bearing questions: 6,159 objective and 261 translation; the remaining 180 writing questions intentionally have no answer. All 149 retained papers have at least one answer, while 15 zero-answer papers are recorded as dropped.
- `2026-07-16` - Modification points: The focused checker now reports answer presence separately from renderer capabilities and rejects only a question explicitly declared auto-gradable when its answer cannot be rendered by its choices. Current coverage is 6,600 browsable/answerable, 5,266 auto-gradable/causal-analyzable, and 6,420 answer-bearing questions.
- `2026-07-16` - Modification points: User imports now accept image or UTF-8 TXT sources in the AI workbench, pass through a reviewable normalized draft, and persist only normalized paper JSON plus provenance/warnings in SQLite. Saved user papers merge into the same offline catalog with distinct `user` origin; raw image bytes are not retained.
- `2026-07-18` - Root-cause correction: Whole-paper deduplication previously favored Kaoyansou because it had answers and explanations, but discarded real stems held by the structured `XixiGod7/kaoyan-english` copy. The importer now merges non-placeholder stems into the answer-rich paper by normalized choice signature before deduplication.
- `2026-07-18` - Asset modification points: Added conservative explanation-overview parsing and `scripts/repair-exam-question-stems.mjs`, then backfilled current Kaoyan assets. The reported 2010 Text 1 now has five real English stems; the repair is idempotent and preserves Kaoyansou answers/explanations.
- `2026-07-18` - Translation asset modification points: Added 21 paragraph-aligned Chinese translations for 2010 Kaoyan English I Text 1-4 to `scripts/exam-paper-paragraph-translations.json`, applied them to the bundled `kaoyan-english-1.json`, and added a production-asset regression test so a future import cannot silently clear or misalign them.

## Mobile Lessons Learned

Use the generated manifest as the mobile entry point, and surface answer coverage explicitly because sources may have question-only, answer-only, or mixed coverage.

## Desktop Follow-Up Notes

Desktop should expose provenance and answer coverage columns so incomplete answer imports are easy to spot during review.

## Route Changes

`kaoyansou` English data should be treated as a distinct API-backed source, not as a direct replacement for GitHub JSON repositories, until content matching and rights posture are reviewed.

- `2026-07-15` - This import capability is now an explicit prerequisite and data source for the PRD-backed `exam-practice-vocab-intelligence` feature. Practice consumers must use normalized question capabilities rather than assuming every answer-bearing record can use a standard-choice renderer.

## Known Pitfalls

- Do not treat `zhenti.kaoyansou.cn/api/question/subject/2` as the English question source; the English content lives on `english.kaoyansou.cn/api`.
- Do not call `/question/answers/{paperId}` for paper answers; use `/paper/{id}` and parse `contentJson`.
- For listening sections, parse the top-level `tiMuJson` field, not only `contentJson`.
- For non-listening sections, do not merge top-level `tiMuJson`; it is not the canonical question list and can create duplicate no-answer questions.
- For translation sections, treat `conts[].zcont` as the prompt/source text and `conts[].econt` as the reference translation when `tm[].daan.zhengque` is absent.
- Do not silently mix question text from one source with answers from another unless year, exam type, section, and question numbering are matched.
- Question-number matching alone is insufficient across sources because one source numbers a Text locally and another uses paper-wide numbers. Require the same paper identity and normalized ordered choices before merging a stem.
- Keep `droppedZeroAnswerPapers` in the report when pruning, so future source refreshes can distinguish intentionally removed zero-answer papers from parser regressions.
- Do not assume every dropped old GitHub paper has a kaoyansou replacement; compare by normalized paper id before reporting coverage.
- Upstream exam-content rights remain unverified even when a public API returns answers and explanations.
- Do not use `hasAnswer` as a proxy for `autoGradable`; translation answers and objective records without a standard-choice shape remain answer-bearing but not automatically scored.
- `2026-08-02` current asset integrity: `load_exam_paper` uses `serde_json::from_str`, so an invalid aggregate resource prevents normal catalog/paper loading even when individual text and choices appear present. Do not treat earlier importer checks as current proof after manual or downstream asset edits; add a strict parse gate before packaging.
- Current assets still contain 192 reading-objective `Question n` placeholders for which the available structured source and explanation overview do not provide a stable full-stem match. They remain an explicit source-coverage gap; do not synthesize stems from choices or answer-analysis prose.
- Kaoyansou reading explanations contain translated stems, choices, and paragraph summaries, but not a dependable sentence-complete translation of every source paragraph. Do not promote those summaries into `paragraphTranslations`. The type-filtered full-corpus audit found 296 relevant passage-bearing sections and 292 still without paragraph translations after the four 2010 reading sections were populated; the separately configured 2010 cloze section is normalized as generic objective data and is outside that count.

## Verification

- Mobile: `node scripts\check-exam-paper-import.mjs` passed with `papers=149`, `questions=6600`, and latest years `cet4:2025`, `cet6:2025`, `kaoyan-english-1:2026`, `kaoyan-english-2:2026`.
- Desktop: not run; desktop route is not implemented.
- Shared/domain: `node --test scripts\import-exam-papers.test.mjs` passed two tests, including kaoyansou objective, listening, translation-answer fallback, and zero-answer pruning behavior. Current kind coverage is `objective=6159`, `translation=261`, `writing=180`; objective and translation questions have no missing answers. `git diff --check -- scripts/import-exam-papers.mjs scripts/import-exam-papers.test.mjs apps/flutter_mobile/assets/exam-papers docs/exam-paper-import-report.json docs/features/exam-paper-import.md` passed.
- Answer-presence audit: manifest and report totals agree at `papers=149`, `questions=6600`, and `answerBearingQuestions=6420`; no retained paper has zero answers, and the report lists 15 dropped zero-answer papers.
- `2026-07-16`: `node --test scripts\check-exam-paper-import.test.mjs scripts\import-exam-papers.test.mjs` passed 4/4; the import review widget and SDK draft tests passed. Manual image/TXT picker smoke was not run because no Android device was connected.
- `2026-07-18`: `node --test scripts\import-exam-papers.test.mjs scripts\check-exam-paper-import.test.mjs` passed 4/4. `node scripts\check-exam-paper-import.mjs` passed with `papers=149`, `questions=6600`, `answers=6420`, answer-bearing objective `6159/6159`, translation `261/261`, and writing `0/180`; latest years remain `cet4:2025`, `cet6:2025`, `kaoyan-english-1:2026`, and `kaoyan-english-2:2026`. `git diff --check` passed with only Git CRLF conversion warnings.
- `2026-07-18`: Stem recovery tests passed 4/4, covering explanation-overview extraction, punctuation-tolerant structured-source merge, and existing importer flows. The focused asset checker passed unchanged totals; a second repair run reported `updatedQuestions=0` before structured-source enrichment, and all five 2010 Text 1 stems were inspected as real English questions.
- `2026-07-18`: Paragraph-translation import tests passed 5/5 and assert 2010 Text 1-4 alignment at `5/5/5/6` paragraphs. The asset checker passed with 149 papers, 6,600 questions, and 6,420 answers; the overlay command updated the five configured 2010 sections (one cloze and four reading sections).
- `2026-08-02`: `Test-Json` reported `false` for `cet4.json`, `cet6.json`, `kaoyan-english-1.json`, and `kaoyan-english-2.json`. This test did not modify assets. The Rust loader's strict parser was inspected and has no tolerant fallback.

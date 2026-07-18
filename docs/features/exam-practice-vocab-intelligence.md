# Feature: Exam Practice And Vocabulary Intelligence

> Slug: `exam-practice-vocab-intelligence`
> Status: `mobile_in_progress`
> Updated: `2026-07-17`

## Product Intent

Make bundled and user-imported exam exercises a first-class Today learning mode, and turn in-context vocabulary behavior into explainable graph relationships and learning priorities.

## UX Contract

- Today shows a compact `单词学习 / 模拟练习` selector beside the page heading.
- Word mode preserves the existing Today experience.
- Practice mode selects exam type, year, paper/set, and section before opening the whole article and its questions in a dedicated reader.
- English words in exercise content can be tapped for meanings and marked/unmarked as unknown without leaving the question.
- Reading uses one continuous mobile window with the passage above all questions. A draggable edge bubble switches between separately remembered passage and question bookmarks; it must not split the screen into two panes.
- Choice selection is provisional. Correctness and explanations remain hidden in the reader; after submission, a section report shows the full status overview and reveals one cleaned answer/explanation only when its question is selected.
- Doing mode uses yellow current-article word/range marks and optional notes without revealing Chinese meanings. Analysis mode immediately projects persisted local-dictionary meanings for yellow words/phrases and may append paragraph translations bundled in the selected paper; it never requests a live translation provider.
- Cloze sections render compact parenthesized blank numbers and compact option rows with labels before option text. Other objective sections reuse the corrected left-label option layout without cloze-only blank formatting.
- Translation keeps only its source passage visible by default, and writing keeps only its prompt visible by default. Each reference answer is revealed independently by an explicit button; neither subjective type is scored in the current version.
- Wrong Words and Reports reuse the page-level learning-content selector. Practice wrong words rank red/yellow annotation evidence, while practice reports select one exam family and chart per-paper objective accuracy overall and by supported question type.
- The AI workbench exposes paper import and analysis tools; normalized imports are reviewed before saving.
- Priority and causal-analysis results show evidence, confidence, and limitations.
- Translation sections initially show only the English source and writing sections hide the bundled reference essay. Each reference translation/essay has an explicit reveal control; neither subjective type exposes submit or scoring UI.
- Wrong Words and Reports expose matching practice modes: practice wrong words rank red/yellow exercise marks, while reports select an exam family and chart whole-paper plus objective-type accuracy.

## Shared Domain/Data Contract

- Normalized catalog: paper, section, question, choice, answer, explanation, optional ordered `paragraphTranslations`, provenance, and question capabilities.
- Question capabilities: `browsable`, `answerable`, `autoGradable`, `causalAnalyzable`.
- Attempt evidence: selected answer, correctness when known, timing, answer changes, token lookup, unknown marks, and stable source IDs.
- Vocabulary evidence preserves original form, normalized form, offsets, sentence/article/question context, and optional canonical dictionary entry.
- Multi-word selections are first-class vocabulary evidence. Curated exam words and phrases are stored as indexed SQLite `entries` plus `entry_meanings`, not page-local fallback strings.
- Same-article relations project into the existing graph `coOccurrence` relation with typed `sameArticle` evidence.
- Priority evidence combines deterministic cross-paper recurrence with attempt, vocabulary, mastery, urgency, recency, and optional AI causal-confidence inputs.
- Practice reports group current auto-graded attempt state by exam family and paper. Supported type series are listening, cloze, reading, and new type; translation and writing are excluded.
- Practice report contract groups non-null `exercise_attempts.is_correct` by exam/paper/section and emits whole-paper totals plus `listening`, `cloze`, `reading`, and `newType` accuracy buckets.

## Flutter Mobile Route

- Owner screen/widget: `TodayShellScreen` owns mode selection; a dedicated exam catalog/practice route owns exercise selection and reading; `AiScreen` owns import and analysis tools; `WrongWordGraphScreen` consumes graph evidence.
- SDK/bridge calls: new catalog, attempt, vocabulary-event, import, relation, and priority methods built on Rust/app-core contracts.
- Loading/cache/reload behavior: word mode first paint remains independent of large exam assets; catalog loads lazily; attempt and mark writes are local-first; Today/graph/AI caches invalidate after relevant commits.
- Orientation/gesture constraints: portrait-first practice reader, compact segmented selector, tappable token spans that do not conflict with scrolling/text selection, and keyboard-safe import review.
- First implementation slice: supported standard-choice bundled paper from Today selection through persisted result and resume.
- Current status: planning complete; normalized assets and storage-only exercise vocabulary tables are partial prerequisites.

## Tauri Desktop Route

- Owner view/window: exam catalog workspace, split reader/question pane, and AI evidence inspector.
- Shared APIs to reuse: the same Rust catalog, attempt, vocabulary-event, graph relation, import, and priority contracts.
- Desktop-specific layout: dense filter table/tree, side-by-side passage and question, hover lookup plus click-to-mark, and an evidence side panel.
- Mobile assumptions to avoid: do not copy the compact Today segmented control, bottom composer constraints, or forced single-column reader.
- First parity slice: open the shared catalog, complete/resume one standard-choice attempt, and inspect persisted vocabulary evidence.
- Current status: not implemented; contract parity is required during mobile work.

## Sync And Storage

SQLite remains the offline source of truth for imported catalog entries, attempts, occurrences, marks, relations, and analysis metadata. Bundled content is immutable; user imports use distinct provenance and stable IDs. Raw source images stay local by default. Normalized user-owned attempts and marks may later enter the existing sync outbox after conflict and payload rules are specified.

## AI Or Provider Implications

Cross-paper occurrence counts are deterministic. AI may help parse imports and interpret structured wrong-answer evidence, but it must not invent official answers, occurrence counts, dictionary meanings, or paragraph translations. Reading translations are immutable paper data. Causal analysis returns candidates, confidence, evidence, limitations, provider/model metadata, and a recoverable failure state.

## Implementation Log

- `2026-07-15` - Modification points: Created PRD-backed Vico tracking under `2026-07-15-exam-practice-vocab-intelligence`; fixed the Today mode, practice reader, import review, same-article graph, and explainable priority routes; reused existing exam import, AI workbench, and graph capabilities rather than creating parallel contracts.
- `2026-07-15` - Verification discovery: The planning-time asset snapshot contained 137 papers and 11,261 questions. It had 6,159 answer-bearing questions, while 5,266 satisfied the strict standard-choice answer/choice-label contract. Answer-bearing matching or word-bank questions need dedicated renderers.
- `2026-07-15` - Verification correction: The regenerated current snapshot uses `Boolean(question.answer)` for `hasAnswer`, not choice-label compatibility. It contains 149 papers, 6,600 questions, 6,420 answer-bearing questions, and 180 questions without answers; all retained papers have at least one answer. Kind consistency is exact: objective `6159/6159` answered, translation `261/261` answered, writing `0/180` answered.
- `2026-07-15` - Problems encountered: The installed Vico `bootstrap_vico_slug.py` wrapper points to a missing `skills/runtime/cli` owner path, so the PRD, plan, and index were created manually from the official templates. This does not affect product execution, but future Vico automation should not be assumed operational until repaired.
- `2026-07-15` - Baseline reconciliation: Storage schema version 16 adds `exercise_articles`, `exercise_vocab_occurrences`, and `exercise_vocab_relations`, with repository APIs for article upsert/read, occurrence upsert/list/mark, and typed relation upsert/list.
- `2026-07-15` - Problems encountered: This is currently a storage-only slice. No app-core facade, Flutter bridge, practice reader, catalog, attempt contract, or sync-outbox integration is present for these tables yet, so platform parity must not be claimed.
- `2026-07-15` - Modification points: Added full offline matching from bundled CET4, CET6, Kaoyan English 1, and Kaoyan English 2 assets into seed-vocabulary `realExamSentence.sentences`. The matching script scans passage, stem, and option text, keeps provenance for exam/paper/section/question/choice, caps each word at eight examples, and emits `docs/seed-vocab-real-exam-examples-report.json`.
- `2026-07-15` - Verification discovery: The current exam corpus produced 40,190 candidate English sentences. Matched seed entries were CET4 `2470/2607`, CET6 `2085/2345`, Kaoyan `3315/3728`, and medical `141/2857`; medical coverage is low because the source corpus is general CET/Kaoyan exam content, not medical exams.
- `2026-07-15` - Contract correction: Real-exam examples are now isolated by exam family instead of maximizing global coverage. CET4 uses only `cet4`, CET6 uses only `cet6`, Kaoyan uses only `kaoyan-english-1/2`, and medical receives no examples until a medical exam-paper corpus exists.
- `2026-07-16` - Modification points: Added the shared Rust exam catalog, explicit `browsable/answerable/autoGradable/causalAnalyzable` capabilities, lazy Flutter SDK loading, the Today `单词学习 / 模拟练习` selector, cascading exam/year/paper/section/question selection, a dedicated practice reader, and resumable local attempts. Standard-choice correctness is shown only when the declared answer matches a rendered choice.
- `2026-07-16` - Modification points: Added shared English tokenization, in-place meaning lookup, matched/unmatched occurrence persistence, unknown-word mark/unmark, UTF-8 byte offsets, attempt vocabulary evidence, and stable same-article relation refresh. Passage, stem, and choices share one article identity while offset namespaces prevent collisions.
- `2026-07-16` - Modification points: Projected exercise `same_article` evidence into existing graph `coOccurrence` edges and added visible source-paper/article/question details without replacing synonym, similar-form, root-family, or existing AI/study relations.
- `2026-07-16` - Modification points: Added AI-workbench `试卷导入` and `试卷分析` tools, image/TXT source selection, AI-normalized review drafts, structured correction of identity/passage/question/choices/answer fields, local `user_exam_papers` persistence, and catalog merging with `origin=user`. Raw imported image bytes are not persisted or synced.
- `2026-07-16` - Modification points: Added deterministic cross-paper word recurrence, article/occurrence separation, unknown/wrong/mastery factors, an explainable priority sheet, and evidence-gated wrong-question AI analysis. Analysis results are keyed by unchanged evidence, retain provider/model/version metadata, and degrade to structured `provider_failure` results while local ranking remains available.
- `2026-07-16` - Cross-platform contract: Android JNI and iOS C/Swift bridge entries now expose catalog, attempt, token, import, priority, and causal-analysis operations. iOS asset discovery probes Flutter's App.framework resource path; Tauri desktop consumption remains unimplemented.
- `2026-07-16` - UX correction and modification points: Replaced the proposed dual-pane reader with one continuous passage-plus-questions `ListView`. `ExamReadingPositionMemory` stores passage/question offsets, and the draggable edge bubble saves the active offset before switching to the other bookmark. All question cards render in the same document and retain their state.
- `2026-07-16` - Modification points: Answer choices now persist as `in_progress` drafts with `isCorrect=null`; one fixed bottom submit command grades and persists the whole section before correctness, explanations, causal-analysis actions, and summary become visible.
- `2026-07-16` - Modification points: Schema 20 adds typed `exercise_annotations` for scoped yellow ranges and optional notes. Shared annotation state separates current-article meanings from prior-article words, and Android/iOS bridges expose `getExamAnnotationState` and `saveExamAnnotation`.
- `2026-07-16` - Modification points: `ExamInteractiveText` now propagates word-mark changes to the article owner so every normalized occurrence across passage, stems, and choices updates together. Yellow overrides pale purple; inline meanings are projected only for current yellow words in analysis mode. The deterministic summary reports score, unanswered count, current yellow words, and yellow/purple intersections.
- `2026-07-16` - Runtime bug diagnosis and modification points: `load_exam_paper` previously reparsed all four bundled exam documents (about 19 MiB total) for every paper selection. It now resolves the requested exam file from the manifest and parses only that document; Flutter also caches opened papers for instant repeat selection.
- `2026-07-16` - Runtime bug diagnosis and modification points: Definition-only `inspectExamWord` calls previously sent `userMark=none`, which cleared the persisted occurrence before the mark button read it. Lookup now omits the field, the bridge preserves the existing mark when no mutation is requested, and explicit mark/unmark updates every persisted occurrence of the normalized word in the active article.
- `2026-07-16` - Dictionary modification points: Lookup now checks `word`, `lemma`, common English inflection candidates, and schema-21 `entry_aliases`. Seed `relWord` derivations are imported once into SQLite, including the supplied `paradoxical` example, without parsing large JSON assets on each tap.
- `2026-07-16` - Reader presentation modification points: Display-only passage normalization removes source sentence counters, preserves paragraph breaks, adds two-em first-line indentation, and removes malformed terminal `.?`. Tappable words have no underline. The edge switch is now a high-contrast `文/题` status bubble with an accessible action label.
- `2026-07-16` - Interaction correction: Doing mode retains the definition bottom sheet and article-level mark control; analysis mode taps toggle the yellow mark directly while meanings remain inline. The article-level mark set, rather than an individual occurrence, determines whether the next action marks or unmarks.
- `2026-07-16` - Reader navigation correction: The continuous reader now eagerly lays out one selected article so the first-question anchor is measurable on the initial frame. Manual scrolling maintains separate passage/question bookmarks, while animated switching freezes bookmark writes to avoid replacing the saved passage position mid-jump.
- `2026-07-16` - Reader presentation modification points: Removed the reserved right gutter so passage and choices use the full content width. The draggable control is now a 44 px, 62%-opacity circle positioned 20 px beyond the right edge; it shows the target action (`↓题` from passage, `↑文` from questions) and may overlay content.
- `2026-07-16` - Today selection correction: Removed the `具体题目` dropdown and its question-selection state. Selecting `题型 / 篇章` opens the complete article and all of its questions.
- `2026-07-16` - Loading-state correction and diagnosis: `ExamPracticeHome` previously used a bare `CircularProgressIndicator` while its catalog loaded. It now uses the same `CrocodileLoadingAnimation` as the Today word-mode startup. The catalog remains lazy and only starts after the user switches to simulation practice; it is not the source of the word-mode startup delay.
- `2026-07-16` - Question-renderer correction: Cloze passages now distinguish source sentence counters from answer blanks and render remaining blanks as smaller `(n)` markers. Cloze cards suppress redundant `Question n` stems, while all objective option rows use tighter vertical density and place `A/B/C/D` before the option text.
- `2026-07-16` - Doing-mode correction: Token taps toggle the article-level yellow mark directly and never open a definition sheet. Long-press ranges persist exact text/offsets; multi-word ranges are also inspected as whole phrase entries, but meanings remain hidden until analysis mode.
- `2026-07-16` - Dictionary modification points: Added idempotent SQLite supplemental source `word-mobile/exam-phrases` with high-frequency phrases (`so long as`, `for fear that`, `in case that`, `as far as`, and others) and screenshot-reported missing words including `upon`, `econometric`, `mischievous`, `peculiar`, and `interpretation`. Existing higher-frequency entries retain lookup priority.
- `2026-07-16` - Report route correction: Submission opens `ExamSectionReportScreen`. It presents score and every question status first, reveals only the selected question's answer and sanitized explanation, removes raw HTML/replacement characters, and replaces per-question AI buttons with one section-level action over all wrong attempts and marked vocabulary.
- `2026-07-16` - Shared AI contract: Added additive `analyzeExamReadingContext` routes to Flutter, Android JNI/Java/Kotlin, iOS C/Swift, and Rust. The bounded request receives passage plus full exercise context and marked words, returning ordered paragraph translations and one context meaning per word/phrase; provider failure leaves local practice usable.
- `2026-07-17` - Runtime bug diagnosis and route correction: Analysis-mode inline meanings were incorrectly gated on a completed `ExamReadingContextAnalysis`, so persisted yellow marks stayed highlighted without Chinese even when SQLite already held a definition. Both passage and question/choice renderers now gate meaning display only on analysis mode, and annotation hydration fills legacy empty `meaning_note` values from the local indexed dictionary.
- `2026-07-17` - Offline translation contract: Added ordered `paragraphTranslations` to the normalized Rust/Dart section schema. The importer preserves Kaoyansou `zcont` paragraph pairs, a checked-in override file supplies the reported 2010 cloze translations, and the generated paper JSON contains four translations aligned to its four source paragraphs. Sections without bundled translations show a disabled `暂无内置译文` action.
- `2026-07-17` - Provider route removal: Removed `analyzeExamReadingContext` from Dart SDK, Rust, Android JNI/Java/Kotlin, and iOS C/Swift. The translation button now only toggles local asset content, eliminating the reported invalid-AI-response path; evidence-backed wrong-answer causal analysis remains a separate opt-in AI feature.
- `2026-07-17` - Subjective renderer correction: Translation and writing no longer reuse objective question cards or the section submit bar. Translation keeps the English source visible and hides each stored reference translation behind its own reveal button; writing hides the stored reference essay until requested. Both state clearly that scoring is unavailable.
- `2026-07-17` - Cross-page practice views: Added a shared compact content selector to Wrong Words and Reports. Practice wrong words use only persisted exercise red/yellow counts; practice reports call one native aggregate and show exam-family, whole-paper, listening, cloze, reading, and new-type trends.
- `2026-07-17` - Cross-platform report contract: Added `getExamPracticeReport` to Dart, Rust, Android JNI/Java/Kotlin, and iOS C/Swift. Rust owns attempt grouping and section classification so Flutter and future desktop consumers cannot diverge on accuracy truth.

## Mobile Lessons Learned

- Do not load the full exam catalog on the critical Today word-mode path.
- Do not force exam passages and choices into the flashcard-oriented `StudyScreen`.
- An answer-bearing question is not necessarily auto-gradable by the standard-choice renderer; capability validation must consider the interaction shape.
- Token offset, sentence, question, article, and attempt identity must be captured when the interaction occurs; they cannot be reconstructed reliably during later AI analysis.
- Keep storage relation names (`same_article`, `same_paragraph`, `same_sentence`, `same_exercise_item`, `canonical_entry`) explicit when projecting them into graph-level `coOccurrence` evidence.
- When reusing exam-paper content as dictionary examples, keep the original provenance object beside the rendered source label; downstream practice and graph features need stable paper/question IDs, not only a human-readable string.
- Keep wordbook-to-exam-family filtering in the enrichment script and checker, not just in UI display, so downstream examples cannot silently cross from CET into Kaoyan or medical books.
- Rust tokenizer offsets are UTF-8 byte offsets, while Dart string slicing uses UTF-16 code units. Persist Rust offsets as evidence, but locate rendered token text sequentially in Dart instead of slicing Chinese-prefixed text with byte offsets.
- Same-article evidence requires one article identity across passage, stem, and choices. Use deterministic offset namespaces for subregions rather than assigning each UI fragment a different article ID.
- A computed evidence detail is not useful if the graph panel only renders relation counts; source details must be part of the visible selected-node UI.
- A mobile reading-position switch is a bookmark problem, not a layout split. Keep one scroll controller and two saved offsets; a two-pane implementation changes the requested reading model and reduces the usable phone viewport.
- Persist draft answer changes without correctness. Otherwise a resumable attempt leaks grading before the explicit article-level submit action.
- Treat section-level reporting as the grading disclosure boundary. Do not render answer correctness or source explanation HTML back into every question card after submit.
- A dictionary mark and its displayed meaning are separate states. Recover old empty mark notes from the current local dictionary during annotation-state hydration, and never make local meaning visibility depend on provider state.

## Desktop Follow-Up Notes

Desktop should use the shared Rust evidence contracts and avoid re-parsing normalized assets in TypeScript. Its first parity target is a split-pane standard-choice practice flow with vocabulary evidence inspection, not a literal copy of the mobile Today selector.

Desktop should read `paragraphTranslations` from the shared section contract and must not add a live translation call. It may present translations and report evidence in a persistent side panel while preserving overall-status-first disclosure, one selected answer detail, and one section-level causal-analysis action.

## Route Changes

- `2026-07-15` - Exam-paper import becomes a prerequisite/data-source feature for the broader exam-practice capability.
- `2026-07-15` - Same-article exercise relationships reuse graph `coOccurrence` with typed evidence instead of introducing a separate graph implementation.
- `2026-07-15` - AI frequency analysis is deterministic-first; AI is reserved for import assistance and evidence-backed causal interpretation.
- `2026-07-16` - Mobile reading route changed from a proposed two-pane workspace to one continuous reader with passage/question bookmarks controlled by the edge bubble.
- `2026-07-17` - Reading-context translation changed from an additive AI bridge route to immutable per-section paper data; local dictionary projection remains the only analysis-mode meaning path.

## Known Pitfalls

- Do not mark missing-answer or unsupported-renderer questions as user errors.
- Do not infer causation from a single wrong answer without lookup/mark/answer-change and question evidence.
- Do not let AI-provided prose replace stable evidence fields and confidence.
- Do not sync raw imported images by default.
- Upstream exam-content rights remain unverified and must remain visible as a release risk.
- The current Vico bootstrap wrapper is broken in this installation; use verified manual templates or repair the owner path before relying on it.
- Do not expose storage-core repositories directly to one platform; add shared app-core contracts before Flutter or Tauri consumes exercise vocabulary evidence.
- Do not use one regex per vocabulary entry against every sentence; the first implementation timed out. Build a `form -> entries` inverted index and scan token/ngram forms from each sentence instead.
- Public exam text may contain Unicode replacement characters. Strip `\uFFFD` before writing examples into seed vocabulary, otherwise the existing seed-prep integrity check fails.
- Do not report global real-exam coverage as quality if the matches cross exam families; per-book source isolation is a correctness gate.
- Do not run the `.cmd` release wrapper from another `cmd /c` when it can spin without launching Dart/Gradle. The PowerShell deploy script uses argument arrays, validates a fresh APK hash/timestamp, and is the verified release path.
- Release/device smoke is incomplete when ADB has no device. A freshly built APK is build evidence, not proof of install, restart persistence, or on-device image/TXT import.
- The bundled corpus rights posture remains unresolved. Public availability and provenance do not establish redistribution rights.
- Flutter text selection reports UTF-16 positions while word-token evidence uses Rust UTF-8 byte offsets. Range annotations therefore use their own scoped display-offset contract and must not be merged numerically with token offsets without conversion.
- Automated widget checks cover bookmark state and bubble switching, but real-device long-press selection, range-note affordance, bubble dragging, and scroll-position feel still require a portrait-phone manual pass.
- Bundled CET/Kaoyan/medical wordbooks are exam lists, not a complete general dictionary. They omit basic words such as `habit`, `researcher`, and `mindless`; inflection and `relWord` aliases improve coverage but a separately licensed general English-Chinese dictionary is still required before claiming every article token has a definition.
- Do not encode lookup as `userMark=none`. Absence means read-only lookup; explicit `none` means unmark and is a state mutation.
- Do not parse every exam JSON to open one bundled paper. Resolve the manifest file by exam and cache already opened paper payloads on the client surface.
- A lazy scrolling list cannot provide an off-screen question `GlobalKey` before that child is built. For the article-sized reader, eagerly lay out the continuous column and calculate the first-question reveal offset after the first frame; reconsider virtualization only if sections grow beyond the current article-sized scope.
- Do not use a separate generic spinner for simulation-practice loading. The Today-level mode switch should preserve the same branded loading language even when the underlying call is `getExamCatalog`.
- Upstream explanations contain HTML, entities, and occasional Unicode replacement characters. Sanitize them at the report presentation boundary; never display the raw source string.
- Current assets label cloze questions as generic `objective`. Mobile detects cloze presentation from the section title (`完型` / `完形`); a future normalizer should add an explicit interaction subtype before desktop depends on this heuristic.
- Gradle release Java/Kotlin task verification did not start in the managed Windows sandbox: wrapper/standalone/no-daemon attempts all exited while forking the single-use JVM, and the project requests a 3 GiB heap. Rust compilation and a 7-route static contract check passed, but a release build remains the authoritative native packaging check.
- Do not call an AI provider from the paragraph-translation button. A missing `paragraphTranslations` array is an asset-coverage gap and must produce a disabled local action, not generation or a provider error.
- Bundled translations must stay index-aligned with non-empty source paragraphs. The asset checker rejects partial or mismatched arrays to prevent a translation from appearing below the wrong paragraph.
- All 180 currently bundled writing sections store non-empty reference/model content in `section.passage` but expose only the generic `Question 1` stem and generic section instructions. Coverage is CET4 47 sections (2018-12 through 2025-06), CET6 48 sections (2018-12 through 2025-06), Kaoyan English I 51 sections (1998-2026), and Kaoyan English II 34 sections (2010-2026). The mobile renderer prevents answer leakage, but source/import data must be backfilled before any bundled writing task can show a complete prompt.
- `CET6 2018-12 Set 3` additionally contains the malformed instruction `Part 1 Writing 写���`; normalization must remove replacement characters when the writing prompts are repaired.
- Do not infer subjective correctness by string comparison. Translation and writing remain reveal-only until a reviewed scoring contract exists.
- Current Kaoyansou writing assets often contain a reference essay in `passage` but only a generic `Question 1` prompt. Mobile now prevents answer leakage, but full writing-prompt coverage remains an upstream normalization gap and must not be mistaken for a renderer failure.
- Do not grade translation or writing by answer-string equality. Their stored reference content is disclosure-only until a dedicated subjective rubric exists.

## Verification

- Current mobile/assets: `node scripts\check-exam-paper-import.mjs` passed on 2026-07-15 with `papers=149`, `questions=6600`, and latest years `cet4:2025`, `cet6:2025`, `kaoyan-english-1:2026`, `kaoyan-english-2:2026`; `node --test scripts\import-exam-papers.test.mjs` passed 2/2 importer tests.
- Current answer audit: an independent manifest/report audit passed with `answerBearingQuestions=6420`, `withoutAnswer=180`, `zeroAnswerPapers=0`, and `droppedZeroAnswerPapers=15`. A kind audit found no violations: all objective and translation questions have answers and all writing questions are intentionally unanswered.
- Current storage/domain: `cargo test -p word-storage-core exercise_article_occurrences_marks_and_relations_round_trip` passed on 2026-07-15.
- Current seed/example enrichment: `node scripts\enrich-seed-vocab-real-exam-examples.mjs` completed on 2026-07-15 and wrote `maxExamplesPerWord=8` real-exam examples from the 40,190-sentence corpus after source-family filtering. `node scripts\check-seed-vocab-real-exam-examples.mjs` passed with `crossExam=0`, no empty examples, no missing provenance, no target-missing sentences, and no duplicate sentences.
- Current supporting checks: `node scripts\check-exam-paper-import.mjs`, `node scripts\check-seed-vocab-question-preps.mjs`, `node scripts\check-seed-vocab-graph-relations.mjs`, and `cargo check` passed on 2026-07-15; `cargo check` still reports the pre-existing unused `today_target_seed_from_plan_value` warning.
- Planning snapshot/mobile assets: the earlier `node scripts/check-exam-paper-import.mjs` result was `papers=137`, `questions=11261`, latest `cet4:2025`, `cet6:2025`, `kaoyan-english-1:2026`, and `kaoyan-english-2:2026`; use the current result above for present asset counts.
- Planning snapshot/data audit: a read-only Node audit found 6,159 answer-bearing questions and 5,266 strict standard-choice auto-gradable questions across the four generated files at that earlier snapshot.
- Vico automation: `python C:\Users\clf20\.codex\skills\vico-plan\scripts\bootstrap_vico_slug.py --help` failed because the wrapper's owner script path does not exist; manual artifact creation was used.
- `2026-07-16` shared Rust: `cargo test -p word-app-core exam_practice_service` passed 5 tests; `cargo test -p word-storage-core exercise_vocab_repo` passed 3 tests; `cargo test -p word-platform-mobile exam_` passed 2 import/provider-failure tests; `cargo check -p word-platform-mobile` passed with the pre-existing unused Today helper warning.
- `2026-07-16` resource contract: `node scripts\check-exam-paper-import.mjs` passed with 149 papers, 6,600 questions, 6,420 answers, 5,266 auto-gradable/causal-analyzable questions; combined Node importer/capability tests passed 4/4.
- `2026-07-16` Flutter: focused practice/import/graph suites passed 12 tests; Today and existing AI regression suites passed 11 tests; targeted `flutter analyze --no-pub` passed with no issues.
- `2026-07-16` single-window annotation slice: `flutter test test\exam_practice_screen_test.dart test\exam_practice_client_test.dart --no-pub` passed 13 tests; `flutter analyze --no-pub` passed with no issues. Coverage includes provisional/submitted presentation, supported-only grading, yellow-over-purple precedence, analysis-only meanings, single-reader dual-bookmark memory, edge-bubble switching, and SDK annotation-state decoding.
- `2026-07-16` shared persistence/native slice: `cargo test -p word-storage-core range_note_and_cross_article_word_state_round_trip` passed; `cargo check -p word-platform-mobile` passed with only the pre-existing unused Today helper warning. The earlier full `cargo test -p word-platform-mobile --lib` also passed 35 tests after annotation bridge implementation.
- `2026-07-16` screenshot regression slice: Flutter practice/client tests passed 17 focused tests, including passage counter removal/indentation, visible `文/题` bubble state, read-only lookup request omission, definition bottom sheet, no underline, and immediate yellow highlight after marking. `flutter analyze --no-pub` passed with no issues.
- `2026-07-16` Rust regression slice: `cargo test -p word-app-core exam_practice_service` passed 6 tests; `cargo test -p word-storage-core exercise_vocab_repo` passed 5 tests; the focused platform test for inflections and seed aliases passed; `cargo check -p word-platform-mobile` passed with only the pre-existing unused Today helper warning.
- `2026-07-16` loading-state regression slice: `D:\flutter\flutter\bin\flutter.bat test test\exam_practice_screen_test.dart --no-pub` passed 11/11, including `practice catalog loading uses the crocodile loader`. Device catalog-load timing was not measured in this turn.
- `2026-07-16` device status: `adb devices -l` returned no connected device, so the corrected release APK was not installed and the exact screenshot flow remains pending on-device verification.
- `2026-07-16` release: the Supabase-aware PowerShell deploy workflow regenerated `app-release.apk` at 112,821,711 bytes (107.6 MiB), verified a new hash/timestamp and packaged Rust JNI library, then failed only at `adb install` because no device/emulator was connected. The previous APK was 104,464,187 bytes; the explicit increase is 8,357,524 bytes (7.97 MiB, 8%).
- `2026-07-16` reader navigation regression: `flutter test test\exam_practice_screen_test.dart test\exam_practice_client_test.dart --no-pub` passed 19 tests, including the half-hidden translucent target bubble and an initially available question anchor after a 5,000 px passage. `flutter analyze --no-pub` passed with no issues.
- `2026-07-16` renderer/report/context regression: focused Flutter practice/client suites passed 25 tests at phone-width coverage, including cloze formatting, compact left-label options, doing-mode no-definition marking, explanation sanitization, section report disclosure, contextual meaning parsing, and paragraph translations. `flutter analyze --no-pub` passed with no issues.
- `2026-07-16` native/context regression: `cargo test -p word-platform-mobile exam_ --lib` passed 6 tests, including idempotent phrase/word supplementation, provider normalization, and oversized-context rejection; `cargo check -p word-platform-mobile` passed with pre-existing unused Today helper warnings. Static verification found `analyzeExamReadingContext` in all 7 Flutter/Android/iOS routing surfaces. Android Gradle task execution remained unverified for the JVM-fork reason above.
- `2026-07-17` offline meaning/translation regression: focused Flutter client/reader suites passed 26 tests, including immediate persisted-yellow meaning display, bundled translation display, and an assertion that no `analyzeExamReadingContext` call occurs; targeted Flutter analysis reported no issues. Node importer tests passed 2/2; the asset checker passed 149 papers, 6,600 questions, 6,420 answers, and verified paragraph alignment; the overlay command was idempotent with `updatedSections=0` on its second run. `cargo test -p word-platform-mobile exam_phrase_entries_are_idempotent_and_queryable --lib` passed the legacy-empty-meaning fallback assertion, and `cargo check -p word-platform-mobile` passed with only pre-existing unused helper warnings.
- `2026-07-17` practice cross-page/subjective regression: five focused Flutter suites passed 31 tests covering hidden subjective answers, per-answer reveal, red/yellow sorting, exam report decoding, exam-family selection, and whole-paper/type charts; targeted analysis of 10 files reported no issues. Rust section classification test and `cargo check -p word-platform-mobile` passed with only pre-existing unused helper warnings; `cargo fmt --all -- --check` and `git diff --check` passed. APK/device verification was not run.
- `2026-07-17` writing-prompt asset audit: a read-only structured scan of all four normalized exam JSON files found 180/180 writing sections with only `Question 1`, no meaningful writing stem, generic instructions, and non-empty passage/reference content. Counts were CET4 47, CET6 48, Kaoyan English I 51, and Kaoyan English II 34; meaningful prompt coverage was zero.
- Device/manual: not run because ADB reported no devices. On-device offline restart/resume, image/TXT picker smoke, provider-backed causal analysis, and install/launch remain unverified.
- Desktop: not run; Tauri route remains pending, but shared contracts and mobile lessons are recorded for consumption.

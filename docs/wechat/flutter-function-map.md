# Flutter Function Map for Mini Program Parity

Date: 2026-05-21

This document is the source map for rebuilding the WeChat mini program from the Flutter implementation. It separates Flutter UI orchestration from the Rust business source. The mini program should copy contracts and flow from these sources, not invent mock English-only behavior.

## Non-Negotiable Findings

- Choice answers submit the option label (`A`, `B`, `C`, `D`), not the option text.
- `exampleToCnChoice` and `exampleToCnChoiceNoTranslation` use an English example/prompt and Chinese meaning choices.
- `newWord` ignores Croc BTI personalized question weights and runs fixed type rounds: example-to-Chinese choice, English-to-Chinese choice, Chinese-to-English choice, English-to-Chinese input.
- Flutter side icons are Material icons, not image assets: study side actions use `lightbulb_outline`, `mode_comment_outlined`, `visibility_outlined`, `gavel_outlined`, `delete_outline`; bottom tabs use `today`, `tune`, `menu_book`, `query_stats`, `auto_awesome`.
- Croc BTI character images are real assets under `apps/flutter_mobile/assets/croc_bti/*`; the mini program already has corresponding copied files under `apps/wechat_miniprogram/src/assets/croc_bti/*`.

## Flutter App Shell

### `apps/flutter_mobile/lib/main.dart`

- `main()`: initializes Flutter app entry and wires `WordApp`.
- `WordApp`: owns top-level theme and `AppState` composition.

### `features/mobile_root_shell.dart`

- `RootRouteInventoryEntry`: enum for route inventory and drawer destinations.
- `MobileRootShell`: root screen with tab navigation, account drawer, and study overlay.
- `_MobileRootShellState._switchToMain(index)`: changes bottom tab and resets study overlay.
- `_openStudy(mode, hint)`: switches to dedicated study page, sets `_studyMode`, `_studyResumeHint`, and bumps study reload seed.
- `_handleStudyClosed()`: exits study and refreshes Today, Wrong Words, Reports, AI, and Study seeds.
- `_openAccountDrawer()`, `_openLeaderboard()`, `_openSettings()`, `_openOnboarding()`, `_openCrocBti()`: drawer-driven routes.
- `build()`: constructs `IndexedStack` with Today, Plan, Wrong Words, Reports, AI, and Study; hides bottom navigation while studying.
- `NavigationBar` destinations: Today/Plan/Wrong/Reports/AI with Material icon pairs. Mini program should use equivalent icon assets or a consistent icon library, not text placeholders.
- `_AccountAvatarButton`: draws account avatar using profile image or fallback initials.

**Joint behavior:** Today opens Study through `_openStudy`; Study returns through `_handleStudyClosed`; this reloads all dependent tabs so progress, reports, wrong words, and AI context reflect latest local state.

## Today Home

### `features/today_shell_screen.dart`

- `TodayShellScreen`: home page that loads today bundle, plan, reward, sync state, announcement, AI context, and auth state.
- `_refreshHomeBundle(showFullLoading)`: fetches `TodayClient.getTodayHomeState`, `PlanClient.getActivePlan`, reward state, resume hint, and optional cloud/sync data; caches page data.
- `_optionalLoad<T>()`: wraps optional async loads so secondary widgets do not break the home page.
- `_openPlan()`, `_openStudy(mode, hint)`, `_openReports()`, `_openWrongWords()`, `_openAi()`, `_openAccount()`: page action routing.
- `_applyPlanToToday()`: applies saved plan to today via plan client and reloads Today.
- `_activePlanFromToday(today)`: extracts embedded active plan from Today state.
- `_snapshotOrPlanFallback(snapshot, activePlan)`: uses Today snapshot if usable; otherwise derives targets from active plan.
- `_buildTaskItems(snapshot, activePlan)`: creates rows for new word, review, mixed test, wrong word, and root affix tasks.
- `_displayTarget(snapshot, mode, activePlan)`: chooses snapshot target first, then plan count fallback.
- `_calculateCompletion(snapshot, activePlan)`: computes completion percent from completed/target values, capped per task.
- `_PrimaryActionCard`: purple hero card with date, active task name, progress bar, completion, remaining units, and CTA.
- `_TaskBreakdownCard`: task list card with edit action and row taps into study modes.
- `_TaskProgressRow`: row with colored dot, title, subtitle, count, chevron, and progress bar.
- `_RewardSlotMachineCard`, `_RewardLever`, `_RewardImagePanel`: reward draw and save UI.

**Joint behavior:** `TodayHomeState.todaySnapshot` plus active plan controls displayed task counts. Study completion must update the underlying snapshot/reports, then Today reload recomputes completion. Do not increment Today completion per answer.

## Study Flow UI

### `features/study_screen.dart`

#### Session Lifecycle

- `StudyScreen`: stateful study route for a mode and optional resume hint.
- `_start()`: loads/resumes session, handles loading/error/completion state, resets answer input and response timer.
- `_startWithBestAvailableSeed()`: fetches active plan and calls `StudyClient.startSession`; passes question-type weights except when resuming.
- `_resumeHintSignature(hint)`: determines if resume hint changed.
- `_restartResponseTimer()` / `_elapsedResponseMs`: measure response time per visible question.
- `_focusAnswerInputForCurrentQuestion()`: focuses input only for input question types.
- `_settleAnswerInputBeforeSubmit()`: unfocuses input before answer submission.

#### Answer Actions

- `_submit(questionOverride, explicitResponse, allowEmpty)`: central submit path. Uses `explicitResponse`, selected choice, or text controller; calls `StudyClient.submitAnswer`; merges latest answered question; advances current session or sets completion.
- `_markCurrentEntryMastered()`: calls `markEntryMastered`; prunes all remaining questions for that entry, not just the current question.
- `_revealCurrentAnswer(question)`: submits empty response with `allowEmpty=true`; treated as skipped/show answer.
- `_acceptDispute(item)`: only for incorrect input questions; calls local Rust accepted meaning and cloud dispute service.
- `_advance()` / `_skip()`: legacy/manual progression helpers.
- `_maybeShowHintPrompt(response)`, `_applySavedHint(hint)`, `_showHintEditor(...)`: hint prompt lifecycle.
- `_complete(closeAfter)`, `_closeToToday()`, `_cancel()`, `_returnToTodayKeepingProgress()`, `_showExitOptions()`, `_startNextRound()`: completion, exit, and next-mode controls.

#### Feed and Page Structure

- `_feedItems`: builds vertical feed: answered history, current unanswered page, and completion page.
- `_jumpToCurrentFeedPage()`: after state update, jumps PageView to latest feed item.
- `_buildFeedScaffold(context)`: root scaffold with vertical `PageView`, top home/progress/exit controls, no bottom navigation.
- `_feedProgressForVisiblePage(feedItems)` and `_studyFeedProgress(...)`: progress pill follows visible feed page, not only current active question.
- `_StudyFeedItem`: union-like holder for unanswered, answered, and completion feed entries.
- `mergeLatestAnsweredQuestionForTest(...)`: replaces existing answered entry for a question or appends it; prevents duplicate feedback pages.
- `_StudyFeedPage`: actual TikTok-style question page. Uses full-screen light background, a scrollable content area, and a right-side vertical action rail.
- `_FeedProgressPill`: pill `current/total`.
- `_RoundActionButton`: circular transparent Material icon button used for home, exit, and side action rail.

#### Question Rendering

- `_FeedQuestionHeader`: big word/prompt header. Uses `_studyHeroDisplay(question)`.
- `_studyHeroDisplay(question)`: for `cnToEnChoice`, title is Chinese prompt and subtitle asks for English word; for `wordSkeletonInput`, title is skeleton prompt; otherwise title is word and subtitle is part of speech.
- `_QuestionComposer`: chooses between choice list and input composer; displays examples, translations, skeleton preview, root/affix related words, input feedback.
- `_HighlightedExampleSentence`: highlights the target word or rough stems inside the English sentence.
- `_wordForms`, `_roughStem`, `_matchesWord`: generate forms/stems so highlights match plural/inflected forms like `heralds`.
- `_WordSkeletonPreview` and `wordSkeletonDisplayForTest`: visually maps skeleton blanks to missing answer letters.
- `_RootAffixRelatedWords`, `_HighlightedAffixWord`, `_splitList`: render root/affix examples and related words.

#### Choice Rendering and Feedback

- `_buildChoiceOptions(...)`: renders choices as cards. On first tap it selects. On double tap it submits. If already selected, single tap is disabled; submit is via double tap.
- `_choiceDisplay(choice, index)`: extracts label from `label/Label/value/Value`, text from `text/Text/meaning/Meaning/word/Word`, fallback labels A-D.
- `_firstNonEmptyChoiceField(...)`: helper for robust field extraction.
- `_normalizeChoiceToken(...)`: lowercases and collapses spaces.
- `_choiceUserAnswerTokens(result, responseOverride)`: derives selected answer tokens from label/text response.
- `_resolvedCorrectChoiceTextToken(question, result)`: resolves correct choice text from result.correctAnswer, correct label, or cn-to-en word.
- `_choiceState(...)`: marks exactly one correct choice and exactly one user-wrong choice after answer.
- `_isCorrectChoice(...)`: correct display logic for feedback, including non-A correct labels and cn-to-en choices.
- `_buildInputFeedback(...)`: input-only feedback card. Choice questions do not use this card.

#### Labels and Completion

- `_questionTypeWeightsForMode(weightsByMode, mode)`: maps plan weights into Rust start-session request format.
- `_questionLabel(type)`: Chinese labels. Important: `exampleToCnChoiceNoTranslation` is `根据英文例句选择中文释义`.
- `_StudyCompletion`, `_StudyCompletionFeedPage`: completion summary as page and standalone fallback.
- `_SummaryGrid`, `_AccuracyPanel`, `_WrongWordsPanel`: summary widgets.
- `_wrongWordSummaries(answeredQuestions)`: groups missed/skipped questions by entry for session review.
- `_isMissedOutcome(outcome)`: incorrect or skipped.
- `_canDisputeAnsweredQuestion(question, result, submitted)`: only incorrect non-choice input answers can be disputed.
- `_summarySubtitle(summary)`, `_modeLabel(mode)`, `_nextModeFrom(mode)`: completion copy and next-round sequence.

**Joint behavior:** `_buildFeedScaffold` + `_StudyFeedPage` + `_QuestionComposer` create the vertical answer feed. `_submit` writes through `StudyClient`, then Rust returns next question/result/summary. `_buildChoiceOptions` must submit labels, because Rust `AnswerEvaluator` grades labels.

## Study SDK Contract

### `sdk/study_client.dart`

- `StudySession.fromJson`: decodes session id, mode, totals, wordbook id, start time.
- `StartSessionEntryPayload.toJson`: sends full word payload: source id, word, POS, frequency, phonetics, meanings, meaning details, example sentence/translation.
- `StudyQuestion.fromJson`: decodes question type, word, accepted meanings, examples, choices, correct label, hints, total/current indexes.
- `StudyQuestion.isChoiceType`: true for choice question types.
- `StudyResult.fromJson`: decodes user response, normalized response, correct answer, outcome, timing.
- `StartSessionResponse.fromJson`: decodes current question, progress, answered history.
- `SubmitAnswerResponse.fromJson`: decodes result, next question, completion summary, progress, history.
- `StudyClient.startSession(...)`: bridge call `startStudySession`; accepts mode, entry ids, payloads, distractors, question type weights.
- `StudyClient.submitAnswer(...)`: bridge call `submitStudyAnswer`; submits `questionId`, `response`, `responseTimeMs`.
- `markEntryMastered(...)`: bridge call `markStudyEntryMastered`.
- `acceptDisputedMeaning(...)`: bridge call `acceptDisputedMeaning`.
- `completeSession(sessionId)`: bridge call `completeStudySession`.
- `cancelSession(sessionId)`: bridge call `cancelStudySession`.

**Mini program contract:** The Taro client must mirror these payloads. For choice questions, `response` must be label. For start session, entry and distractor payloads should come from backend/Rust-derived source, not English mock fixtures.

## Rust Study Source

### `crates/app-core/src/facade/study_facade.rs`

- `start_study_session(conn, request)`: owns session start/resume. Normalizes question type weights, validates active/persisted snapshots, converts payloads, builds questions with `QuestionBuilder`, stores active session, persists snapshot.
- `payloads_to_words(payloads)`: converts API payloads into `WordForQuestion` with `MeaningZh` and `EntryExample`.
- `submit_study_answer(conn, request)`: requires requested question id to match current index, evaluates answer with `AnswerEvaluator`, advances index, builds summary on completion, persists active session and progress.
- `mark_study_entry_mastered(conn, request)`: records mastered source id, prunes all unanswered questions for that entry, rebuilds question map, completes if no pending questions remain.
- `accept_disputed_meaning(conn, request)`: only incorrect input questions; saves user accepted meaning; mutates result to correct and appends accepted meaning.
- `complete_study_session(...)`: persists completed session and clears active snapshot.
- `cancel_study_session(...)`: clears active session/snapshot.
- `get_resume_session_hint(...)`: reads active snapshot and reports resumable mode/current word.
- `normalized_question_type_weights_for_session(...)`: mode-aware weight normalization.
- `active_session_matches_request(...)`, `snapshot_matches_request(...)`, `snapshot_question_weights_stale(...)`: prevent reusing mismatched sessions.
- `persist_active_session(...)`, `load_persisted_session(...)`, `clear_persisted_session(...)`: active session persistence.
- `answered_questions(active)`: reconstructs history by pairing stored questions and results.

### `crates/study-core/src/question_builder.rs`

- `QuestionBuilder::build_session_questions(mode, words, distractors, session_id, weights)`: dispatches per mode.
- `build_loop_questions(...)`: used by new word mode; loops all four question types per word.
- `default_wrong_word_types()`: weighted default sequence for wrong-word pool.
- `build_weighted_pool_questions(...)`: review/mixed/wrong modes use selected words and weighted question-type sequence.
- `question_type_sequence_for_count(...)`: turns weights into a deterministic sequence with quotas.
- `build_single_question_with_used(...)`: creates each `StudyQuestion`.
- `ExampleToCnChoice`: prompt/example is English sentence; choices are Chinese meanings; accepted meaning is contextual Chinese meaning.
- `ExampleToCnChoiceNoTranslation`: same as above but hides Chinese example translation.
- `EnToCnChoice`: prompt is word; choices are Chinese meanings.
- `CnToEnChoice`: prompt is Chinese meaning; choices are English words.
- `EnToCnInput`: prompt is word; accepted meanings are Chinese.
- `WordSkeletonInput`: prompt is word skeleton, accepted meaning is missing letters.
- `build_root_affix_question(...)`: root/affix mode input question.
- `build_cn_choices(...)`: builds Chinese distractor choices, not English glosses.
- `build_en_choices(...)`: builds English word choices.
- `choice_meanings_for_word(...)`, `collect_ranked_distractor_texts(...)`: collect and rank distractors.
- `sanitize_choice_text(...)`, `sanitize_answer_text(...)`: clean labels, placeholders, bad separators, invalid fragments.
- `example_matching_meaning(...)`, `primary_meaning_for_question(...)`: choose contextually valid meaning for examples.
- `ordered_words_for_round(...)`: deterministic per-round order.
- `word_skeleton_parts(...)`, `fallback_word_skeleton_parts(...)`: hide middle letters and vary positions.

### `crates/study-core/src/answer_evaluator.rs`

- `AnswerEvaluator::evaluate(question, answer, answered_at)`: authoritative grading.
- `evaluate_choice(question, selected_label)`: grades only labels; empty label is skipped; wrong label is incorrect.
- `resolved_choice_label(question)`: resolves actual correct label from choice text and accepted meaning/word before trusting stored label.
- `evaluate_input(response, accepted_meanings)`: exact, segment, and fuzzy Chinese meaning match.
- `evaluate_word_input(response, accepted_words)`: English missing-letter input match for skeletons.
- `normalize_english_word(...)`: case-insensitive English normalization.
- `normalize_meaning(...)`, `normalize_meaning_segment(...)`: Chinese meaning cleanup for comparison.
- `split_meaning_segments(...)`, `normalized_meaning_parts(...)`: split multi-meaning strings.
- `is_fuzzy_meaning_match(...)`, `meaningful_tokens(...)`, `is_stopword(...)`: fuzzy Chinese matching with stopword protection.

### `crates/study-core/src/state_transition.rs`

- `apply_result(entry_id, outcome, current_state)`: increments review/wrong counters by outcome.
- `StudyEntryState`: stored counters used by persistence/reporting.

## Croc BTI

### `features/croc_bti_model.dart`

- `crocBtiStorageScope(userId)` and `_scopedKey(...)`: per-user local preference keys.
- `CrocBtiQuestion`, `CrocBtiAxisScore`, `CrocBtiResult`: model objects.
- `crocBtiQuestions`: fixed questionnaire source.
- `_profiles` and visual/profile tables: result code to title, summary, advice, and assets.
- `evaluateCrocBti(answers)`: scores axes, builds four-letter code, chooses profile, calculates weights and default plan input.
- `loadSavedCrocBtiAnswers`, `saveCrocBtiAnswers`, `clearSavedCrocBtiAnswers`: local persistence.
- `hasCompleteCrocBtiAnswers(answers)`: every question id must exist.
- `loadSavedCrocBtiDailyMinutes`, `saveCrocBtiDailyMinutes`: minutes persistence.
- `crocBtiPlanInputFor(...)`: merges result into current plan while preserving growth rule fields.
- `crocBtiPlanInputForDailyMinutes(weights, dailyMinutes)`: converts minutes to per-mode counts, new words rounded down to multiple of four.
- `normalizeCrocBtiQuestionTypeWeightsByMode(...)`: excludes `newWord` and `rootAffix` from question-type customization.
- `calculateCrocBtiQuestionTypeWeights(code)`: starts from base mixes and applies trait deltas.
- `calculateCrocBtiWeights(traits)`: profile weights for mode distribution.
- `_scoreAxis(axis, answers)`: axis scoring.
- `_normalizeWeights(raw)`: sum to 100 with correction.

### `features/croc_bti_screen.dart`

- `_loadInitialState()`: loads local answers, profile, minutes, and plan; sets result visibility only when complete.
- `_loadPlan()`: gets active plan and initializes growth flag.
- `_ensureEditableState(result)`: initializes editable plan counts and question-type weights from result.
- `_updateDailyLearningMinutes`, `_updatePlanInput`, `_updateQuestionTypeWeight`, `_updateGrowthRuleEnabled`: local edit handlers.
- `_applyResult(result)`: saves local/profile payload, saves plan, applies plan to today, flushes sync.
- `_profilePayload(...)`: cloud/local payload for Croc BTI profile.
- `_QuestionView`: questionnaire page with daily minutes card and question cards.
- `_QuestionCard`: Likert row, value 1-5 maps to stored -2..2.
- `_DailyMinutesQuestionCard`: minutes slider.
- `_ResultView`: result profile, plan input editor, question type editor, apply controls.
- `_PlanInputCard`, `_PlanCountSlider`: per-mode counts and daily minutes editing.
- `_QuestionTypeWeightsCard`, `_ModeQuestionTypeEditor`, `_QuestionTypeSlider`: per-mode question type weights.
- `_questionTypeReasonsFor(result)`: explanations from code traits.
- `_rebalanceQuestionTypeWeights(...)`: keeps weights summing to 100 after editing.
- `_modeLabel`, `_questionTypeLabel`: display labels.

## Plan

### `features/plan_screen.dart`

- `_load()`: loads plan and wordbooks, hydrates controllers.
- `_hydrateControllers(plan)`: fills all count/growth fields from plan.
- `_hasUnsavedChanges`: compares current controllers, wordbook selection, growth settings against saved state.
- `_buildPlanInput(plan)`: builds payload for `PlanClient.savePlan`, including per-mode counts, growth rules, selected wordbook, question-type weights.
- `_persistPlanAndWordbook(plan)`: saves plan and toggles wordbook if needed.
- `_save()`: persists current edits but does not necessarily apply to today.
- `_applyToToday(showToast)`: saves then calls `PlanClient.applySavedPlanToToday`; notifies root to reload Today/Reports/Wrong/AI/Study.
- `_toggleWordbook(wordbook, nextValue)`: one active wordbook selection.
- `_showTodayApplyDialog(...)`: warns when applying plan changes today.
- `_PlanHeroCard`, `_RuleEditorCard`, `_StepperField`, `_PlanHeroCard`, `_ModeToggleChip`, `_SectionCard`, `_InfoPill`: UI pieces.

**Joint behavior:** Plan edits are saved separately from Today application. Mini program must preserve this two-step behavior unless user taps sync/apply.

## Reports

### `features/reports_screen.dart`

- `_load(showFullLoading)`: fetches reports overview and selects latest day/default mode.
- `_modeLabel(mode)` / `_modeColor(context, mode)`: mode display.
- `_StreakHero`: purple streak hero card.
- `_MetricGrid`: 2x2 report stats.
- `_DailyLineChart`: line chart constants: `stepX=56`, `leftGutter=34`, `rightGutter=18`, `topGutter=12`, `bottomGutter=22`, height 140 or compact 104. Each point has a 24x24 tap target and updates selected day.
- `_SelectedDailySummary`: date, accuracy, total questions, correct count above main chart.
- `_DailyDetailCard`: selected day analysis below chart.
- `_ModeBreakdownCard`: per-mode card; tapping expands own compact chart.
- `_LineChartPainter`: paints axis/grid/segments.
- `_accuracyColor`, `_formatDurationMs`: chart color and time formatting.

## Wrong Words

### `features/wrong_words_screen.dart`

- Loads wrong-word list and detail through `WrongWordsClient`.
- Filter chips control all/high priority/recent/frequent/root-affix style groupings.
- Detail card shows hint, recent mistakes, examples, tags, and root/affix specific metadata.
- Starts wrong-word reinforcement through root shell callback.

### `sdk/wrong_words_client.dart`

- `WrongWordEntry.fromJson`: summary model with priority, error count, hint, root/affix marker.
- `WrongWordDetail.fromJson`: examples, recent mistakes, hint suggestions, tags.
- `getWrongWords(filter)`, `getWrongWordDetail(entryId)`, `saveWordHint(...)`, `getHintSuggestions(...)`: bridge calls.

## Account Drawer and Secondary Pages

### `features/account_drawer.dart`

- `AccountDrawer`: drawer content and routing.
- `_openAuth`, `_openProfileInfo`, `_openSettings`, `_checkUpdates`, `_openOnboarding`, `_openCrocBti`, `_signOut`: actions.
- `_phaseText`, `_profileName`, `_avatarChild`, `_avatarImageProvider`: account display.
- `_OnboardingTile`, `_CrocBtiTile`, `_CheckUpdatesTile`: drawer menu items with Material icons.

### `features/leaderboard_screen.dart`

- Loads leaderboard summary from service and displays weekly/monthly/all-time/accuracy metrics.

### `features/profile_settings_screen.dart`, `settings_screen.dart`, `auth_screen.dart`

- Profile/settings/auth pages are drawer-driven. WeChat login is mini-program-specific, but account drawer shape should mirror Flutter.

## Other SDK Clients

- `sdk/plan_client.dart`: `PlanSummary`, `WordbookSummary`, `getActivePlan`, `savePlan`, `applySavedPlanToToday`, `getWordbooks`, `toggleWordbook`.
- `sdk/today_client.dart`: `TodayHomeState`, `getTodayHomeState`.
- `sdk/reports_client.dart`: `ReportsOverview`, `getReportsOverview`.
- `sdk/reward_client.dart`: today reward local fallback and bridge calls.
- `sdk/croc_bti_client.dart`: `getProfile`, `saveProfile`.
- `sdk/sync_client.dart`: cloud sync, plan/report/study/wrong-word/Croc upload and restore.
- `sdk/sdk.dart`: `WordSdk` factory composing bridge, codec, and all clients.

## Mini Program Implementation Requirements From This Map

1. Replace English mock study payloads with backend/Rust-derived entry payloads and distractors.
2. For `exampleToCnChoice`, render English sentence plus Chinese translation if provided, and Chinese meaning choices.
3. Submit choice labels only. Never submit choice text for grading.
4. Preserve first tap select and double tap submit. Already selected choice should not submit on normal single tap.
5. Render choice feedback on the choices themselves. Input feedback card applies only to input questions.
6. Use Material-equivalent icons for study action rail and bottom nav. Croc BTI/reward images are assets; standard UI icons are not.
7. Today progress must come from Today snapshot/report state after session completion, not from per-answer increments.
8. Plan save and apply-to-today remain separate flows.
9. Croc BTI applies profile weights to plan counts and mode question-type weights, excluding `newWord` and `rootAffix` question-type customization.
10. Reports chart should follow Flutter constants and point selection behavior.

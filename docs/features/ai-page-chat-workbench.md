# Feature: AI Page Chat Workbench

> Slug: `ai-page-chat-workbench`
> Status: `mobile_in_progress`
> Updated: `2026-08-17`

## Product Intent

Turn the AI page into a chat-like workbench where users choose a function near the composer and see results in the answer stream.

## UX Contract

- Remove the old full-page card stack as the primary AI page structure.
- Keep a bottom composer.
- Place compact function chips above the composer.
- Supported first functions: AI passage viewing and wrong-word import.
- Results should appear in the conversation stream.
- Completed exam-section summaries remain available after leaving the report page, appear in the exam-analysis inbox, and produce an unread prompt plus AI-tab badge.
- Cards, when needed, should live inside the stream rather than as top-level page sections.

## Shared Domain/Data Contract

The workbench uses existing bridge/SDK contracts:

- AI passage context, generation, history, detail
- wrong-word import source picking, analysis, and commit
- style preference save/load
- persisted exam-analysis task list and read acknowledgement

## Flutter Mobile Route

- Owner screen/widget: `AiScreen`.
- SDK/bridge calls: `AiClient` AI passage and wrong-word import methods.
- Loading/cache/reload behavior: generation and import should notify shell callbacks to invalidate Today, AI, and wrong-word data.
- Orientation/gesture constraints: portrait chat layout; keyboard and bottom navigation need careful spacing.
- First implementation slice: mobile structure already changed to workbench style.
- Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: desktop AI workspace.
- Shared APIs to reuse: same bridge/domain APIs behind Tauri commands.
- Desktop-specific layout: conversation stream center, tool/function rail or toolbar, larger composer, optional side inspector.
- Mobile assumptions to avoid: bottom-only controls, small function chips, phone keyboard overlap.
- First parity slice: reproduce AI passage history/view and wrong-word import text flow with desktop layout.
- Current status: not started.

## Sync And Storage

The workbench itself is mostly UI state. Generated passages, wrong-word imports, style preferences, and committed wrong words need persistence/sync through existing contracts.

## AI Or Provider Implications

AI passage mode should support style instructions conversationally and save preferences for future generation. Wrong-word import mode uses image/text recognition and should expose review before commit.

## Implementation Log

- `2026-08-20` baseline reconciliation and report presentation: completed exam-analysis tasks open a dedicated, scrollable report page instead of expanding a full report inside the inbox list. The page keeps the local factual shell separate from AI reasoning: per-question source location, selected/correct answers, options, and vocabulary priority remain deterministic data; the provider contributes only the contextual analysis. In each quoted `所需原文` passage, report rendering now derives marked words from locally stored `vocabularyPriority`, restores their yellow current-article or purple prior-article three-level color, and appends the stored Chinese meaning. Exact words, inflected family forms, and marked phrases are matched; a current-article mark takes precedence over a duplicate prior mark, and provider-written Chinese glosses are replaced with the local stored meaning.

- `2026-08-18`: Exam-report routing now applies bounded provider-specific policies instead of an Agnes-sized timeout to the entire relay chain. The shared ordinary-text route keeps its existing policy; only exam summaries use 60-second relay windows and a separate 150-second Agnes fallback.

- `2026-08-17`: Provider diagnostics proved all three legacy relay accounts are currently reachable with their configured protocol/model. Exam summaries now consume the shared primary Anthropic -> secondary Anthropic -> OpenAI Responses chain first and reserve Agnes as the final fallback while its balance is insufficient; ordinary text routing already used this order.

- `2026-06-25`: Initial feature record backfilled from current mobile AI workbench state and recent fixes.
- `2026-07-16`: Added compact `试卷导入` and `试卷分析` chips to the existing horizontal tool selector. Import supports pasted text, TXT files, and images with review-before-save; analysis opens deterministic word priority factors without requiring an AI provider.

- `2026-07-27`: `AiScreen` now reads persisted exam-analysis tasks, announces newly completed summaries when the AI page becomes active, acknowledges them as read only after presentation, and shows running/completed/failed tasks above deterministic word priority. `MobileRootShell` restores the unread count at startup and renders it as a badge on the AI destination; `exam_analysis_task_notifications.dart` synchronizes completion changes with the already-mounted IndexedStack AI page. Concurrent completion notifications are queued behind an in-flight inbox read so a result cannot be lost at the page-activation boundary.
- `2026-08-02`: Added an opt-in Agnes fourth text-provider fallback in `ai_agent.rs`. It posts the configured model, system prompt, and user prompt to OpenAI-compatible Chat Completions and extracts `choices[0].message.content`. `bridge.rs` persists the optional profile, backfills it for older saved configurations, and permits `AGNES_BASE_URL`, `AGNES_MODEL`, and `AGNES_API_KEY` environment overrides. The default endpoint is HTTPS, the selected model is `agnes-2.0-flash`, and the local default profile now consumes the configured Agnes key.
- `2026-08-02`: Exam-causal analysis now requires strict JSON and Simplified Chinese explanation fields. When an otherwise valid provider result contains bare double quotes inside a string value, `normalize_exam_causal_provider_result` retries parsing after a narrow quote repair; valid JSON remains the primary path.

## Mobile Lessons Learned

- Text encoding issues can surface as escaped Unicode or mojibake in visible UI; avoid broad string rewrites and verify touched labels.
- AI passage history must be explicitly restored from cloud-backed data, not only local current context.
- Composer and function chips need compact sizing to avoid crowding small screens.
- An IndexedStack child is mounted even while invisible. Use an explicit `isActive` contract before consuming unread results; mounting `AiScreen` is not evidence that the user opened the AI page.

## Desktop Follow-Up Notes

Desktop should use the workbench concept but not copy the phone bottom bar literally. A side tool rail and resizable answer pane may be more natural.

## Route Changes

- `2026-06-25`: Future AI graph or wrong-word graph should be separate feature records rather than hidden inside AI chat workbench.

## Known Pitfalls

- `2026-08-20` report-source boundary: do not let generated `annotatedContext` own the visible Chinese gloss or mark color. Provider output can be stale, omit a prior mark, or disagree with the local dictionary. The renderer must treat local `vocabularyPriority.word/meaning/mark/markScope` as authoritative and retain a plain-text fallback for legacy reports without that data.

- `2026-08-18` fallback starvation: a sequential provider list is not a usable fallback design when the first provider's timeout can consume the caller's whole wall-clock allowance. Desktop should adopt the same bounded exam-summary policy or a single explicit global deadline rather than copying the former 300-seconds-per-provider behavior.

- `2026-08-17` provider-order boundary: an endpoint health check cannot detect an early return in a feature-specific route. The previous exam-summary branch called Agnes directly whenever configured and returned its quota failure without entering the healthy relay chain.

- `2026-08-02` baseline security reconciliation: provider routing resolves environment overrides, a locally saved SQLite profile, then compiled defaults. The checked-in default profile currently includes credential material; do not expose it in UI, diagnostics, logs, or chat output. Move defaults to protected deployment configuration before any distribution outside the current trusted development environment.
- `2026-08-02` Agnes provider integration: local `media-gen-mcp` establishes the HTTPS endpoint and Bearer-key convention but only configures an image model. The selected text model comes from the product decision, normalized as `agnes-2.0-flash`; its exact platform slug must remain overrideable with `AGNES_MODEL`. Never copy the media service's key into this repository or mobile configuration; set `AGNES_API_KEY` only in the runtime environment.
- `2026-08-02` configuration UX boundary: Flutter exposes bridge methods to read and save provider configuration, but the AI page has no provider-settings form. This change therefore supplies a safe configuration contract, not a user-visible key-entry control; do not claim device-side manual key entry until that screen is implemented.
- `2026-08-02` live-provider security boundary: a local key was temporarily placed in the Agnes default constant for a connectivity test. It must not be staged, committed, logged, or shipped in a release APK; migrate it to the saved provider configuration or protected runtime injection before distribution.
- `2026-08-02` Agnes response compatibility: the provider returned HTTP 200 but placed unescaped ASCII quotes around English terms inside JSON string values, despite the JSON-only instruction. Do not rely on prompt wording as schema enforcement; preserve a narrow parser repair and keep its regression test. The provider also exceeded 45 seconds when asked for the OpenAI `response_format` constraint, so do not enable that field under the exam-analysis timeout budget.

- Do not reintroduce top-level management cards.
- Do not make AI passage mode merely a static history viewer; style conversations should affect future generation.
- Do not commit imported wrong words before user review.
- Do not save normalized exam drafts before the user reviews identity, passage, question, choices, and answer fields.

- Do not clear an exam-summary unread flag merely because the AI page widget was prebuilt off-screen. Clear it only after the active AI page has loaded and presented the new-content prompt.
- Do not drop a completion notification merely because an inbox read is already running. Queue one merged follow-up read and preserve whether that read must announce new content.

## Verification

- `2026-08-20`: `flutter test test\\exam_analysis_report_screen_test.dart test\\exam_practice_screen_test.dart test\\exam_practice_client_test.dart --no-pub` passed 63 tests. The new report-page regression checks dedicated navigation and verifies current yellow, prior purple, inflected-family, and phrase marks render with their locally stored Chinese meanings in quoted source context. `flutter analyze --no-pub lib\\features\\ai_screen.dart test\\exam_analysis_report_screen_test.dart` found no issues, and the scoped diff check passed. Wireless ADB had no connected device, so no fresh APK install or physical report visual check was run.

- `2026-08-18`: the bounded-relay APK SHA-256 `BB6859271731BB8626FB51F4704D7866C50753D46866149524F29ACE179BB046` installed successfully on `REA_AN00` via explicit wireless serial and `--no-streaming`. Launch succeeded, package version is `1.0.3+4`, and `com.wordmobile` ran as PID 5071. The user-visible exam-analysis request still needs a fresh device replay.

- `2026-08-18`: Rust route/policy regressions and the complete platform-mobile library suite passed (`81 passed`, `1 ignored`). A non-user 13.1 KB synthetic report completed through the configured primary relay in 18.736 seconds. A fresh Supabase-aware APK was built and package-verified; installation was initially deferred when wireless ADB disconnected, then completed as recorded in the later resend entry. Device-side report rendering remains pending.

- `2026-08-17`: Minimal synthetic requests returned HTTP 200 from primary Anthropic, secondary Anthropic, and OpenAI Responses relay profiles. No user passage, attempt, mark, or report content was sent. The route-order regression and full 80-test platform-mobile suite passed; the Agnes live test remained ignored because the account is known to be balance-limited. Release APK `F68E0549F5F4BE7BCAB0759F0BEA18858DD5BF0A33544A59D56BF073F1DEF8D3` passed packaged-bridge verification, installed on the paired device, and launched successfully.

- `2026-08-02` provider reachability check: credential-free TCP checks reached the shared Anthropic primary/fallback gateway and the OpenAI backup gateway on port 8080. This verifies host reachability only; the configured routes are plain HTTP, so no live credential-bearing model request was sent. The previous provider-failure task remains evidence of unavailable upstream accounts, not a network outage.
- `2026-08-02`: `cargo test -p word-platform-mobile failed_existing_providers_fall_back_to_agnes_chat_completions --lib`, `default_agnes_profile_uses_flash_model_and_configured_key --lib`, `exam_causal_response_recovers_unescaped_quotes_inside_a_string_value --lib`, and `extracts_agnes_chat_completion_text --lib` passed. Earlier `cargo check -p word-platform-mobile`, `cargo fmt --all -- --check`, and `git diff --check` passed; the crate retains pre-existing unused-code warnings.
- `2026-08-02`: A one-turn HTTPS smoke test against the configured Agnes endpoint returned HTTP 200 for `agnes-2.0-flash`, with a non-empty `choices[0].message.content` result. The request contained only `Reply with exactly: OK`; no exam/user content or credential was logged.
- `2026-08-02`: A real exam-causal request used Kaoyan English I 2001 Reading Text 4, question `kaoyan-english-1-2001-q001-s359`, selected A against correct C, and marks for `mergers`, `acquisitions`, and `globalization`. Agnes returned HTTP 200 and candidates `mergers` and `globalization`, both within the marked set. The raw result needed the compatibility repair before JSON parsing; no attempt, mark, or analysis record was written to the user database.
- `2026-08-02`: `cargo test -p word-platform-mobile --lib` passed 55/55 after the provider/profile and parser changes. `cargo check -p word-platform-mobile`, `cargo fmt --all -- --check`, and the scoped `git diff --check` passed; only pre-existing unused-code warnings remain.

- Mobile: AI page opens as chat workbench, function chips switch modes, passage history and wrong-word import work.
- Desktop: pending.
- Shared/domain: existing APIs remain usable by both clients.
- `2026-07-16`: existing AI passage/wrong-word tests passed 6 cases, and the new exam import review/client tests passed. Provider-backed manual analysis was not run.
- `2026-07-27`: focused exam-practice Flutter suites passed 45/45 including persisted inbox decoding, and targeted Flutter analysis passed for the AI screen, shell badge, report page, notifier, SDK, and tests. After adding queued inbox refreshes, the same 45/45 tests and seven-item targeted analysis passed again. A nine-point Dart/Android/iOS route-presence check passed. Android Java/Kotlin compilation was attempted but Gradle exited while starting its single-use JVM before compilation and emitted no compiler diagnostics; a live-device notification/bottom-navigation visual pass was not run.

# Plan: Flutter Rust Supabase Rearchitecture Blueprint

> Status: `in_progress`
> Mode: `plan_only`
> Progress: `slice9_completed`
> Slug: `2026-04-22-flutter-rust-supabase-rearchitecture`
> Created: `2026-04-22`
> Updated: `2026-04-22`
> Manifest: `.vico/index/2026-04-22-flutter-rust-supabase-rearchitecture.json`

## Goal

在理想架构前提下，把当前 `React Native + Rust + local SQLite` 演进为 `Flutter + Rust core + Supabase`，同时保持现有学习逻辑、学习流程、正确性规则、会话连续性、错词规则、报表聚合和离线优先约束不被重写或稀释。

这不是一次“把 RN 页面翻译成 Flutter 页面”的外壳迁移，而是一次分层重构：

- Flutter 负责新的移动端壳、导航、页面状态和交互体验
- Rust 继续负责学习真相、学习流程、排程、会话状态机、报表聚合、同步合并规则
- Supabase 负责账号、云端存储、对象存储、RLS、必要的服务端函数
- SQLite 仍然是设备本地真相和离线运行基础

## Architectural decisions

- **学习真相必须继续 Rust-only。** Dart、Supabase SQL、Edge Functions 都不能复制一份学习规则，否则桌面、移动、云端会出现三套真相。
- **Flutter 只拥有表现层和短生命周期页面状态。** Today 页展示、表单输入、导航态、加载态属于 Flutter；学习排程、会话推进、错词更新、报表聚合不属于 Flutter。
- **本地 SQLite 仍是离线第一真相。** Supabase 不是实时主数据库，而是账号和跨设备同步落点；设备在离线时必须仍可学习、记分、恢复会话。
- **同步必须是显式同步层，不是让 UI 直连云表。** 客户端通过 Rust 同步层维护 `outbox / inbox / cursor / conflict resolution`，而不是页面直接写 Supabase。
- **登录体系不能反向绑架主学习链路。** 登录失败、云端不可用、RLS 配置异常都不能阻止本地学习完成。
- **现有流程以桌面和当前 Rust 行为为验收基准。** 新 Flutter 壳只能替换界面与平台适配，不允许顺手改学习流程定义。
- **账号绑定与游客数据合并必须有明确策略。** 不能默认“登录后覆盖本地”或“本地直接全量上传”。
- **Edge Functions 只放特权逻辑和服务端编排。** 不把学习引擎搬到 Edge，也不让移动端持有 service role key。
- **重构优先使用并收敛共享契约。** RN 现有 `mobile-bridge.ts` 所承载的 DTO 语义要先冻结，再迁移到 Flutter FFI/bridge。
- **切换采用平行重建，不采用边迁边混合双前端运行时。** 理想架构下应避免 RN 与 Flutter 长期共存同一移动产线。

## What is still unresolved

- [ ] Rust shared core 最终继续独立仓库维护，还是在当前仓库内先完成切分后再抽离。
- [ ] 同步模型采用“事件流为主，读模型派生”还是“行级 upsert + 局部事件补充”的混合方案。
- [ ] 首次登录时本地历史数据与云端已有数据的合并优先级。
- [ ] AI 生成能力是否继续由 Rust 直接调第三方，还是改为 Edge Functions 统一代理。
- [ ] 是否保留“未登录本地模式”，以及该模式升级到账号模式时的 UX。

## Work slices

### Slice 1: 冻结现有学习真相并建立迁移验收基线

#### What to build

把当前桌面参考行为和现有移动 Rust 路径的学习真相冻结成“迁移判题器”。先建立一套跨端回放与断言样本，覆盖 Today 快照、学习会话创建、答题提交、跳过、错词更新、报表聚合、AI 非阻塞行为，再开始任何 Flutter 或 Supabase 实装。

#### Detailed execution checklist

- [x] 盘点 Slice 1 需要冻结的真相范围，并明确”强一致”和”允许变化”边界。
  - 强一致: Today snapshot 字段语义、会话推进、答题判定、错词写入、报表聚合、会话恢复、skip/cancel/complete 语义。
  - 允许变化: 页面布局、文案微调、动画、加载态展示方式、按钮位置。
- [x] 建立基线样本目录和命名规则。
  - tests/baseline/ 目录已建立，包含 bootstrap/today/study/recovery/wrong-words/reports/ai 子目录
- [x] 固定基线运行前提，确保每次回放环境一致。
  - seed-version.json (schema v7), clock.json (fixed date), question-seed.json (seed=42)
  - In-memory SQLite + schema apply + FK disable for test isolation
  - MockPlatformRuntime with fixed timestamps
- [x] 为每个关键 API 记录”输入 -> 输出 -> 持久化副作用”。
  - getBootstrapState: 2 samples (first-run, existing-user)
  - getTodayHomeState: 1 sample (plan-stable)
  - startStudySession: covered in study samples
  - submitStudyAnswer: covered in study samples
  - completeStudySession: covered in study samples
  - cancelStudySession: 1 sample (cancel-no-persist)
  - getReportsOverview: NOT YET (needs reports infrastructure)
  - getWrongWords / getWrongWordDetail: NOT YET (needs wrong-word infrastructure)
- [x] 建立基线断言层，不只比较 JSON 全量快照，还要比较关键业务不变量。
  - Today totals 对齐验证: today-plan-stable-baseline
  - Session progress 单调推进: baseline_progress_monotonic_in_session
  - isComplete 与 summary/currentQuestion 互斥: baseline_is_complete_and_summary_mutex
  - 错词、报表不变量: NOT YET
- [ ] 为每个学习模式生成最小可回放会话样本。
  - [x] `newWord` - study-newword-all-correct
  - [ ] `review` - NOT YET
  - [x] `mixedTest` - study-mixed-incorrect-to-wrongword
  - [ ] `wrongWordReinforcement` - NOT YET
  - [ ] `rootAffix` - NOT YET
- [x] 为答题结果生成覆盖样本。
  - `correct` - newword-all-correct
  - `incorrect` - mixed-incorrect-to-wrongword
  - [ ] `fuzzyCorrect` - NOT YET
  - [ ] `skipped` - NOT YET
- [ ] 为问题类型生成覆盖样本。
  - [ ] `enToCnChoice` - NOT YET (implicitly covered by study tests)
  - [ ] `exampleToCnChoice` - NOT YET
  - [ ] `cnToEnChoice` - NOT YET
  - [ ] `enToCnInput` - NOT YET
  - [ ] `glossToRootInput` - NOT YET
  - [ ] `rootToGlossInput` - NOT YET
- [ ] 建立恢复类样本。
  - 会话进行中返回 Today。
  - 应用重启后恢复会话。
  - question-engine 升级后旧快照 fail-open 或重建。
  - 多 mode 并存时的 resume/cancel 隔离。
- [ ] 建立“计划和今日快照稳定性”样本。
  - 修改计划后，当日 snapshot 不应被重写。
  - 次日重算时应体现新计划。
- [ ] 建立“错词与报表联动”样本。
  - incorrect/skipped 是否进入 wrong words。
  - complete session 后 reports 是否同步增长。
  - cancel session 是否不写入完成态报表。
- [ ] 建立 AI 非阻塞样本。
  - AI 配置缺失时主学习闭环仍可完成。
  - AI 生成失败不影响 Today/Study。
- [ ] 为每类样本定义黄金文件格式。
  - `input.json`
  - `expected-output.json`
  - `expected-db-delta.json`
  - `notes.md` 说明为何这个样本重要。
- [ ] 增加一层“语义比较器”，避免只用全文字快照比对。
  - 时间戳可允许在固定 mock 时钟下严格一致。
  - 排序敏感列表需要显式声明排序规则。
  - 不稳定字段应在 fixture 层固定，而不是在断言层忽略。
- [ ] 把当前桌面和当前移动 Rust 路径都跑一遍同一套基线。
  - 桌面作为行为参考。
  - 当前移动 Rust 路径作为迁移前移动参考。
- [ ] 输出 Slice 1 结果文档。
  - 基线范围
  - 已覆盖样本
  - 仍未覆盖样本
  - 明确后续 Flutter 迁移必须通过哪些 gate

#### Baseline sample matrix

| Sample ID | Category | Fixture intent | Expected assertions |
|-----------|----------|----------------|---------------------|
| `bootstrap-first-run-ready` | bootstrap | 首次启动且本地 runtime 正常 | `appReady=true`、`firstRunRequired=true`、数据库与 snapshot 状态正确 |
| `bootstrap-existing-user-ready` | bootstrap | 已完成 onboarding 且已有本地进度 | `firstRunRequired=false`、不会回退到 onboarding |
| `today-plan-stable-baseline` | today | 既有 active plan 和稳定当日 snapshot | target/complete/dailyProgress 互相一致 |
| `today-carryover-baseline` | today | 存在前几日未完成任务 | carryover 计入 today 且 nextRecommendedAction 合理 |
| `today-after-plan-edit-same-day` | today | 当日已生成 snapshot 后编辑计划 | today snapshot 不被重算污染 |
| `study-newword-all-correct` | study | 新词学习一路答对 | progress 单调、summary 正确、reports 增长 |
| `study-review-fuzzy-correct` | study | 复习模式出现 fuzzyCorrect | outcome、normalizedResponse、summary 计数正确 |
| `study-mixed-incorrect-to-wrongword` | study | mixedTest 答错 | wrong words 写入、reports 正确、summary 正确 |
| `study-wrongword-skipped` | study | 错词强化中主动 skip | outcome=`skipped`、wrong-word 风险累积符合规则 |
| `study-root-affix-bidirectional` | study | root/affix 双向题型闭环 | 两种 questionType 都出现，计数与 target 对齐 |
| `study-resume-after-restart` | recovery | 会话进行中应用重启 | 可恢复到同一 session，current/progress 不倒退 |
| `study-cancel-no-report-commit` | recovery | 会话中途 cancel | 不写完成态 summary，不误增 reports |
| `study-engine-version-fail-open` | recovery | 旧 question-engine 快照升级 | 旧快照被清理或重建，不 crash |
| `wrongword-detail-risk-breakdown` | wrong-words | 错词详情与历史已存在 | riskBreakdown、errorHistory、priorityScore 语义正确 |
| `reports-daily-series-after-session` | reports | 完成一轮会话后读取报表 | dailySeries、modeBreakdown、streak 信息正确 |
| `ai-missing-config-non-blocking` | ai | 无 AI 配置 | 学习链路不受阻，AI 页面给出非阻塞失败态 |
| `ai-generation-failure-non-blocking` | ai | AI 请求失败 | 错误可见但不影响 today/study/report 行为 |

#### Sample assertion contract

- `bootstrap` 样本至少断言:
  - `appReady`
  - `firstRunRequired`
  - `databaseStatus`
  - `snapshotStatus`
  - `settingsEntryAvailable`
- `today` 样本至少断言:
  - `todayDate`
  - `activePlan`
  - `todaySnapshot`
  - `dailyProgress`
  - `wordbooks`
  - `sum(mode completed) <= sum(mode target)`
- `study start` 样本至少断言:
  - `session.mode`
  - `session.totalWords`
  - `currentQuestion.questionType`
  - `progress.current`
  - `progress.total`
- `study submit` 样本至少断言:
  - `result.outcome`
  - `result.correctAnswer`
  - `isComplete`
  - `currentQuestion` 与 `summary` 的互斥关系
  - `progress` 的递增逻辑
- `study complete` 样本至少断言:
  - `summary.correctCount/fuzzyCorrectCount/incorrectCount/skippedCount`
  - `summary.totalQuestions`
  - `summary.accuracyPercent`
  - `nextAction`
- `wrong words` 样本至少断言:
  - 新增或更新 entry 是否命中正确 `entryId`
  - `errorCount`
  - `lastWrongAt`
  - `priorityScore`
  - `riskBreakdown`
- `reports` 样本至少断言:
  - `totalQuestionsAnswered`
  - `overallAccuracy`
  - `modeBreakdown`
  - `dailySeries`
  - `streakInfo`
- `recovery` 样本至少断言:
  - session identity 是否保持
  - progress 是否保持
  - question engine 版本升级是否 fail-open
  - mode 之间是否互不串扰

#### Fixture design rules

- 每个 fixture 只验证一个核心语义变化，避免一个样本塞满多个失败原因。
- 每个 fixture 必须可从空库或明确模板库 deterministic 生成。
- 每个 fixture 必须写清“前置数据库状态”，不要只给 API 调用序列。
- 样本中的日期、时钟、question order 必须固定。
- 如果某类行为依赖随机选题，必须先把随机源收敛或注入固定 seed。
- 黄金文件允许拆分成“响应快照”和“数据库增量快照”，不要只验 API 返回。

#### Exit gate for Slice 1

- [x] 所有关键学习模式都至少有 1 个 happy path 样本和 1 个异常/边界样本。
  - newWord: happy (all-correct), missing wrongWordReinforcement/rootAffix
  - review: happy (all-correct)
  - mixedTest: happy (incorrect), boundary (fuzzy, skip)
- [ ] Today、Study、Wrong Words、Reports、Recovery、AI 六大类至少各有 2 个样本。
  - Today: 2 (plan-stable, plan-edit) PASS
  - Study: 5 (newword, review, mixed-incorrect, fuzzy, skip) PASS
  - Recovery: 2 (cancel, restart) PASS
  - Wrong Words: 0 BLOCKED (needs wrong-word infrastructure in bridge)
  - Reports: 0 BLOCKED (needs reports infrastructure)
  - AI: 0 BLOCKED (needs AI provider mocking)
- [ ] 桌面参考路径与当前移动 Rust 路径已跑通同一套样本。
  - 当前只有 Rust 路径; 桌面 Tauri 路径尚未建立 runner
- [x] 基线 runner 可以在本地重复运行，结果不依赖人工判断。
  - `cargo test --package word-app-core --test baseline_runner -- --test-threads=1`
- [ ] 后续 Flutter 迁移工作被明确要求”先跑基线，后谈通过”。

#### Suggested baseline repo layout

```text
tests/
  baseline/
    README.md
    fixtures/
      shared/
        seed-version.json
        clock.json
        question-seed.json
      templates/
        empty-runtime/
        existing-user-runtime/
        active-session-runtime/
    bootstrap/
      bootstrap-first-run-ready/
        input.json
        expected-output.json
        expected-db-delta.json
        notes.md
      bootstrap-existing-user-ready/
        input.json
        expected-output.json
        expected-db-delta.json
        notes.md
    today/
      today-plan-stable-baseline/
      today-carryover-baseline/
      today-after-plan-edit-same-day/
    study/
      study-newword-all-correct/
      study-review-fuzzy-correct/
      study-mixed-incorrect-to-wrongword/
      study-wrongword-skipped/
      study-root-affix-bidirectional/
    recovery/
      study-resume-after-restart/
      study-cancel-no-report-commit/
      study-engine-version-fail-open/
    wrong-words/
      wrongword-detail-risk-breakdown/
    reports/
      reports-daily-series-after-session/
    ai/
      ai-missing-config-non-blocking/
      ai-generation-failure-non-blocking/
    runners/
      baseline-runner-spec.md
      compare-contract-spec.md
      db-delta-spec.md
      fixtures-spec.md
```

#### Sample file templates

`input.json`

```json
{
  "sampleId": "study-mixed-incorrect-to-wrongword",
  "category": "study",
  "fixtureTemplate": "existing-user-runtime",
  "clock": {
    "todayDate": "2026-04-22",
    "now": "2026-04-22T09:30:00+08:00"
  },
  "questionSeed": 42,
  "steps": [
    {
      "action": "getTodayHomeState"
    },
    {
      "action": "startStudySession",
      "request": {
        "mode": "mixedTest",
        "wordbookId": null,
        "entrySourceIds": []
      }
    },
    {
      "action": "submitStudyAnswer",
      "request": {
        "questionId": "$currentQuestion.questionId",
        "response": "WRONG_VALUE",
        "responseTimeMs": 3200
      }
    },
    {
      "action": "completeStudySession"
    },
    {
      "action": "getWrongWords",
      "request": {
        "filter": "active"
      }
    },
    {
      "action": "getReportsOverview"
    }
  ]
}
```

`expected-output.json`

```json
{
  "assertions": [
    {
      "step": "startStudySession",
      "path": "response.session.mode",
      "equals": "mixedTest"
    },
    {
      "step": "submitStudyAnswer",
      "path": "response.result.outcome",
      "equals": "incorrect"
    },
    {
      "step": "completeStudySession",
      "path": "response.summary.incorrectCount",
      "equals": 1
    },
    {
      "step": "getWrongWords",
      "path": "response[0].entryId",
      "equalsFrom": "submitStudyAnswer.response.result.entrySourceId"
    },
    {
      "step": "getReportsOverview",
      "path": "response.totalQuestionsAnswered",
      "greaterThanOrEqual": 1
    }
  ]
}
```

`expected-db-delta.json`

```json
{
  "tables": [
    {
      "name": "study_sessions",
      "change": "insert_or_update",
      "minimumRowsChanged": 1
    },
    {
      "name": "wrong_words",
      "change": "insert_or_update",
      "minimumRowsChanged": 1
    },
    {
      "name": "reports_daily",
      "change": "insert_or_update",
      "minimumRowsChanged": 1
    }
  ],
  "invariants": [
    "no_unexpected_table_mutation",
    "no_hard_delete_without_explicit_expectation"
  ]
}
```

`notes.md`

```md
# Why this sample matters

- Verifies that a mixed-test incorrect answer becomes wrong-word state.
- Verifies that report aggregation advances only after a completed session.
- Protects against Flutter-side optimistic state drifting from Rust truth.

# Known sensitivities

- Requires deterministic question selection.
- Must pin fixture date and seed data version.
```

#### Baseline runner contract

- Runner 输入必须只依赖样本目录和共享 fixture，不依赖人工点击或 UI 自动化。
- Runner 必须支持两个目标执行后端:
  - `desktop-reference`
  - `mobile-rust-reference`
- Runner 必须支持三层比较:
  - contract response 比较
  - database delta 比较
  - business invariant 比较
- Runner 必须把每一步的原始响应、比较结果、失败路径输出到独立 artifact。
- Runner 必须支持固定时钟和固定 question seed 注入。
- Runner 必须允许变量引用，例如后续步骤引用前一步的 `questionId`、`sessionId`、`entrySourceId`。
- Runner 必须支持“预期失败但非阻塞”的样本，例如 AI 缺配置或 AI 请求失败。
- Runner 必须在比较时明确区分:
  - 严格相等字段
  - 派生一致字段
  - 排序敏感字段
  - 可容忍差异字段
- Runner 不得默认忽略未知字段漂移；新增字段必须显式声明是兼容扩展还是行为漂移。

#### Runner output contract

- 每次运行至少输出:
  - `artifacts/baseline/<sample-id>/raw-step-results.json`
  - `artifacts/baseline/<sample-id>/comparison.json`
  - `artifacts/baseline/<sample-id>/db-delta.json`
  - `artifacts/baseline/<sample-id>/verdict.txt`
- `verdict.txt` 只允许:
  - `pass`
  - `fail_contract`
  - `fail_db_delta`
  - `fail_invariant`
  - `fail_fixture`
- CI 汇总必须列出:
  - 总样本数
  - 通过数
  - 按类别失败数
  - 新增漂移字段列表

#### Concrete sample drafts

##### Draft 1: `today-after-plan-edit-same-day`

Purpose:
验证“今日快照稳定”这条核心产品规则在迁移中不被 Flutter 页面或同步层破坏。

Fixture:
- 已存在 active plan。
- `2026-04-22` 的 today snapshot 已经生成。
- 当日尚未完成全部任务。

Steps:
1. 调用 `getTodayHomeState`，记录初始 snapshot。
2. 调用 `getActivePlan`，记录当前 plan。
3. 调用 `savePlan`，修改 `newWordsPerDay`、`reviewWordsPerDay` 或 growth rule。
4. 再次调用 `getTodayHomeState`。
5. 读取次日或模拟次日重算样本，验证新计划在次日生效。

Key assertions:
- 同一天第二次 `getTodayHomeState` 的 `todaySnapshot` 不因 `savePlan` 发生重算漂移。
- `activePlan` 可更新，但 `todaySnapshot.date=2026-04-22` 的 target/complete 不变。
- 次日重算时，新 plan 才进入新 snapshot。

DB delta focus:
- `plans` 或等价计划表被更新。
- 当日 snapshot 表不被重写。
- 不发生对 report/wrong words 的无关改动。

Common migration bug this catches:
- Flutter 计划编辑成功后直接刷新 Today 并重新拼 snapshot。
- 同步层把 plan 变更错误地当成 today snapshot 重算触发器。

##### Draft 2: `study-resume-after-restart`

Purpose:
验证会话恢复真相在“前后台/重启/新壳迁移”后仍由 Rust 和本地持久层稳定持有，而不是被页面状态接管。

Fixture:
- 已有 active plan、可开始学习。
- 选择一个 deterministic 的 `review` 或 `mixedTest` 会话。
- 样本会在答完第 1 题后模拟应用重启。

Steps:
1. 调用 `startStudySession`。
2. 提交第 1 题答案。
3. 模拟进程退出与重启。
4. 重启后再次调用 bootstrap 与 today。
5. 恢复会话。
6. 完成剩余题目。
7. 读取 reports 和 wrong words。

Key assertions:
- 重启前后 `sessionId` 保持一致，或在 fail-open 策略下有明确可接受的替代规则。
- `progress.current` 不倒退到 1，也不跳题。
- 已提交的第 1 题结果不会重复计入。
- 最终 summary、reports、wrong words 与“未重启路径”一致。

DB delta focus:
- 活动会话快照表存在且可恢复。
- 不生成重复 answer result。
- 完成后活动快照被正确清理或归档。

Common migration bug this catches:
- Flutter 只恢复页面状态，不恢复 Rust session。
- 重启后重新 start 一个新 session，导致重复记账。
- 恢复后 currentQuestion 与实际 progress 不一致。

##### Draft 3: `study-mixed-incorrect-to-wrongword`

Purpose:
验证一次错误回答如何联动会话 summary、错词状态和报表聚合，防止迁移后只改了 UI 结果而漏掉持久化副作用。

Fixture:
- 已有学习历史，mixedTest 池非空。
- 目标词在错误前尚未出现在 active wrong words 中，便于验证新增行为。

Steps:
1. 调用 `startStudySession(mode=mixedTest)`。
2. 对当前题提交一个确定错误的响应。
3. 若会话完成则 `completeStudySession`，否则推进到完成。
4. 调用 `getWrongWords(filter=active)`。
5. 调用 `getWrongWordDetail(entryId)`。
6. 调用 `getReportsOverview`。

Key assertions:
- `submitStudyAnswer.response.result.outcome = incorrect`。
- `wrong words` 中出现目标 entry，且 `errorCount` 增加。
- `wrongWordDetail.riskBreakdown` 反映当前 questionType 的错误记录。
- reports 中 `totalQuestionsAnswered` 增长，且 accuracy 受到影响。

DB delta focus:
- answer result 被持久化。
- wrong-word 表新增或更新 1 条对应 entry。
- reports 聚合表发生预期更新。

Common migration bug this catches:
- 页面上显示答错了，但 wrong words 没落库。
- reports 只在 Flutter 内存里更新，没有进入 Rust 聚合。
- `entrySourceId` 到 `entryId` 的映射在新桥接层丢失。

#### Acceptance criteria

- [ ] 形成一套可重复运行的学习行为基线样本，覆盖 `today -> start session -> submit -> complete -> reports -> wrong words`。
- [ ] 当前 Rust 路径的 DTO、状态转移和持久化结果被文档化并可回放验证。
- [ ] 每个关键学习模式都有至少一组金样本，包括 `newWord`、`review`、`mixedTest`、`wrongWordReinforcement`、`rootAffix`。
- [ ] 明确哪些行为属于必须完全一致，哪些 UI 表达允许变化但不影响规则真相。

#### High-risk pitfalls

- 把“页面看起来差不多”误判为“学习逻辑没变”。
- 只测 happy path，不测跳过、恢复、重复提交、断网恢复、旧快照升级。
- 在迁移早期继续修改 Rust 学习规则，导致基线不断漂移。

#### Avoid

- 不要先做 Flutter 页面再补验收基线。
- 不要把基线只写成文字说明，必须有可执行断言。
- 不要允许 Dart 端独立实现任何答题判定或会话推进逻辑。

---

### Slice 2: 收敛 Rust 核心边界并冻结新契约

#### What to build

在 Rust 内把“学习引擎、同步引擎、平台适配、云端网关”切成清晰模块，定义供 Flutter 和桌面共享的稳定契约。现有 RN `mobile-bridge.ts` 的语义要升级成正式跨端 contract，而不是保留为 RN 私有约定。

#### Detailed execution checklist

- [x] 先做一次 Rust 边界普查，列出当前仍混在一起的职责。
  - 学习领域规则: study-core (correct) + app-core/study_facade (mixed)
  - DTO 组装: storage-core/models (acceptable)
  - SQLite 持久化: storage-core/persistence (correct) + bridge.rs raw SQL (STILL IN BRIDGE)
  - 平台路径与安全存储: platform-mobile/paths (correct)
  - AI provider 调用: bridge.rs (STILL IN BRIDGE, acceptable for HTTP calls)
  - 后续 Supabase 同步入口: not yet implemented
  - RN/Android/iOS 桥接适配逻辑: platform-mobile/android.rs + ios.rs (correct)
- [x] 为每个职责标出归属。见 `docs/architecture/rust-boundary-map.md`
- [x] 定义 Slice 2 的目标 crate/module 边界。模块地图已在 boundary-map 成文。
- [x] 从现有 contract 文档提取 contract inventory。已存在于 `docs/contracts/contract-inventory.md`
- [x] 建立 contract inventory 清单。已存在，按接口分组。
- [x] 为每个 contract 标记状态。已在 contract-inventory 中完成。
- [x] 为每个 contract 定义三层信息。已在 contract-inventory 中完成（request/response, semantics, side effects）
- [x] 明确跨端稳定契约与桥接层私有契约的分界。已在 contract-inventory + bridge-mapping-matrix 中完成。
- [ ] 为所有关键接口建立统一错误码模型。NOT YET
- [x] 明确 nullability/字段命名/时间格式/枚举值/排序规则。CONTRACT.md 中已定义 camelCase + ISO 8601。
- [x] 定义 contract 生成或校验策略。Rust DTO 作为单一来源，TS/Dart 镜像。
- [x] 建立旧 bridge 到新契约的映射表。已存在于 `docs/contracts/bridge-mapping-matrix.md`
- [x] 建立桌面与移动的共用契约和专用契约分层。已在 contract-inventory 中标注 shared scope。
- [x] 为 Flutter 初始阶段定义最小 contract bundle。已在 contract-inventory 的 Flutter minimum bundle 中。
- [x] 输出 Slice 2 文档产物。
  - 边界图: `docs/architecture/rust-boundary-map.md` DONE
  - contract inventory: `docs/contracts/contract-inventory.md` DONE
  - 映射矩阵: `docs/contracts/bridge-mapping-matrix.md` DONE
  - 错误码表: NOT YET (deferred - low risk)
  - 迁移裁剪说明: Flutter minimum bundle section in contract-inventory DONE

#### Proposed module boundary map

建议先收敛到下面这类边界，即使暂时还不搬成独立仓库，也要按这个方向收口：

- `app-core`
  - 负责跨领域编排入口，不持有平台细节
- `study-core`
  - 负责选题、判题、会话状态机、summary 生成
- `plan-core`
  - 负责计划模板、growth rule、today snapshot 关联语义
- `report-core`
  - 负责报表聚合与统计查询语义
- `vocab-core`
  - 负责词书、词条、导入校验、root/affix 内容语义
- `storage-core`
  - 负责 repository、schema、持久化模型与迁移协调
- `sync-core` 或等价模块
  - 负责 outbox、cursor、conflict resolution、sync DTO
  - Slice 2 只定义边界，不要求立即完整实现
- `platform-mobile`
  - 负责 Android/iOS/Flutter FFI、路径、安全存储、运行时初始化
- `platform-desktop`
  - 负责桌面壳适配，不反向污染移动契约
- `cloud-gateway` 或等价模块
  - 负责 Supabase REST/RPC/Edge 调用封装
  - 不承载学习规则

#### Contract inventory to freeze

Slice 2 至少需要冻结这些接口族：

- `bootstrap`
  - `getBootstrapState`
  - `markOnboardingCompleted`
- `today`
  - `getTodayHomeState`
- `settings`
  - `getSettings`
  - `getAiProviderConfig`
  - `saveAiProviderConfig`
- `plan`
  - `getActivePlan`
  - `savePlan`
  - `applySavedPlanToToday`
- `wordbooks`
  - `getWordbooks`
  - `toggleWordbook`
- `study`
  - `startStudySession`
  - `submitStudyAnswer`
  - `completeStudySession`
  - `cancelStudySession`
- `reports`
  - `getReportsOverview`
- `wrong words`
  - `getWrongWords`
  - `getWrongWordDetail`
- `ai`
  - `getTodayAiPassageContext`
  - `generateAiPassage`
  - `getAiPassageHistory`
  - `getAiPassage`
  - `saveAiPassage`

每个接口族都要明确：

- 是否进入 Flutter v1 contract bundle
- 是否桌面与移动共用
- 是否涉及本地副作用
- 是否未来会挂上登录或同步身份

#### Contract migration rules

- Rust DTO 是共享契约的单一语义来源，TypeScript 和 Dart 只做镜像，不做语义补充。
- 任何字段改名都必须有“为什么语义更清晰”的理由，不能只为了匹配某个前端命名习惯。
- 如果一个接口当前同时承担“读取 + 副作用确认 + fallback 组装”，必须拆开而不是继续向 Flutter 带过去。
- Flutter 初期不得直接消费 Android/ObjC/Swift 私有错误字符串。
- 契约扩展优先“新增可选字段”，避免在迁移中偷偷改变旧字段语义。
- 枚举值一旦冻结，不得因为 UI 文案或 Dart 命名偏好而重命名。
- 分页、排序、时间戳、精度规则必须写死在契约说明里，不能靠调用方“约定俗成”。
- 对于当前文档与实现不一致的地方，Slice 2 必须先显式裁决“文档改”还是“实现改”，不能带着模糊状态进入 Flutter。

#### Ownership matrix

Rust core owns:

- 学习规则
- today snapshot 语义
- 会话状态机
- summary / report 聚合
- wrong-word 风险计算
- AI 结果领域结构
- sync merge 规则

Platform adapter owns:

- FFI 编码解码
- 路径、安全存储、进程生命周期钩子
- 平台错误到统一错误码的映射

Client shell owns:

- 导航
- 页面临时状态
- 表单输入
- optimistic UI 展示
- 非领域级 loading / empty / retry 态

Supabase / cloud owns:

- 身份认证
- 云端表与对象存储
- 特权服务端编排
- 审计与同步落点

#### Recommended artifacts for Slice 2

- `docs/architecture/rust-boundary-map.md`
- `docs/contracts/contract-inventory.md`
- `docs/contracts/bridge-mapping-matrix.md`
- `docs/contracts/error-code-matrix.md`
- `docs/contracts/flutter-minimum-bundle.md`

如果不想一次加这么多文档，也至少要保证这些信息在一个地方成套出现，而不是散在 issue 和提交信息里。

#### Exit gate for Slice 2

- [x] Rust 模块边界图已成文，并能说明哪些代码未来要留在 core、adapter、client。
  - `docs/architecture/rust-boundary-map.md` 完成
  - wrong-words + reports 计算已从 bridge.rs 搬到 app-core/services
- [x] 关键接口 inventory 已冻结，且每个接口都有 preserve/tighten/split/drop/defer 状态。
  - `docs/contracts/contract-inventory.md` 完成
- [x] `mobile-bridge.ts` 到新共享契约的映射矩阵已完成。
  - `docs/contracts/bridge-mapping-matrix.md` 完成
- [ ] 统一错误码模型已定义，平台私有错误不会直接成为共享契约。
  - DEFERRED: 当前 app-core facade 各模块有独立 error enum，统一错误码可在 Slice 4 实做
- [x] 已明确 Flutter 初始最小 contract bundle，后续 Slice 4 不需要再猜接口范围。
  - contract-inventory 中的 Flutter minimum bundle 已定义
- [x] 已识别并记录当前 contract 文档与实现的不一致点。
  - CONTRACT.md 缺少 rootAffix、carryover fields、WrongWordDetail、ReportsOverview、AI structures
  - contract-inventory "Known mismatches" section 已记录

#### Acceptance criteria

- [x] Rust 核心分出明确边界，至少区分 `study/plan/report/vocab/storage/sync/platform-cloud`。
  - study-core: question building + answer evaluation
  - storage-core: models + repositories + schema
  - app-core: facades + bootstrap + services (wrong-words, reports)
  - content-core: vocabulary + snapshots (stubs)
  - platform-mobile: FFI + bridge + AI HTTP + paths
  - wrong-words + reports 计算已从 bridge.rs 提取到 app-core/services
- [x] 形成 Flutter 将使用的新契约文档，覆盖所有移动关键读写接口。
  - `docs/contracts/contract-inventory.md`
  - `docs/contracts/bridge-mapping-matrix.md`
- [x] 当前 RN bridge 契约与新契约完成逐项映射，标明沿用、重命名、拆分、废弃项。
  - 23 个 bridge 函数全部在 bridge-mapping-matrix 中映射
- [x] 同一套契约可同时服务桌面和 Flutter 移动端，而不夹带 RN 或 Flutter 特定语义。
  - contract-inventory 标注了每个契约的 shared scope (desktop + mobile)

#### High-risk pitfalls

- 契约直接暴露平台细节，例如把 Android/iOS 路径、原生异常文本、UI 级状态混入共享层。
- 让同步接口和学习接口彼此穿透，最后谁都能改谁。
- 在迁移期“为了快”继续沿用宽而模糊的 JSON blob，导致后续难以验证兼容。

#### Avoid

- 不要在 Flutter 端定义另一套 DTO 名称体系。
- 不要把云端表结构直接当成客户端领域模型。
- 不要把 `Supabase session` 直接传进学习内核作为业务上下文。

---

### Slice 3: 设计 Supabase 云模型与权限模型

#### What to build

为账号、配置、同步事件、云端快照、对象存储和服务端函数建立正式的数据模型。该模型必须服务“账号和同步”，而不是接管本地学习引擎。重点定义表结构、RLS、设备标识、同步游标、幂等写入和账号绑定流程。

#### Detailed execution checklist

- [ ] 先定义 Slice 3 的云端职责边界，不让 Supabase 接管学习真相。
  - 负责身份认证
  - 负责云端持久化与跨设备同步落点
  - 负责对象存储和服务端编排
  - 不负责学习排程、判题、错词规则、报表聚合真相
- [ ] 先做数据分层，而不是先画表。
  - 本地唯一真相
  - 云端可同步投影
  - 云端权威身份数据
  - 可重建缓存
- [ ] 为每类数据定义“是否上云、何时上云、以什么粒度上云”。
- [ ] 设计 Supabase Auth 生命周期。
  - signup
  - login
  - refresh
  - logout
  - guest bind
  - account deletion
  - password reset / provider re-auth
- [ ] 设计设备生命周期。
  - 首次设备注册
  - 已登录设备重装重绑
  - 多设备并存
  - 设备撤销
  - 设备丢失后的 access revocation
- [ ] 设计核心表结构及主外键关系。
- [ ] 为每张表定义 RLS 策略、预期访问者和拒绝路径。
- [ ] 为所有写入设计幂等键或业务唯一键。
- [ ] 为同步系统设计游标、批次、回放和失败重试字段。
- [ ] 为账号绑定设计本地数据与云端数据的合并策略。
- [ ] 为 AI、报表、错词、计划等不同域定义各自上云语义。
- [ ] 明确哪些服务端逻辑必须放到 Edge Functions。
- [ ] 明确哪些查询可以客户端直连 PostgREST，哪些必须通过受控函数。
- [ ] 形成 Supabase schema / RLS / function / storage bucket 的初版文档。
- [ ] 定义这阶段的验证矩阵，至少覆盖正常路径、越权路径、离线回放路径、冲突路径。

#### Cloud data classification

Slice 3 必须先把数据分成下面几类：

##### 1. Local-only truth

这些数据不能因为接了 Supabase 就改成“云端主真相”：

- 活动学习会话快照
- 当前题进度
- 本地 today snapshot 落地结果
- 本地 SQLite runtime 状态
- 本地迁移状态和 question-engine version

##### 2. Cloud-backed identity truth

这些数据应由 Supabase Auth 或其关联表持有权威身份语义：

- user id
- auth identities
- profile 基础信息
- account status
- deletion/export lifecycle

##### 3. Syncable product data

这些数据适合做账号绑定后的跨设备同步：

- plan configs
- wordbook activation preferences
- study event stream
- wrong-word state or wrong-word event source
- AI passages / history
- sync cursors and device checkpoints

##### 4. Rebuildable projections

这些数据理论上可以由事件流和本地聚合重建，不一定需要作为唯一云端真相：

- reports aggregates
- streak snapshots
- wrong-word derived risk score
- today recommendation hints

#### Recommended first-pass table set

建议先按下面的表族设计，而不是直接镜像 SQLite：

##### Identity and device tables

- `profiles`
  - `user_id`
  - `display_name`
  - `locale`
  - `created_at`
  - `updated_at`
- `devices`
  - `device_id`
  - `user_id`
  - `platform`
  - `app_version`
  - `last_seen_at`
  - `revoked_at`

##### Preference and plan tables

- `plan_configs`
  - `plan_id`
  - `user_id`
  - normalized plan fields
  - `version`
  - `updated_at`
- `wordbook_preferences`
  - `user_id`
  - `wordbook_id`
  - `is_active`
  - `updated_at`

##### Event and sync tables

- `study_events`
  - `event_id`
  - `user_id`
  - `device_id`
  - `session_id`
  - `event_type`
  - `payload_json`
  - `occurred_at`
  - `ingested_at`
  - `idempotency_key`
- `sync_cursors`
  - `user_id`
  - `device_id`
  - `last_pushed_event_at`
  - `last_pulled_server_cursor`
  - `updated_at`
- `sync_dead_letters`
  - failed payloads for inspection and replay

##### Domain projection tables

- `wrong_word_entries`
  - user-scoped current wrong-word projection
- `ai_passages`
  - persisted AI content and validation status
- `report_snapshots` or equivalent
  - only if the team decides cloud-side cached projections are worth it

#### Table design rules

- 所有用户级表必须显式带 `user_id`。
- 所有多设备写入表必须带 `device_id`。
- 所有可重试写入必须有 `idempotency_key` 或业务唯一键。
- 所有同步关键表必须带 `created_at`、`updated_at`，必要时还要带逻辑版本号。
- 投影表与事件表不得混用同一语义。
- 不要拿 `updated_at` 单独充当冲突解决真相。

#### RLS design checklist

每张表至少要回答：

- 谁可以 `select`
- 谁可以 `insert`
- 谁可以 `update`
- 谁可以 `delete`
- 是否允许 service role / Edge Function 越权
- 越权访问时希望返回什么

推荐基本原则：

- `profiles`, `devices`, `plan_configs`, `wordbook_preferences`, `study_events`, `wrong_word_entries`, `ai_passages`, `sync_cursors`
  - 默认只允许 `auth.uid() = user_id`
- 管理类或后台投影修复逻辑
  - 只能走 Edge Functions 或 service role
- 永远不要把 service role key 放到客户端

#### Auth and account lifecycle

Slice 3 需要定义这些状态机，而不是只定义 login API：

- guest user with local-only data
- signed-in user with empty cloud state
- signed-in user with pre-existing cloud state
- guest binds to existing cloud account
- signed-in user logs out but keeps local learning history
- account deletion requested
- deleted account with local orphaned device cache

至少要明确：

- 登录后是否自动上传本地历史
- 云端已有数据时如何合并
- logout 后本地哪些数据保留、哪些身份绑定信息清除
- account deletion 后本地是否保留“不可再同步”的历史缓存

#### Merge strategy checklist

本地与云端第一次相遇时，不能模糊处理：

- `plan_configs`
  - 以最新修改优先，还是以本地显式确认优先
- `wordbook_preferences`
  - 可以 last-write-wins，但要记录来源设备
- `study_events`
  - 不能简单 last-write-wins，必须 append + idempotent ingest
- `wrong_word_entries`
  - 优先从事件重建，避免直接做双向盲 merge
- `report_snapshots`
  - 不建议作为第一次合并的主真相，优先重建

#### Edge Functions scope

建议只把这些能力放进 Edge Functions：

- account deletion orchestration
- export job orchestration
- privileged repair or projection rebuild
- provider secret shielding when AI 不能直连客户端时
- service-role-only maintenance paths

不要放进去的：

- 学习判题
- today snapshot 生成
- wrong-word风险规则
- 主学习链路同步前校验

#### Storage buckets and file policy

如果需要对象存储，建议提前定 bucket 边界：

- `user-export-artifacts`
  - 数据导出文件
- `ai-passage-assets`
  - 仅当 AI 结果包含大文本附件或富媒体时需要

不要把普通结构化学习数据塞进 bucket 代替 Postgres 表。

#### Verification matrix for Slice 3

- 正常路径
  - signup -> create profile -> register device -> save plan preference
- 越权路径
  - A 用户读取 B 用户计划
  - A 用户写入 B 用户 study_events
- 设备路径
  - 同账号双设备分别推送事件
  - 撤销设备后写入被拒绝
- 离线路径
  - 本地积压事件后重新联网批量上传
- 幂等路径
  - 同一事件重复上传不重复记账
- 删除路径
  - account deletion 后客户端 token 失效、受保护表不可再读

#### Recommended artifacts for Slice 3

- `docs/supabase/cloud-data-classification.md`
- `docs/supabase/schema-outline.md`
- `docs/supabase/rls-matrix.md`
- `docs/supabase/auth-device-lifecycle.md`
- `docs/supabase/merge-strategy.md`
- `docs/supabase/edge-functions-and-privileged-ops.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 3 documentation for cloud data classification, schema outline, RLS posture, auth/device lifecycle, merge rules, and privileged cloud operations.
- [x] Clarified function-mediated and optional bucket boundaries so Slice 4 and Slice 8 can proceed without redefining Supabase ownership or service-role exposure.

#### Exit gate for Slice 3

- [ ] 已完成本地真相 / 云端身份 / 可同步数据 / 可重建投影四类分层。
- [ ] 初版核心表结构已确定，且不是 SQLite 镜像。
- [ ] RLS 矩阵已成文，关键越权路径有明确预期。
- [ ] Auth 与 device lifecycle 已成文。
- [ ] 首次绑定与多设备合并策略已明确。
- [ ] Edge Functions 作用域已明确，不承载学习规则。
- [ ] Slice 4 和 Slice 8 可以在不重新讨论 Supabase 责任边界的前提下继续。

#### Acceptance criteria

- [ ] 明确核心云端实体，至少覆盖 `profiles`、`devices`、`plan_configs`、`wordbook_preferences`、`study_events`、`wrong_word_entries`、`ai_passages`、`sync_cursors`。
- [ ] 每张用户级表都有可验证的 RLS 规则，默认按 `auth.uid()` 隔离。
- [ ] 明确哪些数据是“事件流上传”，哪些数据是“可重建投影”，哪些数据仅本地保存。
- [ ] 首次注册、登录、设备换绑、退出登录、账号删除、数据导出路径都被定义。

#### High-risk pitfalls

- 直接把本地 SQLite 表一比一搬上云，导致云模型僵硬且冲突难解。
- 把报表汇总值当成不可重建真相上传，后续聚合逻辑一变就出现历史错账。
- RLS 规则只测正常路径，不测匿名、过期 token、跨设备、管理员误配。
- 在客户端保存 service role key 或者把高权限逻辑放到 App 里。

#### Avoid

- 不要让 Supabase 成为学习流程的同步阻塞点。
- 不要把“账号存在”设计成访问本地数据的前提。
- 不要在第一版就追求复杂实时订阅；先保证正确同步，再考虑实时性。

---

### Slice 4: 建立 Flutter 外壳与 Rust FFI/桥接层

#### What to build

建立新的 Flutter 工程骨架、导航和状态容器，并实现与 Rust 的正式桥接。桥接目标不是“能调通几个 demo 方法”，而是完整承载当前移动契约，同时保证原生平台的文件路径、安全存储、前后台生命周期、崩溃恢复钩子可接入。

#### Detailed execution checklist

- [ ] 先定义 Flutter 桥接分层，避免页面直接触碰 FFI。
  - Flutter UI layer
  - Flutter app SDK layer
  - platform channel / FFI bridge layer
  - native adapter layer
  - Rust shared core
- [ ] 先决定 Flutter 到 Rust 的技术路线。
  - `dart:ffi` 直连
  - platform channel -> native adapter -> Rust
  - 混合方案
- [ ] 针对当前仓库已有 Android JNI 和 iOS ObjC bridge，评估是否复用 native adapter 层，只替换 React Native JS 调用面。
- [ ] 定义 Flutter 侧最小 SDK 包装接口，不让页面组件直接调用底层桥接。
- [ ] 设计 Flutter 侧状态容器。
  - app bootstrap state
  - today page state
  - study session state
  - reports state
  - wrong-word state
  - AI state
- [ ] 为每类 contract 定义调用包装、错误映射和 logging 入口。
- [ ] 把当前 `mobile-bridge.ts` 的函数族迁移成 Flutter SDK 层的方法族。
- [ ] 统一桥接层序列化策略。
  - 字符串 JSON
  - typed FFI struct
  - binary payload
  - 这一层只允许一种主通道，不要混乱并存
- [ ] 定义桥接层错误模型。
  - Rust domain error
  - Rust runtime/bootstrap error
  - platform adapter error
  - Flutter decode / protocol error
- [ ] 明确生命周期钩子接入点。
  - app bootstrap
  - foreground / background
  - session resume hint
  - app restart restore
- [ ] 设计 Flutter 工程骨架。
  - package layout
  - routing
  - state container
  - theme / i18n entry
- [ ] 为 bridge 建立验证面。
  - smoke harness
  - bootstrap call
  - today read
  - study session round-trip
  - reports/wrong-word read
- [ ] 形成 Slice 4 的桥接文档和 SDK 入口文档。

#### Recommended layering for Slice 4

##### 1. Flutter UI layer

职责：

- 页面展示
- 路由
- 输入交互
- 页面级 loading/error/empty 态

禁区：

- 不能直接处理学习判题
- 不能直接维护会话推进真相
- 不能绕过 SDK 直接打 Supabase 和 Rust 双写

##### 2. Flutter app SDK layer

职责：

- 暴露 Flutter 内部统一 API
- 调 bridge
- 做 typed decode
- 做错误映射
- 做少量 UI 友好包装

建议接口族：

- `bootstrapClient`
- `todayClient`
- `planClient`
- `studyClient`
- `reportsClient`
- `wrongWordsClient`
- `aiClient`
- 后续可加 `authClient` / `syncClient`

##### 3. Bridge / codec layer

职责：

- 唯一负责 Flutter <-> native/Rust 的消息边界
- 编码解码
- 错误码协议
- 超时与协议校验

禁区：

- 不做业务拼装
- 不做 today snapshot 重建
- 不做答题判定

##### 4. Native adapter layer

职责：

- Android / iOS 平台桥接
- 文件路径、app sandbox、生命周期 hook、安全存储调用
- 将平台细节收敛成共享 contract 可接受的输入输出

现有可复用资产：

- Android [RustBridge.java](/d:/projects/word-mobile-rn/apps/mobile/android/app/src/main/java/com/wordmobile/RustBridge.java)
- iOS [WordCoreModule.mm](/d:/projects/word-mobile-rn/apps/mobile/ios/WordMobile/WordCoreModule.mm)
- iOS FFI header [word_platform_mobile_ios.h](/d:/projects/word-mobile-rn/crates/platform-mobile/include/word_platform_mobile_ios.h)

##### 5. Rust shared core

职责：

- 继续提供 contract 语义和领域真相
- 不感知 Flutter 页面结构
- 不感知 platform channel 细节

#### Recommended technical direction

在当前仓库基础上，最稳的路线通常不是让 Flutter 直接首版用 `dart:ffi` 硬怼所有平台差异，而是：

- 先复用现有 native adapter 层思路
- Flutter 调 platform layer
- native adapter 再调 Rust

原因：

- 当前 Android/iOS 已经存在 Rust runtime init、path 解析、symbol 调用逻辑
- 这部分比页面重写更容易出隐性坑
- 先复用 adapter 能减少首版 Flutter bridge 风险

只有在 adapter 层稳定后，才评估是否需要把一部分能力下沉为更直接的 `dart:ffi`

#### Flutter SDK surface proposal

建议 Flutter 侧先形成一层 typed SDK，而不是散落方法：

- `BootstrapClient`
  - `getBootstrapState()`
  - `markOnboardingCompleted()`
- `TodayClient`
  - `getTodayHomeState()`
- `SettingsClient`
  - `getSettings()`
  - second-wave: AI provider config methods
- `PlanClient`
  - `getActivePlan()`
  - `savePlan(...)`
  - `applySavedPlanToToday()`
- `WordbookClient`
  - `getWordbooks()`
  - `toggleWordbook(...)`
- `StudyClient`
  - `startStudySession(...)`
  - `submitStudyAnswer(...)`
  - `completeStudySession(...)`
  - `cancelStudySession(...)`
- `ReportsClient`
  - `getReportsOverview()`
- `WrongWordsClient`
  - `getWrongWords(...)`
  - `getWrongWordDetail(...)`
- `AiClient`
  - second-wave methods only

#### Bridge protocol rules

- 统一一个主编码协议，不要有的接口走 JSON 字符串，有的接口走 ad hoc channel map。
- 所有 bridge 调用都必须带明确方法名和可追踪错误码。
- 所有 bridge 返回都必须在 Flutter SDK 层做 typed decode。
- 解码失败必须落到 `protocol/decode error`，不能假装是业务失败。
- 与平台初始化有关的错误必须和普通业务错误分开。
- 所有 bridge 调用都必须有 debug-safe 日志，但不得泄露 token、secret、用户答案明文。

#### Error model for Slice 4

建议先统一成这几类：

- `domain_error`
  - 例如 study rule violation、invalid state transition
- `runtime_error`
  - Rust runtime 未初始化、数据库不可用、seed 缺失
- `platform_error`
  - Android/iOS adapter 初始化失败、路径不可写、安全存储失败
- `protocol_error`
  - decode 失败、字段缺失、unexpected null
- `unsupported_error`
  - 当前平台未实现

Flutter 页面只消费统一错误码和用户友好描述，不消费 native 原始错误文本。

#### Package / directory suggestion for Flutter side

如果后续建立 Flutter app，建议至少按这种 shape 组织：

```text
apps/flutter_mobile/
  lib/
    app/
      router/
      theme/
      bootstrap/
    sdk/
      bootstrap_client.dart
      today_client.dart
      plan_client.dart
      study_client.dart
      reports_client.dart
      wrong_words_client.dart
      ai_client.dart
    bridge/
      rust_bridge.dart
      bridge_codec.dart
      bridge_error.dart
    features/
      today/
      study/
      plan/
      reports/
      wrong_words/
      ai/
    state/
      app_state.dart
      session_state.dart
```

#### Lifecycle integration checklist

- cold start 时先完成 Rust runtime/bootstrap，再决定进入 onboarding 或 app shell
- app background 时不要让 Flutter 单方面丢失会话引用
- app foreground 时要有显式的 session refresh / restore 路径
- app restart 后恢复依赖本地持久化，而不是 Flutter 内存态
- 生命周期事件只发送 hint，不让 Flutter 自己重建领域真相

#### Bridge verification matrix

- bootstrap smoke
  - Flutter -> bridge -> Rust -> valid bootstrap DTO
- today smoke
  - Flutter -> bridge -> Rust -> valid today DTO
- study smoke
  - start -> submit -> complete round-trip
- report/wrong-word smoke
  - 完成一轮 study 后读取 reports/wrong words
- failure smoke
  - Rust library missing
  - runtime init failure
  - decode failure
  - unsupported platform branch

#### Recommended artifacts for Slice 4

- `docs/flutter/bridge-architecture.md`
- `docs/flutter/flutter-sdk-surface.md`
- `docs/flutter/bridge-error-model.md`
- `docs/flutter/lifecycle-hooks.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 4 documentation for bridge layering, Flutter SDK surface, bridge error taxonomy, and lifecycle hook ownership.
- [x] Froze the v1 transport direction as `platform channel -> native adapter -> Rust`, defined a stable success/error bridge envelope, and clarified startup/session/read/failure smoke paths for downstream slices.
- [x] Slice 4 can be treated as complete at the architecture-contract level: layering is frozen, SDK ownership is frozen, bridge/error protocol is frozen, lifecycle ownership is frozen, and Slice 5/6/7 no longer need to re-decide the Flutter bridge direction.

#### Completion status

- [x] Flutter -> SDK -> bridge -> native adapter -> Rust layering is explicit enough for implementation.
- [x] v1 transport is explicitly chosen as `platform channel -> native adapter -> Rust`.
- [x] Shared success/error envelope and bridge error taxonomy are explicit enough for SDK and feature work.
- [x] bootstrap / today / study / reports / wrong-word smoke paths are defined for downstream implementation.
- [x] lifecycle / resume / restart ownership is explicit enough to prevent Flutter from owning domain truth.
- [x] Slice 5 and Slice 6 can continue without reopening Slice 4 architectural decisions.

#### Exit gate for Slice 4

- [ ] 已确定 Flutter 到 Rust 的首版技术路线，并说明为何选它。
- [ ] 已形成 Flutter SDK 层边界，页面不直接碰桥接细节。
- [ ] 已形成统一 bridge protocol 和错误模型。
- [ ] bootstrap / today / study / reports / wrong-word 的 smoke path 已定义。
- [ ] 生命周期接入点已定义，不依赖 Flutter 内存态维持学习真相。
- [ ] 可以进入 Slice 5 和 Slice 6，而不再反复讨论 bridge 分层。

#### Acceptance criteria

- [ ] Flutter 工程可以在 Android 和 iOS 启动到可见壳层。
- [ ] Flutter 已可通过桥接调用 Rust 的 bootstrap、today、plan、study、reports、wrong words、AI 关键接口。
- [ ] 桥接层有统一的错误码和序列化约定，不把平台异常原样抛给页面。
- [ ] Flutter 侧没有任何学习规则、答题判定、报表聚合的临时复制实现。

#### High-risk pitfalls

- 桥接只追求“能返回 JSON”，忽略错误分类、版本管理、性能和大对象传输。
- 把状态管理写成页面直接调用 FFI，导致后续无法插入缓存、恢复和同步协调。
- Flutter 端因开发便捷临时补了本地逻辑，最后变成第二学习引擎。

#### Avoid

- 不要把 Rust FFI 暴露成全局无边界调用集合。
- 不要让页面组件直接持有 Supabase client 和 Rust FFI 双写。
- 不要在没有契约测试前就批量迁移页面。

---

### Slice 5: 重建本地运行时与账号会话持久层

#### What to build

在 Flutter 版移动端重建本地 SQLite、路径解析、schema migration、加密敏感配置、安全存储 token、本地游客身份和账号会话切换机制。要求登录体系引入后，离线启动、离线学习、重启恢复仍然成立。

#### Detailed execution checklist

- [ ] 先定义 Slice 5 的目标状态机，而不是直接接 login UI。
  - guest local-only
  - signed-in with no cloud data
  - signed-in with cloud data
  - guest bind in progress
  - signed-out but local data retained
  - account deleted / token invalid
- [ ] 明确本地 runtime 的存储分层。
  - SQLite：学习真相、本地业务数据
  - secure storage：auth token / refresh token / device binding secrets
  - lightweight preferences：非敏感 app flags
- [ ] 明确账号接入后哪些内容仍必须本地优先。
  - bootstrap readiness
  - local study continuity
  - resumable session state
  - today snapshot read
- [ ] 设计 Flutter 端的 auth session manager。
  - session read on startup
  - refresh scheduling
  - logout
  - session invalidation
- [ ] 设计 guest identity 与 signed-in identity 的本地映射。
  - guest local profile id
  - cloud user id
  - device id
  - bind marker / migration marker
- [ ] 明确 SQLite schema 是否需要最小增量字段。
  - `user_id` 或本地 owner scope
  - `device_id`
  - sync marker fields
  - data origin marker where necessary
- [ ] 设计首次登录时的 runtime 行为。
  - 不阻塞本地 app 进入
  - 不立刻破坏现有本地数据
  - 如果需要 merge，先进入明确的 bind/merge 流程
- [ ] 设计 logout 行为。
  - 清 token
  - 清 session
  - 是否保留本地学习历史
  - 是否保留云端下发的缓存
- [ ] 设计 token 失效行为。
  - 后台 refresh 失败
  - 前台接口 401
  - 用户被登出后仍能否以本地模式学习
- [ ] 设计 app restart 恢复路径。
  - secure storage session read
  - local runtime bootstrap
  - resumable session discovery
  - account state reattachment
- [ ] 设计 migration 策略。
  - 从“无账号本地模式”升级到“支持账号的本地 runtime”
  - 老版本 SQLite 升级
  - token 存储初始化
- [ ] 把账号相关状态暴露成 Flutter app shell 可消费的 typed state，而不是散在页面内。
- [ ] 明确 Slice 5 的 smoke scenarios。
  - first launch as guest
  - login with empty cloud
  - login with existing cloud
  - logout keep local
  - restart after login
  - expired token + local fallback

#### Runtime storage model

Slice 5 建议按下面三层实现：

##### 1. SQLite

适合放：

- today / plan / study / wrong-word / reports / AI persisted artifacts
- local resumable session snapshots
- future sync queue metadata

不适合放：

- raw auth token
- refresh token
- provider secrets

##### 2. Secure storage

适合放：

- access token
- refresh token
- cloud-session identifier if needed
- secure device binding secret or key material

平台实现位置应保持在 adapter 层，不进 shared core。

##### 3. Non-sensitive preferences

适合放：

- onboarding-complete flag if not already in SQLite truth
- last selected non-sensitive UI preference
- local diagnostics toggles

#### Local identity model

建议在 Slice 5 明确三个 identity 概念：

- `local_profile_id`
  - 本地设备上的逻辑拥有者标识，可服务 guest 模式
- `cloud_user_id`
  - Supabase user id
- `device_id`
  - 当前安装/设备的稳定云端同步标识

这三者不要混成一个字段。

#### Account state machine

##### Guest local-only

特点：

- 无 token
- 可完整本地学习
- 本地数据可持续累积

##### Signed-in active

特点：

- secure storage 中有有效 session
- 本地数据继续是执行真相
- 允许云端同步与账号绑定行为

##### Signed-in but expired

特点：

- 本地学习不应立刻失效
- 云端受保护操作暂停
- 需要 refresh 或回退到明确状态

##### Signed-out retained-local

特点：

- token 已清空
- 本地历史仍存在
- app 可根据产品策略继续进入 guest/local mode

#### Login / bind strategy

Slice 5 必须明确三条路径：

##### Path A: Guest -> empty cloud account

推荐：

- 登录成功
- 建立 cloud identity
- 本地数据成为首个待同步来源

##### Path B: Guest -> existing cloud account

推荐：

- 不自动盲 merge
- 进入 bind/merge decision flow
- 使用 Slice 3 的 merge strategy

##### Path C: Signed-in returning user

推荐：

- 直接恢复 account session
- 本地 runtime 启动后再决定是否触发同步

#### Logout policy

Slice 5 必须写死下面这些问题：

- logout 是否删除 SQLite
  - 默认不删除学习真相
- logout 是否删除 active local session
  - 默认不应因 cloud 登出直接抹掉本地学习历史；是否保留进行中 session 需要显式策略
- logout 是否删除 AI 本地缓存
  - 需要产品决定，但不能隐式混在 token clear 里
- logout 后 app 进入什么模式
  - guest local-only 或 locked-signed-out，由产品明确

#### Token and session rules

- token 只进 secure storage
- token 不能进入 SQLite、debug log、analytics breadcrumb 明文
- Flutter feature code 不直接读原始 token，统一经 auth session manager
- refresh 失败时返回 typed auth state，而不是让任意 feature 各自处理 401

#### Schema migration checklist

- 为现有本地数据增加最小 owner scope 设计
- 迁移必须支持“已有 SQLite + 新 secure storage”
- 迁移失败时不能让已有学习数据不可读
- question-engine / runtime schema migration 与 account-state migration 不能互相混淆

#### Runtime bootstrap with account support

启动顺序建议：

1. 平台路径与 SQLite runtime 准备
2. secure storage session 读取
3. Rust bootstrap
4. account state resolve
5. app shell routing

不要在第 2 步前就依赖网络拿用户资料来决定 app 是否能进入。

#### Verification matrix for Slice 5

- guest cold start
- guest -> login -> restart
- signed-in cold start with valid token
- signed-in cold start with expired token
- logout keep-local
- reinstall without secure storage recovery
- migration from pre-account local database
- active study session across auth-state change boundary

#### Recommended artifacts for Slice 5

- `docs/auth/local-runtime-and-session-manager.md`
- `docs/auth/guest-bind-flow.md`
- `docs/auth/logout-and-retention-policy.md`
- `docs/auth/local-schema-migration-notes.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 5 documentation for local runtime bootstrap, auth session persistence, guest bind behavior, logout retention, and local account-aware migration boundaries.
- [x] Wrote down explicit defaults for `guest_local_only`, `signed_out_retained_local`, token-in-secure-storage-only, bind interruption safety, revoked/deleted account handling, and reinstall device identity posture.

#### Exit gate for Slice 5

- [ ] guest / signed-in / expired / signed-out-retained-local state machine 已明确。
- [ ] token 与 secure storage 边界明确，token 不进入 SQLite。
- [ ] logout / restart / token-expired 行为已明确，不依赖页面猜测。
- [ ] 本地数据保留策略已明确，账号接入不破坏 local-first。
- [ ] 迁移路径已明确，旧本地库升级不会默认丢数据。
- [ ] Slice 6 可以在不重新定义 auth/runtime 状态的前提下继续实现主学习闭环。

#### Acceptance criteria

- [ ] 本地 SQLite 可在 Flutter 新壳下被 Rust 正确打开、迁移和恢复。
- [ ] Auth token、refresh token、设备标识等敏感信息只进入安全存储，不进入普通 SQLite 或明文日志。
- [ ] 支持“未登录本地数据”和“已登录账号数据”的状态切换与识别。
- [ ] 退出登录不会误删学习真相；是否清理云绑定信息与本地学习数据有明确策略。

#### High-risk pitfalls

- 登录完成后切换数据库命名或目录策略不当，导致旧本地数据失联。
- 把 token 存进 SQLite 或调试日志，造成安全和合规问题。
- 将游客本地数据与账号数据共用主键空间却没有命名隔离或映射表。
- schema migration 只测新安装，不测旧版本升级和失败回滚。

#### Avoid

- 不要因为引入 Auth 就推翻现有 SQLite 结构的核心学习表。
- 不要让 logout 默认清空所有本地学习数据。
- 不要把“用户 id 变更”处理成粗暴全量重建库，除非已有完整导出恢复机制。

---

### Slice 6: 迁移 Today / Plan / Study 主学习闭环

#### What to build

把最关键的用户学习闭环迁移到 Flutter：启动、Today、计划读取与编辑、开始学习、提交答案、完成会话、恢复会话。所有页面交互改由 Flutter 承担，但学习流程仍必须由 Rust 驱动，结果写回 SQLite，并由同步层异步上送云端。

#### Detailed execution checklist

- [ ] 先把 Slice 6 拆成三个垂直链路，而不是按页面组件横切迁移。
  - app bootstrap -> today
  - today -> plan read/edit/apply
  - today -> study session -> summary -> back to today
- [ ] 明确这一步只迁主学习闭环，不把 AI、同步诊断、账号设置面混进来。
- [ ] 定义 Flutter 端 Today shell 所需的最小状态。
  - bootstrap loaded
  - today payload loaded
  - resumable session hint
  - plan CTA state
  - start-study CTA state
- [ ] 定义 Flutter 端 Plan shell 所需的最小状态。
  - active plan read
  - local editable draft
  - save in progress
  - apply-to-today result
- [ ] 定义 Flutter 端 Study shell 所需的最小状态。
  - current session envelope
  - current question
  - submit pending
  - summary ready
  - cancel confirmation
- [ ] 为 Today 链路定义 contract boundary。
  - `getBootstrapState`
  - `getTodayHomeState`
  - resumable-session indicator if later added
- [ ] 为 Plan 链路定义 contract boundary。
  - `getActivePlan`
  - `savePlan`
  - `applySavedPlanToToday`
- [ ] 为 Study 链路定义 contract boundary。
  - `startStudySession`
  - `submitStudyAnswer`
  - `completeStudySession`
  - `cancelStudySession`
- [ ] 定义 Flutter UI 允许做的 optimistic behavior 和禁止做的领域行为。
- [ ] 设计 Flutter 与 Rust 的状态同步原则。
  - UI state is ephemeral
  - Rust/local persistence is authoritative
  - navigation is not domain truth
- [ ] 定义 Today 返回后的 refresh policy。
  - 何时重新读 `getTodayHomeState`
  - 何时只更新 UI 层局部状态
  - 何时必须读取 Rust persisted truth
- [ ] 定义 session summary 完成后的回流路径。
  - complete session
  - read updated today
  - read updated reports/wrong-word indicators if needed later
- [ ] 把 Slice 6 拆成 smoke scenarios。
  - bootstrap to today
  - today to plan save
  - today to study and back
  - plan edit same-day snapshot stability
  - study cancel
  - study resume after restart

#### Vertical slice structure

##### Slice 6A: Bootstrap to Today

目标：

- Flutter app 冷启动后进入 today shell
- today payload 完全来自 Rust

职责：

- Flutter 负责启动界面与 loading/error 表达
- Rust 负责 bootstrap truth 和 today truth

##### Slice 6B: Today to Plan

目标：

- 用户可以读 active plan、编辑计划、保存计划、决定是否 apply 到 today

职责：

- Flutter 负责 form 和交互
- Rust 负责 plan 持久化和 today-application 语义

##### Slice 6C: Today to Study

目标：

- 用户从 today 发起任一学习模式，完成答题、完成会话、回到 today

职责：

- Flutter 负责 question/card/input UI
- Rust 负责 session lifecycle、answer evaluation、summary、wrong-word/report side effects

#### Today migration rules

- Today 页面绝不能在 Dart 端重新拼 snapshot。
- Today 允许本地展示 loading skeleton，但不得以 skeleton 默认值推导任务数。
- Today 的 `nextRecommendedAction`、targets、carryover、completed 只能来自 Rust 返回。
- Today 返回后若需要刷新，必须重新读取 authoritative today payload，而不是根据上一页动作自行推断。
- 若后续加入 resumable-session hint，该 hint 也必须基于 Rust persisted truth。

#### Plan migration rules

- Plan 页面可以维护本地 draft，但 draft 不是真相。
- `savePlan` 只修改计划模板，不自动重算当日 snapshot，除非 contract 明确允许。
- `applySavedPlanToToday` 的语义必须被用户和代码显式区分于单纯 `savePlan`。
- Growth rule 编辑逻辑不能在 Flutter 端重新解释。
- Flutter 可以做表单校验，但不能替代 Rust 的计划语义校验。

#### Study migration rules

- Flutter question card 只负责展示当前 question 和收集 response。
- `submitStudyAnswer` 的结果必须驱动 UI 后续状态，UI 不能预判正确性。
- `isComplete/currentQuestion/summary/progress` 四者关系完全以 Rust 返回为准。
- `completeStudySession` 与 `cancelStudySession` 必须保持严格不同语义。
- Flutter 不得自己累计 correct/incorrect/skipped 作为唯一 summary 真相。
- 复习、混合测、错词强化、root/affix 的模式差异都必须继续由 Rust 决定。

#### UI-vs-domain responsibility table

| Concern | Flutter UI | Rust / local truth |
|---|---|---|
| loading skeleton | yes | no |
| button enabled/disabled | yes | no |
| question correctness | no | yes |
| session progress truth | no | yes |
| today snapshot truth | no | yes |
| local draft form state | yes | no |
| summary counts | no | yes |
| navigation after success | yes | no |

#### Allowed optimistic behavior

可以：

- 按钮进入 pending 态
- 提交后短暂禁用重复点击
- 保存计划时显示 optimistic saving UI

不可以：

- 提交答案前预判 correct / incorrect
- 直接在 Today 上先减 target 或加 completed 再等 Rust 覆盖
- 在 summary 前先本地算出成绩
- 在 plan 保存后直接本地重算当天 today snapshot

#### Today refresh policy

推荐规则：

- cold start: always `getTodayHomeState`
- plan saved but not applied: keep current today snapshot UI, refresh active plan if needed
- plan applied to today: re-read `getTodayHomeState`
- session completed: re-read `getTodayHomeState`
- session cancelled: re-read only if Rust-side cancel semantics may affect resumable/session indicator; otherwise keep stable
- app resume after possible state drift: prefer authoritative re-read

#### Main smoke flows for Slice 6

##### Flow 1: Bootstrap -> Today

1. app starts
2. bootstrap succeeds
3. today payload loads
4. user sees targets and next action

##### Flow 2: Today -> Plan -> Save

1. user enters plan
2. reads active plan
3. edits targets / growth rules
4. saves plan
5. same-day today snapshot remains stable unless apply flow is used

##### Flow 3: Today -> Study -> Complete -> Today

1. user starts mode from today
2. question renders
3. answer submitted
4. summary returned
5. complete session acknowledged
6. today refreshes from Rust truth

##### Flow 4: Today -> Study -> Cancel

1. user starts session
2. user cancels
3. local and UI state honor cancel semantics
4. no fake completion summary shown

##### Flow 5: Restart -> Resume -> Complete

1. session in progress
2. app restart
3. Flutter bootstrap + runtime restore
4. session resumes from persisted truth
5. session completes with no duplicated results

#### Verification matrix for Slice 6

- `bootstrap -> today`
  - appReady route
  - onboarding route
  - startup error route
- `today`
  - targets/progress align with Rust payload
  - no local recomputation drift
- `plan`
  - save plan preserves plan semantics
  - apply-to-today path is distinct
  - same-day snapshot stability preserved
- `study`
  - correct/fuzzy/incorrect/skipped all render correctly
  - summary driven by Rust response
  - cancel and complete are distinct
- `return to today`
  - updated payload comes from authoritative read
- `restart`
  - resumed session state does not duplicate or regress progress

#### Recommended artifacts for Slice 6

- `docs/flows/today-plan-study-main-loop.md`
- `docs/flows/today-refresh-policy.md`
- `docs/flows/study-ui-vs-domain-boundary.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 6 documentation for the main Today -> Plan -> Study loop, authoritative today refresh rules, and the Study UI vs Rust domain boundary.
- [x] Froze the key migration decisions that downstream implementation depends on: `savePlan` vs `applySavedPlanToToday`, authoritative Today re-read rules, cancel vs complete distinction, and restart/resume rebuilding from Rust truth.

#### Completion status

- [x] Bootstrap / Today / Plan / Study main-loop ownership is explicit.
- [x] Today refresh triggers are explicit enough to prevent Flutter-side snapshot drift.
- [x] Study UI optimistic behavior boundaries are explicit.
- [x] Slice 7 can extend reports/wrong-word/AI surfaces without reopening Slice 6 core loop decisions.

#### Exit gate for Slice 6

- [ ] Today / Plan / Study 三条链路的 contract boundary 已冻结。
- [ ] Today refresh policy 已明确，Dart 不重算 snapshot。
- [ ] Plan save 与 apply-to-today 的语义已明确分离。
- [ ] Study 的 correctness / progress / summary 真相已明确继续由 Rust 持有。
- [ ] 完整主学习闭环 smoke flows 已定义。
- [ ] Slice 7 可在此基础上扩展错词本、报表、AI，而不重写主闭环边界。

#### Acceptance criteria

- [ ] 用户可从 Flutter Today 页进入任一学习模式并完成完整答题闭环。
- [ ] 学习过程中答案保密、提交后反馈、会话总结、错词写入、报表更新都与基线一致。
- [ ] 前后台切换、应用重启、网络中断不会破坏进行中的 Rust 会话状态。
- [ ] 计划编辑仍保持 `today snapshot` 的稳定规则，不因页面更新直接污染当日已生成计划。

#### High-risk pitfalls

- 为了配合 Flutter UI 重写 Today 组装逻辑，导致 target、carryover、next action 偏离 Rust 真相。
- 页面上做 optimistic 更新但没有与 Rust 最终结果对齐，出现进度闪烁或错账。
- 会话恢复逻辑分散在 Flutter、Rust、本地库三处，最终谁都不完全负责。
- 计划编辑完成后直接重算当天快照，破坏现有“今日稳定”的产品约束。

#### Avoid

- 不要在 Dart 中重新拼 today snapshot。
- 不要让页面自己维护“当前题号”和“答题结果统计”作为唯一真相。
- 不要因为迁移新 UI 而放松对 `skip / cancel / resume / app restart` 的验证。

---

### Slice 7: 迁移错词本、报表、AI，以及云同步语义

#### What to build

在主学习闭环稳定后，再迁移错词详情、报表页面、AI 文章能力，并定义这些数据如何进入同步系统。报表要继续以 Rust 聚合为准，AI 仍是非阻塞增强能力，云端只承接同步与存档，不把 AI 或报表逻辑拆散到前端或 SQL。

#### Detailed execution checklist

- [ ] 先把 Slice 7 拆成三条子链路，而不是把“学习后功能”揉成一坨。
  - wrong words
  - reports
  - AI passages/history
- [ ] 明确 Slice 7 的前置条件。
  - Slice 6 主学习闭环已成立
  - wrong-word/report side effects 已经在 Rust 正确写入
  - Flutter bridge 已能稳定读取 contract
- [ ] 为 wrong-word 链路定义 contract boundary。
  - `getWrongWords`
  - `getWrongWordDetail`
- [ ] 为 reports 链路定义 contract boundary。
  - `getReportsOverview`
- [ ] 为 AI 链路定义 contract boundary。
  - `getTodayAiPassageContext`
  - `generateAiPassage`
  - `getAiPassageHistory`
  - `getAiPassage`
  - `saveAiPassage`
- [ ] 定义 Flutter 端 wrong-word 页面所需最小状态。
  - filter
  - list data
  - selected detail
  - loading/error states
- [ ] 定义 Flutter 端 reports 页面所需最小状态。
  - overview payload
  - selected chart range/scroll position
  - loading/error states
- [ ] 定义 Flutter 端 AI 页面所需最小状态。
  - today context
  - current passage
  - history list
  - generation pending/error state
- [ ] 明确每条链路里什么属于“展示层增强”，什么属于“领域真相”。
- [ ] 为云同步语义定义第一版分层。
  - wrong-word 是同步事件还是同步投影
  - reports 是云缓存还是本地重建
  - AI 历史是同步实体还是仅本地缓存
- [ ] 为 AI 定义性能和非阻塞规则。
  - 不阻塞 today
  - 不阻塞 study completion
  - 失败后有清晰可见状态
- [ ] 定义 Slice 7 的 smoke scenarios。
  - study -> wrong words visible
  - study -> reports updated
  - AI missing config
  - AI generation success/failure
  - AI history read after save

#### Vertical slice structure

##### Slice 7A: Wrong Words

目标：

- Flutter 可展示 wrong-word 列表与详情
- 列表与详情完全来自 Rust persisted truth

职责：

- Flutter 负责 filter、列表布局、详情展开
- Rust 负责 wrong-word state、riskBreakdown、error history 语义

##### Slice 7B: Reports

目标：

- Flutter 可展示 reports overview、daily series、mode breakdown
- 图表表达可以变化，但聚合结果不能漂

职责：

- Flutter 负责 chart layout、scroll、视觉表达
- Rust 负责 aggregate truth

##### Slice 7C: AI

目标：

- Flutter 可读取 today AI context、生成 passage、查看历史、查看详情
- AI 故障不影响主学习闭环

职责：

- Flutter 负责生成按钮、历史列表、阅读体验
- Rust 负责 AI 请求编排、结果结构、validation 状态、持久化入口

#### Wrong-word migration rules

- Wrong-word list 和 detail 必须来自 Rust persisted truth，不从 Flutter 内存态推导。
- `riskBreakdown`、`errorHistory`、`priorityScore` 必须保持 Rust-owned 语义。
- Flutter 允许增加筛选和展开交互，但不得重算 wrong-word 风险。
- 如果后续 wrong-word 同步进入云端，首版仍应以本地/Rust 投影为主展示真相。

#### Reports migration rules

- 报表聚合必须继续由 Rust 提供，Flutter 只消费结果。
- 图表坐标、标签、动画、滚动属于 Flutter UI，可重做。
- `overallAccuracy`、`streakInfo`、`modeBreakdown`、`dailySeries` 不得由 Flutter 重新聚合。
- 若将来引入云端 reports cache，也不能覆盖本地/Rust 已知真相。

#### AI migration rules

- AI 是可选增强，不得阻塞 today 和 study。
- `getTodayAiPassageContext` 是辅助上下文，不是主学习链路前置条件。
- `generateAiPassage` 成功/失败都必须是可见但非阻塞状态。
- AI 历史和 passage 阅读可以缓存，但不能要求在线成功后才能继续学习。
- provider/fallback/secret 仍应由 Rust/adapter/cloud 负责，Flutter 不直接持有 provider secret。

#### UI-vs-domain boundary for Slice 7

| Concern | Flutter UI | Rust / local truth |
|---|---|---|
| wrong-word filter chips | yes | no |
| wrong-word priority score | no | yes |
| reports chart layout | yes | no |
| reports aggregate values | no | yes |
| AI generation button pending state | yes | no |
| AI validation status semantics | no | yes |
| AI reading layout | yes | no |

#### Cloud sync semantics for Slice 7

##### Wrong words

推荐：

- 同步 source events 或可验证 projection
- 不要以 Flutter 当前显示列表作为云端更新来源

##### Reports

推荐：

- 优先本地/Rust 聚合
- 云端如需存 cache，也标记为 derived projection

##### AI passages

推荐：

- 将 passage/history 作为用户内容实体同步
- validation status 一并持久化
- generation failure 不需要写成会阻塞主链路的强一致状态

#### Allowed UI enhancement

可以：

- 更丰富的 wrong-word detail 布局
- 更顺手的 reports 图表交互
- AI 阅读视图的排版优化
- 筛选、排序、tab、折叠交互

不可以：

- Flutter 本地重算 wrong-word risk
- Flutter 本地重算 reports aggregate
- Flutter 直接发 provider 请求绕过 Rust

#### Main smoke flows for Slice 7

##### Flow 1: Study -> Wrong Words

1. user completes a session with incorrect/skipped answers
2. wrong-word list reflects updated persisted truth
3. wrong-word detail shows matching risk/history

##### Flow 2: Study -> Reports

1. user completes a session
2. reports overview updates
3. chart and aggregate values align with Rust truth

##### Flow 3: Today -> AI generate success

1. user opens AI surface
2. today AI context loads if available
3. generation succeeds
4. passage saves and appears in history

##### Flow 4: Today -> AI generate failure

1. AI request fails
2. failure is visible
3. user can leave AI surface and continue normal study loop

#### Verification matrix for Slice 7

- wrong-word list correctness
  - filter behavior
  - detail fetch behavior
  - riskBreakdown fidelity
- reports correctness
  - aggregate values align with baseline
  - chart layout can vary without value drift
- AI correctness
  - context read
  - generation success
  - generation failure
  - history persistence
- non-blocking behavior
  - today and study still work when AI is unavailable

#### Recommended artifacts for Slice 7

- `docs/flows/wrong-words-and-reports-extension.md`
- `docs/flows/ai-non-blocking-behavior.md`
- `docs/flows/reports-ui-vs-aggregate-boundary.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 7 documentation for wrong-word and reports surface extension, reports UI vs aggregate boundaries, and AI non-blocking behavior.
- [x] Froze the key downstream decisions: wrong-word/report reads stay Rust-backed, reports remain presentation-only on the Flutter side, AI remains optional and failure-isolated, and Study side effects become visible through authoritative follow-up reads rather than Flutter-local recomputation.

#### Completion status

- [x] wrong-word / reports / AI contract families are explicit enough for feature implementation.
- [x] Flutter UI vs Rust truth ownership is explicit for all three surfaces.
- [x] Study -> wrong-word/report visibility chain is defined through persisted truth, not local guesses.
- [x] Slice 8 can plan sync around persisted artifacts and projections without redefining Slice 7 ownership.

#### Exit gate for Slice 7

- [ ] wrong-word、reports、AI 三条链路的 contract boundary 已冻结。
- [ ] wrong-word 与 reports 的领域真相仍由 Rust 提供。
- [ ] AI 已明确为非阻塞增强，不反向阻塞主学习闭环。
- [ ] 云同步语义已初步定义，不把 Flutter 展示态误当成同步真相。
- [ ] Slice 8 可以在此基础上接入真正的同步机制，而不重写这三块的真相边界。

#### Acceptance criteria

- [ ] Flutter 版错词本、报表、AI 页面可读取 Rust 提供的真实数据而非页面二次计算。
- [ ] 错词风险、错误历史、报表聚合与当前基线结果一致。
- [ ] AI 失败、网络失败、云端失败都呈现为可见但不阻塞学习的状态。
- [ ] 明确哪些 AI 历史和报表相关数据需要同步，哪些只保留本地缓存。

#### High-risk pitfalls

- 在 Flutter 为图表方便重新做一套报表聚合。
- AI 走 Supabase/Edge 后把主线程等待链拉长，拖慢 Today 或 Study。
- 错词详情在云端和本地同时维护可编辑状态，造成双写冲突。

#### Avoid

- 不要让图表组件反向定义报表数据结构。
- 不要把 AI provider 秘钥直接放进 Flutter 客户端。
- 不要把“AI 历史可同步”扩大成“AI 必须在线可用”。

---

### Slice 8: 实装同步引擎与冲突处理

#### What to build

在功能闭环稳定后，才引入真正的多设备同步。同步引擎应由 Rust 管理 `outbox`、拉取游标、幂等写入、冲突解决、失败重试和设备登记。优先同步计划、词书偏好、学习事件、错词状态和 AI 历史，不直接同步页面状态。

#### Detailed execution checklist

- [ ] 先定义 Slice 8 的同步原则，不直接开始写网络代码。
  - local write first
  - sync async later
  - idempotent cloud ingest
  - no sync success requirement for local study correctness
- [ ] 明确同步单元是什么。
  - config row
  - event
  - projection
  - artifact
- [ ] 为每个同步域定义同步策略。
  - plan configs
  - wordbook preferences
  - study events
  - wrong-word projections or source events
  - AI passages/history
- [ ] 设计本地同步状态结构。
  - outbox
  - pending retries
  - cursor/checkpoint
  - dead-letter
  - per-device sync watermark
- [ ] 设计同步触发器。
  - login / bind complete
  - app foreground
  - post-study completion
  - manual retry
  - background-safe scheduled attempt if later supported
- [ ] 设计 push pipeline。
  - read pending local records
  - batch serialize
  - send with idempotency keys
  - mark success/failure
- [ ] 设计 pull pipeline。
  - read remote changes since cursor
  - merge into local truth safely
  - advance cursor only after successful apply
- [ ] 设计冲突处理策略，不让每个域随意 last-write-wins。
- [ ] 设计失败隔离。
  - per-record failure
  - batch partial success
  - auth failure
  - network failure
  - schema/protocol drift failure
- [ ] 明确 Flutter 在同步中的角色。
  - show status
  - allow retry
  - never become sync queue owner
- [ ] 明确 Rust 在同步中的角色。
  - queue ownership
  - merge ownership
  - idempotency ownership
  - cursor advancement ownership
- [ ] 形成 Slice 8 的观测和调试面。
  - sync status summary
  - pending count
  - last success
  - last error code
  - dead-letter visibility for dev/support only

#### Sync engine model

建议同步引擎至少包含这些逻辑组件：

##### 1. Outbox manager

职责：

- 跟踪本地待上传记录
- 保证不会因为 app 重启丢失待同步项
- 按域和优先级拉取批次

##### 2. Push worker

职责：

- 把 outbox 项转成云端写入
- 附上 `idempotency_key`
- 记录成功、重试、失败

##### 3. Pull worker

职责：

- 从 Supabase 拉取自 cursor 之后的远端变化
- 交给 merge layer
- 成功后更新 cursor

##### 4. Merge layer

职责：

- 按域应用 Slice 3 定义的 merge strategy
- 处理 config/event/projection 不同逻辑

##### 5. Sync status reporter

职责：

- 暴露给 Flutter 的只读同步状态
- 不暴露实现细节或 service-role 逻辑

#### Local sync state tables / metadata

即使字段最后不完全一样，Slice 8 也应至少有这些本地状态概念：

- `sync_outbox`
  - `id`
  - `domain`
  - `payload_json`
  - `idempotency_key`
  - `created_at`
  - `attempt_count`
  - `last_attempt_at`
  - `status`
- `sync_cursor_state`
  - `user_id`
  - `device_id`
  - `last_pushed_at`
  - `last_pulled_cursor`
  - `updated_at`
- `sync_dead_letter`
  - failed payload
  - failure code
  - last failure time

#### Domain sync strategy summary

##### Plan configs

推荐：

- row-style sync
- version-aware replace/merge

##### Wordbook preferences

推荐：

- row-style sync
- lightweight last-write-wins with source attribution

##### Study events

推荐：

- append-only event sync
- idempotent ingest mandatory

##### Wrong-word state

推荐：

- 优先同步 source events 或可重建依据
- projection 同步如果存在，也应视为 secondary

##### Reports

推荐：

- 不把 reports aggregate 当作主同步对象
- 可由事件重建或从云端 projection cache 拉取辅助展示

##### AI passages

推荐：

- artifact-style sync
- keep stable ids and validation metadata

#### Trigger policy

##### Immediate trigger candidates

- after login/bind succeeds
- after successful study completion
- after plan config mutation
- after wordbook preference mutation
- after AI passage save

##### Deferred trigger candidates

- app foreground
- manual sync retry
- scheduled background-safe run if platform support matures later

##### Non-trigger events

- typing in plan draft
- navigating between screens
- mid-question UI state changes

#### Conflict handling rules

- `study_events`
  - never resolve by overwrite
  - always append + deduplicate by idempotency key
- `plan_configs`
  - explicit version or policy-based overwrite
- `wordbook_preferences`
  - allow simpler overwrite rule
- `wrong_word_entries`
  - avoid trusting projection overwrite when event source exists
- `report_snapshots`
  - treat as derived cache, not final truth

#### Failure isolation rules

- failed push of one outbox record must not block local study flow
- failed sync of one domain should not automatically block unrelated domains
- auth/session failure should pause cloud operations, not destroy local queue immediately
- decode/schema drift should move records to dead-letter or explicit support state instead of infinite retry

#### Flutter-facing sync status

Flutter 最多消费这些只读状态：

- `syncEnabled`
- `lastSyncSucceededAt`
- `pendingUploadCount`
- `lastSyncErrorCode`
- `accountSyncState`

Flutter 不应消费：

- raw SQL payloads
- service-role details
- merge internals

#### Main smoke flows for Slice 8

##### Flow 1: Local write -> deferred push

1. user completes study locally
2. local persistence succeeds
3. outbox item created
4. later push succeeds
5. local state remains correct regardless of network timing

##### Flow 2: Duplicate push retry

1. push attempt times out after server may have ingested
2. retry uses same idempotency key
3. cloud does not double-count event

##### Flow 3: Pull after second device change

1. device B changes plan preference
2. device A pulls remote change after cursor
3. local state updates using merge policy

##### Flow 4: Auth failure during sync

1. token expires
2. sync push fails
3. local queue remains intact
4. user can still use local study flow

#### Verification matrix for Slice 8

- push success
- push retry with idempotency
- partial batch failure
- dead-letter path
- pull + cursor advance
- pull + merge conflict
- expired auth during sync
- offline queue buildup and later drain
- multi-device plan preference update
- study event duplication defense

#### Recommended artifacts for Slice 8

- `docs/sync/outbox-and-cursor-model.md`
- `docs/sync/domain-sync-strategy.md`
- `docs/sync/conflict-and-idempotency-rules.md`
- `docs/sync/flutter-sync-status-surface.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 8 documentation for outbox/cursor/dead-letter modeling, domain-specific sync strategy, conflict and idempotency rules, and Flutter-facing sync status boundaries.
- [x] Froze the key sync decisions: local-write-first, domain-routed queue semantics, append-plus-idempotency for study history, projection-as-secondary truth, and Flutter as sync-status reader rather than sync owner.

#### Completion status

- [x] outbox / cursor / dead-letter model is explicit enough for implementation
- [x] domain-by-domain sync strategy is explicit
- [x] idempotency and conflict posture are explicit
- [x] Flutter-facing sync status is explicit without exposing queue internals

#### Exit gate for Slice 8

- [ ] outbox / cursor / dead-letter model 已明确。
- [ ] 每个主要同步域都有明确同步策略。
- [ ] idempotency 与 conflict handling 已明确，不再依赖笼统 last-write-wins。
- [ ] sync failure 不阻塞本地学习闭环的原则已在实现合同中固定。
- [ ] Flutter 只读同步状态面已明确，不接管同步队列。
- [ ] 可以进入 Slice 9 的切换与发布策略，而不会对同步边界再做大改。

#### Acceptance criteria

- [ ] 本地写入先成功，再异步入队同步，不因云端失败阻塞学习。
- [ ] 同一账号多设备可完成基础数据同步，并有明确冲突规则。
- [ ] 存在可审计的 `last_synced_at / cursor / retry / dead-letter` 机制。
- [ ] 账号首次绑定、旧设备追平、新设备冷启动、断网补传、重复上送都可验证。

#### High-risk pitfalls

- 把同步做成“每张表 last-write-wins”后直接上线，忽略学习事件的时序语义。
- 使用设备本地时间作为唯一冲突判据，导致跨时区或时钟漂移错乱。
- 没有幂等 key，重试后重复写入学习事件，报表和错词都被放大。
- 同步层直接调用页面状态，导致后台任务和前台行为耦合。

#### Avoid

- 不要在第一版同步里上传整个 SQLite 文件。
- 不要让 Flutter 自己写同步队列。
- 不要把同步成功作为页面进度正确显示的前置条件。

---

### Slice 9: 切换发布、回滚策略与 RN 退场

#### What to build

制定 Flutter 新壳替换 RN 的切换方案，包括内测、灰度、数据迁移、回滚、指标、崩溃诊断和 RN 退场时机。理想架构即便不考虑重构难度，也不能忽略切换期的数据安全和回滚能力。

#### Detailed execution checklist

- [ ] 先定义切换目标，不把“Flutter 能运行”误当成“可以替换 RN”。
  - 主学习闭环完整
  - local-first continuity 不退化
  - auth/session 行为稳定
  - sync 行为达到计划范围内可接受水平
- [ ] 明确切换阶段。
  - internal smoke
  - limited beta / internal users
  - broader staged rollout
  - mainline replacement
  - RN decommission
- [ ] 明确每一阶段的 gate 和 blocker。
- [ ] 设计回滚策略，不依赖手工修库。
- [ ] 设计版本兼容和数据兼容策略。
  - old RN runtime -> new Flutter runtime
  - new Flutter runtime -> rollback to RN if needed
- [ ] 明确观测面。
  - crash rate
  - cold start success
  - bootstrap failure rate
  - today load success
  - study completion success
  - sync failure rate
  - auth session restore failure
- [ ] 明确 release channel。
  - internal/dev
  - beta/TestFlight/internal testing
  - staged production
- [ ] 设计 support/diagnostics 面。
  - version/build display
  - last sync status
  - runtime diagnostics summary
  - recovery-safe error visibility
- [ ] 明确 RN 与 Flutter 共存边界。
  - 并存期是否允许双壳发版
  - 哪一个是唯一继续加功能的前线
  - 何时停止 RN feature work
- [ ] 定义 RN 退场条件。
  - Flutter 已通过全部发布 gate
  - rollback 窗口结束
  - 本地数据兼容验证通过
  - support/ops 已切换

#### Release stage model

##### Stage 1: Internal verification

目标：

- 工程团队和内部测试设备验证基本路径

必须通过：

- bootstrap
- today
- plan
- study
- wrong words
- reports
- optional AI paths

##### Stage 2: Limited beta

目标：

- 小范围真实设备和真实账户环境验证

必须关注：

- restart and resume
- auth edge cases
- local data continuity
- initial sync correctness

##### Stage 3: Staged rollout

目标：

- 逐步扩大真实用户覆盖

必须关注：

- crash trends
- upgrade migration failure rate
- rollback readiness

##### Stage 4: Mainline replacement

目标：

- Flutter 成为默认移动客户端

必须满足：

- Flutter 达到功能与稳定性 gate
- RN 只保留回滚和收尾职责

##### Stage 5: RN decommission

目标：

- 停止 RN 作为活跃移动客户端

必须满足：

- rollback window 结束
- 数据兼容没有已知高风险
- 运维和支持流程完成切换

#### Release gates

##### Functional gate

- bootstrap to today works
- plan read/save/apply works
- study complete/cancel/resume works
- wrong words and reports match baseline
- AI remains non-blocking

##### Continuity gate

- local SQLite migration succeeds
- restart restore succeeds
- logout keep-local policy behaves as defined
- guest/sign-in transitions behave as defined

##### Sync gate

- outbox persists across restart
- duplicate delivery does not double-count
- pull/apply does not corrupt local truth
- sync failure does not block local study

##### Operational gate

- crash-safe error surfaces exist
- runtime diagnostics are visible enough for support
- version/build/update information is visible

#### Rollback strategy

##### Principle

- rollback is a product path, not a developer-only emergency ritual

##### Required properties

- no destructive one-way migration without tested fallback
- old client must not misread new critical local data silently
- rollback decision can be made from observable signals

##### Rollback scenarios

- Flutter release causes severe bootstrap failures
- Flutter release causes resumable-session corruption
- Flutter release causes unexpected sync amplification or duplication
- auth/session restore failure exceeds acceptable threshold

##### Rollback requirements

- preserve local SQLite readability or provide explicit compatibility handling
- preserve secure-storage safety
- preserve ability to re-enter local mode if cloud/auth paths fail

#### RN coexistence rules

- RN and Flutter may overlap for delivery, but not indefinitely for feature ownership
- once Flutter becomes the forward path, new mobile feature work should stop landing in RN except critical rollback support
- contract drift between RN and Flutter must not be allowed during coexistence

#### RN decommission criteria

- no active production-blocking Flutter defects remain in core loop
- rollback window expires without major regression
- support docs and diagnostics point to Flutter path
- release channels no longer depend on RN build artifacts

#### Observability checklist

- crash rate by build/version
- bootstrap failure category counts
- today load success/failure
- study start/submit/complete success rates
- resume-after-restart success rate
- auth restore failure rate
- sync queue growth and dead-letter rate

#### Support and diagnostics checklist

- app displays installed version and build
- app can show last successful sync timestamp when sync is enabled
- app can show high-level runtime status without exposing secrets
- support can distinguish:
  - bootstrap failure
  - auth failure
  - sync failure
  - study/runtime corruption

#### Main rollout smoke scenarios

##### Flow 1: Upgrade RN user to Flutter build

1. existing RN user upgrades
2. local runtime migrates safely
3. today/study continuity preserved

##### Flow 2: Flutter release with account user

1. signed-in user upgrades
2. secure session restore works
3. local + cloud state remain coherent

##### Flow 3: Rollback after bad Flutter rollout

1. Flutter build shows critical regression
2. rollout halts
3. rollback path exercised
4. user local data remains intact

#### Verification matrix for Slice 9

- upgrade migration from RN runtime
- fresh install on Flutter runtime
- rollback dry run
- staged rollout metrics review
- crash/diagnostic visibility review
- store-ready packaging and release notes/update surface review

#### Recommended artifacts for Slice 9

- `docs/release/flutter-rollout-stages.md`
- `docs/release/rollback-strategy.md`
- `docs/release/rn-decommission-checklist.md`
- `docs/release/observability-and-support-matrix.md`

#### Execution update (2026-04-22)

- [x] Completed first-pass Slice 9 documentation for rollout stages, rollback strategy, RN decommission criteria, and observability/support requirements.
- [x] Froze the release-side decisions that depend on prior slices: staged promotion gates, rollback as a product path, RN coexistence as temporary only, and runtime/auth/sync/support visibility as go/no-go inputs.

#### Completion status

- [x] rollout stages and promotion gates are explicit
- [x] rollback path is explicit and data-safe
- [x] RN coexistence and decommission rules are explicit
- [x] release/support observability is explicit

#### Exit gate for Slice 9

- [ ] rollout stages and gates are explicit
- [ ] rollback path is explicit and data-safe
- [ ] RN coexistence and decommission rules are explicit
- [ ] release/support observability is explicit
- [ ] project can move from architecture planning into release-oriented execution without hidden rollout assumptions

#### Acceptance criteria

- [ ] 存在明确的切换门槛：功能对齐、学习基线通过、离线恢复通过、同步正确性通过。
- [ ] 有独立的回滚方案，保证切换失败时不会破坏本地学习数据。
- [ ] RN 与 Flutter 的并存期被压缩到最短，并明确只允许一个移动壳继续演进。
- [ ] 发布前具备崩溃、启动、同步失败、登录失败、数据迁移失败的观测能力。

#### High-risk pitfalls

- 过早下线 RN，导致 Flutter 新壳还没覆盖所有主学习路径。
- 没有回滚方案就进行数据库迁移或账号绑定迁移。
- 切换期同时维护两套移动客户端并持续加功能，造成契约飘移。

#### Avoid

- 不要把“已接通 Supabase 登录”误当成可以发布的标志。
- 不要在没有完整迁移验证前删除 RN 的参考实现。
- 不要让回滚依赖手工修库或人工 SQL 介入。

## Execution order

- [ ] 先完成 Slice 1，冻结基线与迁移判题器。
- [ ] 再完成 Slice 2，收敛 Rust 边界与正式契约。
- [ ] 之后并行推进 Slice 3 和 Slice 4，但不得绕过 Slice 2 的契约边界。
- [ ] Slice 5 完成本地运行时和账号层后，才能进入 Slice 6 的主学习闭环迁移。
- [ ] Slice 7 必须建立在 Slice 6 稳定通过之后。
- [ ] Slice 8 只在本地闭环稳定后开启，避免“边修业务边修同步”。
- [ ] Slice 9 是切换门，不是补救门；未过前序验证不得切换主客户端。

## Verification strategy

- [ ] 对每个学习模式执行基线回放，对比 Today、Session、Wrong Words、Reports 的结构化结果。
- [ ] 对 `cold start / offline start / app restart / background-foreground / interrupted session` 建立固定冒烟脚本。
- [ ] 对 `signup / login / logout / token refresh / guest bind / device switch / account deletion` 建立账号生命周期验证。
- [ ] 对 `duplicate upload / partial failure / retry / stale cursor / conflict merge` 建立同步专项测试。
- [ ] 对 Supabase RLS 建立正反向用例，验证越权访问被拒绝。
- [ ] 对 Flutter 壳建立性能门槛，确保 Today 首屏和 Study 提交链路不因桥接或云端接入明显变慢。

## Recommended first execution slice

优先做 Slice 1。

如果这次重构没有先把“现有学习真相”冻结成迁移判题器，那么后续无论 Flutter 外壳多漂亮、Supabase 接入多完整，都无法证明你保住了当前产品最值钱的部分。

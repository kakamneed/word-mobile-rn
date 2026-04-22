# Plan: Flutter Rust Supabase Rearchitecture Blueprint

> Status: `in_progress`
> Mode: `plan_only`
> Progress: `not_started`
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

- [ ] 盘点 Slice 1 需要冻结的真相范围，并明确“强一致”和“允许变化”边界。
  - 强一致: Today snapshot 字段语义、会话推进、答题判定、错词写入、报表聚合、会话恢复、skip/cancel/complete 语义。
  - 允许变化: 页面布局、文案微调、动画、加载态展示方式、按钮位置。
- [ ] 建立基线样本目录和命名规则。
  - 推荐最小结构:
  - `tests/baseline/bootstrap/`
  - `tests/baseline/today/`
  - `tests/baseline/study/`
  - `tests/baseline/wrong-words/`
  - `tests/baseline/reports/`
  - `tests/baseline/recovery/`
  - `tests/baseline/ai/`
- [ ] 固定基线运行前提，确保每次回放环境一致。
  - 固定 seed 数据版本。
  - 固定初始 SQLite 模板或 fixture 生成逻辑。
  - 固定时钟输入和 today date。
  - 固定随机性来源或提供 deterministic question seed。
- [ ] 为每个关键 API 记录“输入 -> 输出 -> 持久化副作用”。
  - `getBootstrapState`
  - `getTodayHomeState`
  - `getActivePlan`
  - `startStudySession`
  - `submitStudyAnswer`
  - `completeStudySession`
  - `cancelStudySession`
  - `getReportsOverview`
  - `getWrongWords`
  - `getWrongWordDetail`
- [ ] 建立基线断言层，不只比较 JSON 全量快照，还要比较关键业务不变量。
  - Today totals 是否与 snapshot 各 mode target/complete 对齐。
  - Session progress 是否单调推进。
  - isComplete 与 summary/currentQuestion 是否互斥正确。
  - 错词、报表、会话 summary 是否与提交历史匹配。
- [ ] 为每个学习模式生成最小可回放会话样本。
  - `newWord`
  - `review`
  - `mixedTest`
  - `wrongWordReinforcement`
  - `rootAffix`
- [ ] 为答题结果生成覆盖样本。
  - `correct`
  - `fuzzyCorrect`
  - `incorrect`
  - `skipped`
- [ ] 为问题类型生成覆盖样本。
  - `enToCnChoice`
  - `exampleToCnChoice`
  - `cnToEnChoice`
  - `enToCnInput`
  - `glossToRootInput`
  - `rootToGlossInput`
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

- [ ] 所有关键学习模式都至少有 1 个 happy path 样本和 1 个异常/边界样本。
- [ ] Today、Study、Wrong Words、Reports、Recovery、AI 六大类至少各有 2 个样本。
- [ ] 桌面参考路径与当前移动 Rust 路径已跑通同一套样本。
- [ ] 基线 runner 可以在本地重复运行，结果不依赖人工判断。
- [ ] 后续 Flutter 迁移工作被明确要求“先跑基线，后谈通过”。

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

#### Acceptance criteria

- [ ] Rust 核心分出明确边界，至少区分 `study/plan/report/vocab/storage/sync/platform-cloud`。
- [ ] 形成 Flutter 将使用的新契约文档，覆盖所有移动关键读写接口。
- [ ] 当前 RN bridge 契约与新契约完成逐项映射，标明沿用、重命名、拆分、废弃项。
- [ ] 同一套契约可同时服务桌面和 Flutter 移动端，而不夹带 RN 或 Flutter 特定语义。

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

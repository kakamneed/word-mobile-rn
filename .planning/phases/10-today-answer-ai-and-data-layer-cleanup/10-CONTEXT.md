# Phase 10：Today、答题、AI 与数据层清洗 - Context

**整理时间：** 2026-05-13
**状态：** 可进入规划

<domain>
## 阶段边界

Phase 10 执行 Phase 09 准备好的 Flutter 版学习流程清洗工作。清洗范围包括当前 Flutter 版 Today、学习答题、AI、SDK/bridge、Rust mobile bridge、app-core/study-core/storage、SQLite 持久化，以及与 Flutter 学习流程相关的云端/同步边界。

本阶段只关注 Flutter 版。React Native 路径不是活跃实现目标，只能作为旧残留或历史背景被标注，避免后续修改时误入旧路径。

本阶段可以积极删除旧残留、重复真相、空适配层、mock/demo 数据、过期文档和误导性测试；但必须保守对待产品行为：Rust/SQLite 仍然是学习真相来源，Flutter 只负责展示、交互、页面状态和导航衔接。
</domain>

<decisions>
## 实现决策

### 清洗策略
- **D-01：** 按 canonical 层级清洗：Flutter 页面 -> Flutter SDK DTO/client -> Flutter bridge codec/error -> Android/iOS native adapter -> Rust `platform-mobile` bridge -> `app-core` facade -> `study-core` 领域服务 -> `storage-core`/SQLite -> 可选的云同步和 AI 副作用。
- **D-02：** 不做大范围视觉重设计。保留现有页面、动效、导航手感和仍在使用的相邻功能，除非 Phase 09 或代码证据证明某条路径已经过时或有害。
- **D-03：** 每一次删除都必须有依据：无 import/caller、已被 canonical 路径替代、只服务 mock/demo、编码/文档残留、空适配层、RN-only 旧引用，或测试 fixture 已经断言过期行为。
- **D-04：** 避免无关重构。一个文件如果同时包含学习主链路和相邻功能，只清洗学习流程相关职责。

### Flutter Today 清洗
- **D-05：** `TodayShellScreen` 仍然是 Flutter 学习入口，但 Phase 10 应将核心学习数据和奖励、公告、排行榜、账号/同步诊断、AI 快捷卡等相邻内容区分开，减少旧耦合。
- **D-06：** Today 进度、任务目标、resume hint 和主按钮动作必须来自 `WordSdk` + Rust truth，不能由 Flutter 自己作为权威重新计算。
- **D-07：** `applySavedPlanToToday`、active plan fallback、resume/session handoff 必须清理到一致：Today、Plan、Study 共享同一目标来源，计划或词书变化后不能复用旧 active session。
- **D-08：** Flutter Today 学习主链路里影响理解的乱码或 placeholder 文案，应作为清洗项处理；与学习流程无关的相邻文案可登记后延后。

### 学习答题清洗
- **D-09：** `StudyScreen` 继续拥有本地选项选择状态、输入框文本、feed 翻页、loading/submitting 状态和服务端返回后的展示。
- **D-10：** `StudyScreen` 不能拥有正确性、最终进度、总结真相、错词副作用或报告副作用。
- **D-11：** 错误选项标红必须兼容用户答案的不同形态：label、value、text 都可能出现；同时正确选项仍必须来自权威 question/result 内容。
- **D-12：** 正确答案必须绑定真实 A/B/C/D 位置或选项文本。过期的 `correctChoiceLabel = A` fallback 不能在已有更可靠内容时把所有答案塌缩成 A。
- **D-13：** 保留当前类短视频 feed 学习模型：已答页面可回看，当前未答页面保持可操作，答题后不自动跳过反馈页，页面可见进度跟随当前 feed/page 语义。
- **D-14：** `sample_study_payloads.dart`、demo payload、test-only helper 和 fallback payload 逻辑必须审计：删除、迁移到测试，或明确标注为非生产路径。

### AI 清洗
- **D-15：** AI 短文、历史和错词导入应统一走一个 canonical Flutter AI 页面和一个 typed `AiClient` 路径。
- **D-16：** AI 仍然是可选增强：AI context、生成、历史、云恢复或 provider 调用失败，都不能阻塞 Today 或 Study。
- **D-17：** Flutter AI 代码不能拥有 provider secrets、生成真相、校验真相或持久化历史真相。
- **D-18：** Today AI 快捷入口和独立 AI 页面应共享同一套 context/history/generation 语义；重复的临时生成逻辑应合并或明确分层。
- **D-19：** AI 的同步/云恢复副作用可以保留，但必须和本地学习正确性、Today 可用性隔离。

### 错词页清洗
- **D-19A：** `WrongWordsScreen` 必须作为学习闭环的一等页面进入 Phase 10 清洗范围，而不是仅作为 Study/AI 的附属引用。
- **D-19B：** 错词列表、错词详情、错词筛选、hint 保存、AI hint suggestion、mastered/trash 排除关系必须拆开梳理，明确哪些是真正的错词本状态，哪些只是展示筛选或辅助建议。
- **D-19C：** 错词页不能重新发明错词真相。错词出现、错误次数、最近错误、优先级、hint、mastered/trash 排除都应来自 Rust/SQLite 或明确的 sync restore 结果。
- **D-19D：** Study 提交错误答案、WrongWordsScreen 展示、AI 错词导入、AI passage context、Reports 聚合之间的错词数据流必须被清洗成一条可解释链路，避免出现多个“看起来像错词本”的旧来源。
- **D-19E：** 清洗时要特别检查 mastered/trash 后错词页是否仍展示该词、错词导入是否绕过本地去重、高频词过滤是否只是 UI 标记、以及 hint 保存是否正确写入本地并参与后续展示。

### 报告页清洗
- **D-19F：** `ReportsScreen` 必须作为学习闭环的一等页面进入 Phase 10 清洗范围，覆盖 Today 完成后的 persisted aggregates，而不是只看 Study summary。
- **D-19G：** 报告页要拆分清楚：即时 session summary、Today 进度、历史报告、mode-based summary、streak/accuracy 等聚合字段分别由谁提供，不能把 Flutter UI 计算值当作报告真相。
- **D-19H：** Study 完成、WrongWords 更新、Reports 聚合、Today 刷新之间必须保持一致：完成一次学习后，报告页、错词页和今日页看到的结果不能来自三套不同计数。
- **D-19I：** 报告清洗必须审计 mock/fallback aggregate、旧 RN 报告语义、Flutter 本地临时计数和 Rust persisted aggregate 的重叠，删除或标注非 canonical 路径。
- **D-19J：** 报告页失败应是可恢复页面失败，不能反向影响 Today/Study 可用性；但报告数据如果为空或过期，必须能追溯到持久化层原因。

### 冷启动进入学习退回 Today 现象
- **D-19K：** 用户观察到“软件刚开启时进入学习后会退回一次今日页”的现象，Phase 10 必须把它作为启动态、导航态和学习 session handoff 的重点问题分析。
- **D-19L：** 该现象优先从 Flutter 启动链路排查：`AppState.initialize()`、auth/local data owner 恢复、`MobileRootShell` 当前 tab/route 状态、`TodayShellScreen._openStudy`、`StudyScreen._start()`、`StudyScreen.didUpdateWidget()`、以及返回 Today 的 callback。
- **D-19M：** Rust/bridge 侧需要同步检查首次启动后的 bootstrap、resume hint、active session snapshot、plan/today snapshot fallback、sync restore/backfill 是否触发了二次状态刷新，导致 Study 页面被重建或导航回 Today。
- **D-19N：** 修复该现象时不能用“延迟跳转”或“忽略第一次返回”这类 UI 补丁掩盖真因；必须明确是 ready/auth/sync 状态刷新、route rebuild、session start failure、resume mismatch，还是 Today refresh side effect 造成。
- **D-19O：** Phase 10 验收要增加冷启动 smoke：杀进程/冷启动 -> Today ready -> 立即进入某个学习模式 -> 不应被自动退回 Today -> 可提交一题 -> 返回 Today 后进度刷新正确。

### SDK、Bridge、Rust 与 SQLite 清洗
- **D-20：** `WordSdk` clients 仍然是 Flutter 学习功能唯一面向业务页面的 API。业务页面不应直接 import bridge primitives。
- **D-21：** Dart DTO 应镜像 Rust-owned contracts，而不是 native adapter quirks 或旧 mock payload。
- **D-22：** bridge method name 应集中在 SDK clients 和 native/Rust bridge adapter，不散落在 feature screen。
- **D-23：** `BridgeCodec` 和 `BridgeError` 是 Flutter 协议/错误边界；清洗应强化这个边界，而不是绕开它。
- **D-24：** Rust `platform-mobile/src/bridge.rs` 很大，但清洗必须保持行为并以测试护航。只有当抽取能真实降低旧耦合或提升学习流程边界安全性时才抽取。
- **D-25：** SQLite 中的 `study_results`、active session snapshots、Today plan/snapshot settings、wrong-word state、reports、AI passage history、mastered/trash state 都是高风险持久化面。不能为了修当前状态错乱而删除历史学习结果。

### 云端 / Supabase 边界清洗
- **D-26：** Supabase/auth/sync 只在影响学习流程启动、本地数据归属、AI 短文恢复、错词/报告同步或 Today 诊断时进入本阶段范围。
- **D-27：** Phase 10 不引入强制登录。游客/本地优先场景下，学习正确性必须依然成立。
- **D-28：** 云端 flush/restore 是本地 truth 周边的副作用，不是答案、进度和 Today truth 的第一来源。

### 验证要求
- **D-29：** Phase 10 完成前，Flutter 聚焦测试必须覆盖错误选项标红和非 A 正确选项身份保持。
- **D-30：** Rust 测试必须覆盖 answer evaluator/question builder 正确性、Today target 和 session target 一致性、active session restore、旧 snapshot 失效，以及涉及的持久化重启行为。
- **D-31：** AI 验证必须证明生成/历史失败不会阻塞 Today 或 Study。
- **D-32：** 如果 Flutter test runner 挂住但 analyze 通过，要明确报告测试 runner 未验证，不能当作通过。
- **D-33：** 代码清洗后应规划 release-device smoke：Today -> Study -> Submit -> Complete -> Today，以及 Today/AI 失败路径。
- **D-34：** 错词页验证必须覆盖：学习答错后进入错词页可见、mastered/trash 后不再作为活跃错词展示、hint 保存后可重新读取、AI 错词导入不会绕过去重和高频过滤。
- **D-35：** 报告页验证必须覆盖：完成学习后报告聚合更新、Today/Reports/Study summary 的核心计数一致、报告页空态/失败态不影响 Today/Study。
- **D-36：** 冷启动学习验证必须覆盖：刚打开软件后立即进入学习不会自动退回 Today 一次；如果发生退回，必须记录触发链路和最早错误来源。

### Agent 自主决策空间
- 大 Flutter 页面或 Rust bridge 文件内部的具体抽取边界。
- Phase 10 plan wave 内的具体任务顺序。
- 旧文档刷新后的具体文件名。
- demo payload 在 caller analysis 后是删除、迁移还是重标注。
### 侧边栏、排行榜和图片能力验证
- **D-37：** Phase 10 的清洗范围必须覆盖侧边栏中所有 Flutter 入口的可达性验证，不能只验证 Today、答题、AI、错词、报告这几条主学习路径。侧边栏入口要作为真实用户导航面检查：入口是否仍指向最新 Flutter 页面、是否存在旧 RN/占位/空壳页面、是否存在重复路由或隐藏旧实现。
- **D-38：** 排行榜页面需要被纳入拆解和清洗范围。验证重点包括排行榜数据来源、排序规则、空状态、刷新状态、用户自身排名展示、与学习结果/报告统计的边界，以及是否仍存在旧 mock 排名或临时本地计数。
- **D-39：** 排行榜的图片投票排行模式是独立功能链路，不能被当作普通排行榜 UI 顺手带过。它要拆开验证“图片来源 -> 图片抽取 -> 图片上传 -> 投票/排行记录 -> 排行榜展示 -> 失败/重试/离线状态”的完整路径。
- **D-40：** 图片抽取与上传能力要和 Flutter SDK、bridge、Rust/data、云端/同步边界一起确认所有权。Flutter 只负责选择、预览、上传交互和状态展示；持久化、远端标识、投票排行关联、失败恢复等真相不能散落在页面临时状态里。
- **D-41：** 侧边栏验证的验收要覆盖每个入口的 smoke path：打开页面、触发主要动作、返回/切换页面、冷启动后再次进入、无数据/有数据两种状态、错误状态展示。排行榜图片投票模式还要额外覆盖图片抽取失败、上传失败、重复上传、投票后排行刷新、图片资源缺失时的降级显示。

</decisions>

<canonical_refs>
## Canonical References

**下游 agent 在规划或实现前必须阅读这些文件。**

### 上游上下文
- `.planning/phases/09-learning-flow-structure-map-and-pitfall-inventory/09-CONTEXT.md` - Flutter-only 边界、层级结构、历史坑位和清洗 canonical refs。
- `.planning/phases/09-learning-flow-structure-map-and-pitfall-inventory/09-DISCUSSION-LOG.md` - 范围从全项目修正为 Flutter-only 的讨论记录。
- `.planning/ROADMAP.md` - Phase 10 目标、计划项和验收标准。
- `.planning/STATE.md` - 当前 resume point 和项目历史。

### Flutter 学习页面
- `apps/flutter_mobile/lib/features/today_shell_screen.dart` - Today 数据来源、进度、resume/action handoff、AI shortcut、同步和相邻卡片。
- `apps/flutter_mobile/lib/features/study_screen.dart` - 题目展示、选项状态、反馈、提交/下一题、resume、完成、mastered 行为。
- `apps/flutter_mobile/lib/features/ai_screen.dart` - AI 短文、历史、错词导入和非阻塞体验。
- `apps/flutter_mobile/lib/features/plan_screen.dart` - plan save/apply-to-today 和词书目标来源。
- `apps/flutter_mobile/lib/features/wrong_words_screen.dart` - 错词列表、详情、筛选、hint、AI suggestion、mastered/trash 排除关系。
- `apps/flutter_mobile/lib/features/reports_screen.dart` - session summary 之外的持久化报告、历史聚合、mode summary、streak/accuracy 展示。
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart` - 根导航和学习入口 handoff。
- `apps/flutter_mobile/lib/state/app_state.dart` - bootstrap/auth/local data owner 启动状态。
- `apps/flutter_mobile/lib/features/sample_study_payloads.dart` - 可能的 demo/fallback payload 残留，需要审计。

### Flutter SDK 与 Bridge
- `apps/flutter_mobile/lib/sdk/sdk.dart` - typed SDK 聚合入口。
- `apps/flutter_mobile/lib/sdk/today_client.dart` - Today DTO 和 bridge 调用。
- `apps/flutter_mobile/lib/sdk/plan_client.dart` - plan/wordbook DTO 和 bridge 调用。
- `apps/flutter_mobile/lib/sdk/study_client.dart` - study DTO、答题提交、resume、completion、mastered 方法。
- `apps/flutter_mobile/lib/sdk/ai_client.dart` - AI 短文/历史/导入/provider config 方法。
- `apps/flutter_mobile/lib/sdk/reports_client.dart` - reports DTO/client。
- `apps/flutter_mobile/lib/sdk/wrong_words_client.dart` - wrong-word/hint DTO/client。
- `apps/flutter_mobile/lib/sdk/sync_client.dart` - sync/cloud restore 副作用 client。
- `apps/flutter_mobile/lib/bridge/rust_bridge.dart` - MethodChannel transport 边界。
- `apps/flutter_mobile/lib/bridge/bridge_codec.dart` - JSON encode/decode 边界。
- `apps/flutter_mobile/lib/bridge/bridge_error.dart` - Flutter bridge 统一错误模型。

### Native / Rust / Data 层
- `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java` - Android MethodChannel/native adapter。
- `apps/flutter_mobile/android/app/src/main/kotlin/com/wordmobile/flutter_mobile/MainActivity.kt` - Android Flutter 集成。
- `apps/flutter_mobile/ios/Runner/AppDelegate.swift` - iOS native bridge 集成。
- `crates/platform-mobile/src/bridge.rs` - mobile bridge coordination、Today、study hydration、reports/wrong words、AI、sync、persistence。
- `crates/platform-mobile/src/android.rs` - Android Rust exports。
- `crates/platform-mobile/src/ios.rs` - iOS Rust exports。
- `crates/platform-mobile/src/paths.rs` - mobile DB/path ownership。
- `crates/app-core/src/facade/study_facade.rs` - active session 和 answer submission truth。
- `crates/app-core/src/facade/today_facade.rs` - Today facade boundary。
- `crates/study-core/src/question_builder.rs` - question/choice/correct option generation。
- `crates/study-core/src/answer_evaluator.rs` - correctness authority。
- `crates/study-core/src/session_definition.rs` - study mode/question count rules。
- `crates/storage-core/src/models/study_question.rs` - study question DTO truth。
- `crates/storage-core/src/models/study_requests.rs` - study request DTO truth。
- `crates/storage-core/src/models/today_home_state.rs` - Today DTO truth。
- `crates/storage-core/src/persistence/schema.rs` - SQLite schema。
- `crates/storage-core/src/persistence/mod.rs` - persistence module registry。
- `crates/storage-core/src/persistence/mastered_entry_repo.rs` - mastered/trash exclusion persistence。

### 边界文档和 Skill
- `docs/flows/today-plan-study-main-loop.md` - 主学习循环 ownership 和失败边界。
- `docs/flows/study-ui-vs-domain-boundary.md` - Flutter 与 Rust 学习职责划分。
- `docs/flows/ai-non-blocking-behavior.md` - AI 可选/非阻塞规则。
- `docs/flows/reports-ui-vs-aggregate-boundary.md` - 报告聚合边界。
- `docs/flows/wrong-words-and-reports-extension.md` - 错词和报告作为学习闭环延展面的参考。
- `docs/flutter/flutter-sdk-surface.md` - typed SDK 目标形态。
- `docs/flutter/bridge-architecture.md` - bridge architecture。
- `docs/flutter/bridge-error-model.md` - bridge error model。
- `docs/flutter/flutter-current-behavior-review.md` - 旧 Flutter 行为图，当前疑似乱码，需要刷新或替换。
- `skill/word-mobile-study-flow-map/SKILL.md` - 当前学习流程 debugging skill 和已知 invariants。
- `skill/flutter-today-study-target-consistency/SKILL.md` - Today/study target consistency skill。
- `skill/flutter-android-release-wireless-deploy/SKILL.md` - release-device 验证流程。

### 测试与检查
- `apps/flutter_mobile/test/study_question_display_test.dart` - Flutter UI 回归测试：答案展示、错误选项标红、正确 label/text、answered list merge。
- `crates/platform-mobile/src/bridge.rs` tests - Rust bridge 层 Today/session/AI/sync/persistence 测试。
- `crates/study-core/src/question_builder.rs` tests - 题目生成和正确选项测试。
- `crates/study-core/src/answer_evaluator.rs` tests - 答案正确性测试。
</canonical_refs>

<code_context>
## 现有代码洞察

### 可复用资产
- `StudyScreen` 已经有聚焦测试 helper：`choiceDisplayForTest`、`choiceStateForTest`、`studyHeroDisplayForTest`、`wordSkeletonDisplayForTest`、`mergeLatestAnsweredQuestionForTest`。Phase 10 应扩展这些，而不是另造测试框架。
- `WordSdk` 已经把主要学习能力都封装成 typed Flutter client；清洗应围绕它收敛。
- 现有文档已经明确核心边界：Flutter 负责呈现，Rust 负责领域真相，SQLite 负责持久化真相，AI 是可选增强。
- 现有本地 skill 已经记录 Today/study target mismatch、错误选项反馈、旧 correct label、active session snapshot 等真实踩坑。

### 既有模式
- Flutter feature screen 是 stateful 的，当前混合了 UI composition 和 data loading。清洗可以在降低风险时做分离，但不应整页重写。
- Bridge 调用使用 JSON string 和命名方法，例如 `getTodayHomeState`、`startStudySession`、`submitStudyAnswer`、`generateAiPassage` 等 contract family。
- Rust `platform-mobile` 是当前集成枢纽，累积了较多逻辑；Phase 10 不应在没有聚焦测试的情况下改变领域语义。
- Flutter 和文档中都有 Supabase/cloud 工作，但本地优先学习必须在无账号/无云端时仍成立。

### 集成点
- Today target 清洗横跨 `TodayShellScreen`、`PlanClient`、`TodayClient`、`StudyClient` 和 `platform-mobile` target/session hydration。
- 答题清洗横跨 `StudyScreen`、`StudyClient`、`platform-mobile`、`app-core::study_facade`、`study-core::question_builder` 和 `study-core::answer_evaluator`。
- AI 清洗横跨 `TodayShellScreen`、`AiScreen`、`AiClient`、`SyncClient`、`platform-mobile`、AI provider config/generation/history persistence，以及本地/云恢复。
- 错词页清洗横跨 `StudyScreen` 答错提交、`WrongWordsScreen`、`WrongWordsClient`、`AiClient` 错词导入、AI passage context、`platform-mobile` wrong-word loaders、SQLite wrong-word/hint/mastered 数据。
- 报告页清洗横跨 `StudyScreen` completion、`ReportsScreen`、`ReportsClient`、Today progress refresh、wrong-word side effects、`platform-mobile` aggregate builder 和 SQLite persisted reports。
- 冷启动进学习退回 Today 现象横跨 `AppState.initialize()`、auth/local data owner restore、`MobileRootShell` route/tab state、`TodayShellScreen._openStudy`、`StudyScreen._start()`、bridge session start/resume、以及 sync restore/backfill 引发的二次刷新。
- 持久化清洗横跨 active session app settings、`study_results`、wrong-word state、reports、AI passage history、mastered entries 和 Today plan/snapshot settings。

### 风险备注
- 当前工作区已有大量无关未提交改动。Phase 10 规划和执行时不能 revert 或覆盖用户已有改动。
- 多个 Flutter 文件和文档存在乱码文本。清洗要区分编码/文案修复与学习真相修复。
- Rust bridge 文件很大，影响面广；计划任务应小步、测试护航，并围绕 invariant 排序。
</code_context>

<specifics>
## 具体要求

- 保留当前 Flutter UX 和效果，同时移除旧残留。
- 两个历史硬 bug 是验收门槛：
  - 错误选项必须标红；
  - 正确答案不能塌缩到 A。
- 用户追加要求：Phase 10 必须补充错词页、报告页的拆解和清洗。
- 用户追加观察：软件刚开启时进入学习后会退回一次今日页，Phase 10 必须分析并纳入验证。
- Phase 09 是活跃 Flutter 层级结构的来源。
- Phase 10 的清洗结果要为 Phase 11 的 skill 化做准备。
</specifics>

<deferred>
## Deferred Ideas

- React Native 活跃功能清理/退役不属于本阶段，除非旧引用正在误导 Flutter 工作。
- 强制账号同步或多设备冲突解决不属于本阶段。
- 与学习流程旧结构无关的 Flutter 大改版不属于本阶段。
- 除非规划证明对安全清洗必要，否则不把 Rust bridge 全面重写成新模块。
</deferred>

---

*Phase: 10-today-answer-ai-and-data-layer-cleanup*
*Context gathered: 2026-05-13*

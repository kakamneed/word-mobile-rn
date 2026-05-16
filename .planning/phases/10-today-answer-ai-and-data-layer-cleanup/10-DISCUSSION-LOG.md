# Phase 10：Today、答题、AI 与数据层清洗 - Discussion Log

> **仅作讨论审计记录。** 不要把本文件作为 planning、research 或 execution agent 的输入。
> 真正决策已经写入 `10-CONTEXT.md`；本文件只保留讨论过程和备选项。

**日期：** 2026-05-13
**阶段：** 10-today-answer-ai-and-data-layer-cleanup
**讨论范围：** Flutter-only 清洗边界、行为保留、答题回归门槛、AI 和数据层边界

---

## 范围继承

| 选项 | 说明 | 是否选择 |
|------|------|----------|
| Flutter-only 清洗 | 清洗 Flutter app、Flutter SDK/bridge，以及 Flutter 学习流程使用到的 Rust/data 层 | 是 |
| 整个 mobile 清洗 | 把 React Native app 也作为活跃实现目标 | 否 |
| 只清文档 | 不做实现清洗，只刷新文档 | 否 |

**用户选择：** 继承 Phase 09 的修正：只关注 Flutter 版。

**备注：** React Native 路径只作为历史背景。Phase 10 只在旧 RN 引用会误导 Flutter 工作时进行标注或清除。

---

## 清洗深度

| 选项 | 说明 | 是否选择 |
|------|------|----------|
| 保守补丁 | 只修两个已知答题 bug | 否 |
| 分层彻底清洗 | 清洗 Today、答题、AI、SDK/bridge、Rust/SQLite 和云相关学习边界，同时保留活跃行为 | 是 |
| 重写主要页面 | 从头重写大页面或 bridge 模块 | 否 |

**用户选择：** 继承原始要求：彻底清除旧残留，不用计较清理复杂度，但保留现有功能和效果。

**备注：** 对旧残留要积极，对产品行为和视觉效果要保守。

---

## 不可妥协的回归门槛

| 门槛 | 必须达到的结果 |
|------|----------------|
| 错误选项标红 | 用户选择的错误选项必须清楚显示为错误，同时正确选项仍可识别 |
| 正确选项身份 | 正确答案必须保留真实 option index/text，不能塌缩到 A |
| Today truth | Today 进度和 Study session target 必须共享 Rust-backed truth |
| AI 非阻塞 | AI context/generation/history 失败不能阻塞 Today 或 Study |
| 持久化 | 不能为了修当前状态错乱而删除历史 `study_results` |

**备注：** 现有 Flutter 测试已经覆盖部分 UI 层答题状态；Phase 10 应保留或扩展它们，并在 domain/bridge 层变化时补 Rust 侧测试。

---

## 清洗边界

| 边界 | 决策 |
|------|------|
| Flutter 页面 | 可以简化，但不做大范围重设计 |
| Flutter SDK | 必须保持为 feature-facing bridge API 的唯一入口 |
| Bridge codec/error | 保留为协议边界 |
| Rust/domain | 拥有学习真相 |
| SQLite | 拥有本地持久化真相 |
| Supabase/cloud | 是副作用和恢复/同步层，不是本地答案真相 |
| AI | 是可选增强，不阻塞 bootstrap/study |

**备注：** 这些边界来自 Phase 09 和现有文档：`today-plan-study-main-loop.md`、`study-ui-vs-domain-boundary.md`、`ai-non-blocking-behavior.md`、`flutter-sdk-surface.md`。

---

## Agent 自主决策空间

- 大 Flutter/Rust 文件内部的具体抽取边界。
- Phase 10 planning 中的具体任务顺序。
- 旧文档刷新后的具体文件名。
- demo payload 在 caller analysis 后删除、迁移还是重标注。

---

## 用户追加范围

| 追加项 | 说明 | 处理结果 |
|--------|------|----------|
| 错词页拆解和清洗 | 错词页不能只作为 Study/AI 的附属引用，需要完整拆解列表、详情、筛选、hint、AI suggestion、mastered/trash 排除和持久化来源 | 已加入 `10-CONTEXT.md` 的“错词页清洗”、canonical refs、集成点和验证门槛 |
| 报告页拆解和清洗 | 报告页需要覆盖 persisted aggregates、历史报告、mode summary、streak/accuracy，并和 Today/Study summary 对齐 | 已加入 `10-CONTEXT.md` 的“报告页清洗”、canonical refs、集成点和验证门槛 |
| 冷启动后进学习退回 Today 一次 | 用户观察到软件刚开启时进入学习后会退回一次今日页，需要分析启动、导航、auth/sync restore、session handoff 的交界问题 | 已作为高优先级现象加入 `10-CONTEXT.md`，要求 Phase 10 做冷启动 smoke 和最早错误来源追踪 |

**用户原话：** “应该再补充上错词、报告两页的拆解和清洗，以及分析在软件刚开启时进入学习后会退回一次今日页的现象”

**备注：** 这不是扩展到新产品能力，而是补齐学习闭环清洗范围。错词、报告和冷启动学习入口都直接影响 Flutter 学习流程稳定性。
# 用户补充：侧边栏、排行榜、图片投票排行和图片抽取上传

| 补充范围 | 规划含义 | 必须验证 |
|--------|------|----------|
| 侧边栏各功能入口 | Phase 10 不能只清洗学习主链路，还要验证侧边栏里所有 Flutter 功能入口是否仍指向最新页面 | 页面可达、返回切换、冷启动再进入、无数据/有数据/错误状态 |
| 排行榜 | 排行榜属于侧边栏功能验证的一部分，需要拆清数据来源、排序、刷新、空状态、自身排名展示 | 不允许残留 mock 排名、旧 RN 路由、临时页面计数冒充真实排行 |
| 图片投票排行模式 | 这是排行榜下的独立模式，和普通学习排行不同 | 图片来源、图片抽取、上传、投票记录、排行刷新、失败重试 |
| 图片抽取/上传能力 | 该能力与 Flutter SDK、bridge、Rust/data、云端/同步边界有关 | Flutter 负责交互和状态，持久化/远端标识/排行关联要有明确真相层 |

**结论：** Phase 10 的计划要把“侧边栏功能验证”作为独立验收面，并把排行榜图片投票模式拆到图片抽取、上传、投票、排行展示的完整链路中。

---

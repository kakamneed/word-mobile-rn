# RN 到 Flutter 对齐审计

状态：进行中草稿
负责人：移动端壳层对齐工作
阶段：`2026-04-22-flutter-app-supabase-implementation` 的 RN parity replan

## 目的

这份文档用于记录旧 RN 客户端和当前 Flutter 壳之间，面向用户的行为差异。

它不是架构说明，而是为了保证 Flutter 后续实现继续围绕 RN 的真实用户流程推进，而不是重新长成一套调试壳。

## 根壳层

### RN 版真实情况

文件：
- `apps/mobile/src/navigation/mobile-root.tsx`

行为：
- 根壳拥有稳定的主导航：`today / plan / wrongWords / reports / ai`
- `study` 不是主 tab，而是由 Today 的任务流进入
- 每个主页面都有明确返回路径
- 底部导航是稳定心智模型，不是页面里零散的 CTA 按钮

### Flutter 之前的偏移

文件：
- `apps/flutter_mobile/lib/main.dart`
- 旧 Today 动作区

问题：
- 主导航几乎被藏在 Today 页面里
- Plan / Reports / Wrong Words / AI 更像散落的工具页，不像稳定产品页
- Study 看起来更像普通子页，而不是从任务流进入的会话流程

### Flutter 当前方向

- `MobileRootShell` 负责主导航
- Today 只负责任务流和 carryover，不再承担全局路由职责

## Today 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/today/today-screen.tsx`

结构顺序：
1. 下一步 Hero
2. 今日任务拆解
3. carryover / 计划 / 词书上下文
4. AI 上下文与生成入口
5. 刷新行为

行为：
- 首先回答“我今天下一步做什么”
- `getPrimaryAction` 根据权威 snapshot 进度推导 study mode
- Today 不以底层调试状态为主
- AI 入口依赖今日完成情况和错词可用性

### Flutter 之前的偏移

问题：
- Today 以前以 bootstrap/account/sync 诊断卡开场
- 原始 map，比如 `{status: none}`，直接暴露给用户
- 主页面切换按钮混在正文中
- 页面更像“实现状态说明”，不是“学习任务入口”

### Flutter 当前方向

- 诊断只保留在折叠区
- Hero、任务拆解、carryover、AI shortcut 始终排在前面
- 不再把 placeholder 结构和原始 map 暴露在主路径

## Plan 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/plan/plan-screen.tsx`
- `apps/mobile/src/screens/plan/plan-editor-screen.tsx`
- `apps/mobile/src/screens/plan/plan-quick-edit.tsx`
- `apps/mobile/src/screens/plan/wordbook-selector-section.tsx`
- `apps/mobile/src/screens/plan/growth-rule-section.tsx`

行为：
- Plan 是一条编辑工作流，不是原始配置表单
- 快速编辑数字要易调节
- 词书选择属于计划页职责
- 保存和同步到 Today 是两个独立动作
- growth rule 是单独成组的编辑语义

### Flutter 之前的偏移

问题：
- 页面更像对 plan API 的薄包装
- 用户面对的是裸数字字段，结构不够清晰
- 词书虽然可切换，但没有成为完整 planning workflow 的一部分

### Flutter 当前方向

- hero 摘要 + 快速编辑 + growth rule 摘要 + 词书管理
- 保持 `save` 和 `apply-to-today` 的动作区分
- 后续继续向 RN 编辑器形态靠拢，而不是停留在简化表单

## Study 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/study/study-session-screen.tsx`
- `apps/mobile/src/screens/study/question-card.tsx`
- `apps/mobile/src/screens/study/study-summary-screen.tsx`
- `apps/mobile/src/screens/study/resume-session-prompt.tsx`

行为：
- Study 是一个连续会话流
- 阶段清晰：开始、题目、反馈、总结、取消/完成
- resume/carryover 语义很重要
- 页面应该像一个会话，不是若干 endpoint 的拼接

### Flutter 之前的偏移

问题：
- Study 更像 API harness，而不是会话流
- Resume 语义不完整
- 总结和推进节奏仍偏技术态

### Flutter 当前方向

- 保持 Rust 作为权威学习会话真相
- UI 围绕 session lifecycle 组织，而不是请求生命周期组织
- Today 和 Study 之间用真实 mode 语义连接起来

## Reports 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/reports/reports-overview-screen.tsx`

行为：
- 先给 streak / 总览 framing
- 关键指标要一眼可扫
- 趋势浏览支持切日期
- 模式拆分支持聚焦查看
- 页面整体是“分析感”，不是“结构体展示”

### Flutter 之前的偏移

问题：
- 页面直接暴露原始结构或弱文本输出
- 趋势和模式的探索交互不够
- 视觉层级不够像报告页

### Flutter 当前方向

- 总览 hero + 指标卡 + 横向日趋势浏览 + 点选日详情 + 模式拆分 + 模式详情
- 继续向 RN 的分析体验靠拢，而不是停在对象转文本

## Wrong Words 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/wrong-words/wrong-word-list-screen.tsx`

行为：
- 顶层统计先总结整个错词本
- 筛选 chips 改变浏览集合
- 主列表要清晰可扫
- 点选词条后要进入“帮助复习”的详情层，而不是 inspection-only
- 整体是学习支持工具，不是调试面板

### Flutter 之前的偏移

问题：
- 更像 debug split-pane
- 原始 meanings/error arrays/risk arrays 直接打印
- 筛选和分区语义弱

### Flutter 当前方向

- 统计 + 筛选 + 列表 + 详情
- `all` 视图增加高优先级分区
- 后续再补更明确的强化入口

## AI 页面

### RN 版真实情况

文件：
- `apps/mobile/src/screens/ai/ai-passage-screen.tsx`

行为：
- AI 页面必须从 today context 出发
- 能否生成依赖任务完成和错词可用性
- 历史和最新正文都是一等内容
- 正文渲染是结构化阅读体验，不是对象 dump
- 错误文案面向用户，而不是面向调试

### Flutter 之前的偏移

问题：
- 有硬编码 `targetWords` 和 `level`
- 历史记录只是标题数组直出
- 页面没有真正锚定在 Today 学习上下文上

### Flutter 当前方向

- 先看 today context，再决定是否生成
- 生成目标词从当天错词里推导
- 历史记录可切换
- 正文块继续向 RN 的阅读样式靠近

## 诊断信息位置规则

诊断仍然要保留，但必须让出主路径。

规则：
- bootstrap/account/sync 仍可见
- 原始技术状态不能主导 Today
- support 信息应放在折叠区、debug surface 或独立支持入口
- 用户主路径中的空态/错误态应优先用任务导向文案表达

## 当前优先级顺序

1. Today 任务流可信度
2. 稳定根导航语义
3. Plan / Study 工作流对齐
4. Reports / Wrong Words / AI 产品态对齐
5. 诊断信息降级整理
6. 在 parity 达到可用前，不优先深化 Supabase / sync 扩展

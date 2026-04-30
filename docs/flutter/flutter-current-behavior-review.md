# Flutter 当前行为说明

状态：审阅草稿
负责人：移动端壳层对齐工作
日期：2026-04-26

## 目的

这份文档用于说明当前 Flutter 版本在各页面上的真实运行方式和逻辑细节，方便逐项核对：

- 现在页面实际怎么跑
- 页面依赖哪些数据源
- 哪些行为已经向 RN 版靠拢
- 当前更适合重点审哪些细节

它不是架构文档，而是给产品/实现对照检查用的运行说明。

## 全局壳层

### 启动路由

文件：
- `apps/flutter_mobile/lib/main.dart`

当前行为：
- 应用启动后进入 `AppState.initialize()`
- 然后根据初始化结果进入以下其一：
  - 启动加载页
  - onboarding
  - ready shell
  - startup error
- 当前 ready shell 是 `MobileRootShell`

### 根导航

文件：
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`

当前行为：
- 底部导航有 5 个主入口：
  - `Today`
  - `Plan`
  - `Wrong`
  - `Reports`
  - `AI`
- `Study` 不是底部主 tab，而是从 Today 的任务流进入
- 从其他 tab 切回 `Today` 时会触发 Today 重载，避免一直显示旧状态

## Today 页面

文件：
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`

### 数据来源

当前读取：
- Rust bridge 的 `getTodayHomeState()`
- `getActivePlan()`
- `getWordbooks()`
- `getTodayAiPassageContext()`
- `getAiPassageHistory()`
- `getSyncStatus()`
- `getResumeSessionHint()`

### 未完成学习恢复卡片

当前行为：
- 如果 Rust 侧发现存在已持久化但未完成的学习会话，Today 会展示恢复卡片
- 卡片会显示：
  - 模式
  - 当前进度
  - 当前词（如果有）
- 点击后会用该模式重新进入 `StudyScreen`

### 主动作 Hero

当前行为：
- 如果没有可用的 Today snapshot，而且也没有活动计划：
  - Hero 会提示先配置计划
  - 主按钮会去 Plan 页
- 如果已经有活动计划，但 Today snapshot 还没生成：
  - Hero 会提示“当前已有计划，但今天的任务快照还没有生成”
  - 主按钮会执行 `applySavedPlanToToday()`
- 如果 Today snapshot 已存在：
  - 下一步动作会按剩余未完成任务桶顺序推导：
    1. `newWord`
    2. `review`
    3. `mixedTest`
    4. `wrongWordReinforcement`
    5. `rootAffix`
- 如果所有任务桶都完成：
  - Hero 会显示今日任务已完成
  - 主按钮会引导进入 AI 页面

### 任务拆解区

当前行为：
- 如果没有 snapshot：
  - 会显示说明文案
  - 提供“前往计划页”或“同步到 Today”按钮
- 如果有 snapshot：
  - 会渲染所有 target > 0 的任务项
  - 每项包含：
    - 名称
    - 辅助文案
    - 进度条
    - 完成数 / 目标数
  - 点击未完成项会用该项对应 mode 进入 `StudyScreen`

### 计划与词书卡片

当前行为：
- 有活动计划时显示计划摘要
- 显示当前启用词书摘要
- 提供返回 Plan 页入口

### AI 快捷入口

当前行为：
- 会根据 Today AI context 判断当前是否适合进入 AI
- 会显示今日错词数
- 如果已有历史短文，会显示最近一篇标题
- 点击进入 AI 页面

### 诊断折叠区

当前行为：
- 默认折叠
- 保留 bootstrap/account/sync 诊断信息可见性
- 不再占据首页主路径

### 当前审阅关注点

- `同步到 Today` 后，任务快照是否会立刻正确生成
- 完成度和 Hero 文案是否与真实学习进度一致
- 恢复未完成学习卡片是否能顺利回到原会话

## Plan 页面

文件：
- `apps/flutter_mobile/lib/features/plan_screen.dart`

### 数据来源

读取：
- `getActivePlan()`
- `getWordbooks()`

写入：
- `savePlan()`
- `applySavedPlanToToday()`
- `toggleWordbook()`

### 当前行为

- 顶部 hero 显示当前计划摘要
- 快速编辑区可编辑：
  - 新词学习
  - 复习
  - 混合测试
  - 错词强化
  - 词根词缀
- growth rule 现在可编辑两个关键参数：
  - `intervalDays`
  - `increment`
- 词书管理已经内嵌在计划页中
- “保存计划”和“同步到 Today”是两个独立动作
- 页面会显示未保存改动提示

### 当前审阅关注点

- quick edit 的步进手感是否足够顺
- growth rule 这两个核心参数是否满足当前使用需要
- 保存后回到 Today 时，是否能看到预期变化

## Study 页面

文件：
- `apps/flutter_mobile/lib/features/study_screen.dart`

### 进入方式

- 页面必须带 `mode`
- 从 Today 进入时，会使用当前主动作或任务项对应的 mode
- `wrongWordReinforcement` 优先尝试用当前错词本数据起会话
- 如果 richer 数据不可用，仍会回退到 `starterStudyPayloads`
- 如果从 Today 恢复卡进入，会优先尝试匹配持久化会话

### 会话阶段

当前 UI 有这些阶段：
- loading
- question
- feedback
- complete
- error

### 题目阶段

- 顶部 hero 显示当前词和进度
- 中间显示 prompt 和示例句
- 选择题渲染单选项
- 输入题渲染文本输入框
- 提交通过 Rust bridge 走真实学习链路

### 反馈阶段

- 显示这题答对还是需要强化
- 显示用户答案和正确答案
- 可以继续下一题或进入总结

### 总结阶段

- 显示总结指标卡片
- 提供：
  - 返回 Today
  - 再来一轮
  - 如果有下一模式，则继续下一轮

### 当前审阅关注点

- 恢复未完成学习时，是否真的回到原会话而不是新开一轮
- 取消会话的提示和结果是否符合预期
- `wrongWordReinforcement` 模式是否真的用到了当前错词本数据

## Wrong Words 页面

文件：
- `apps/flutter_mobile/lib/features/wrong_words_screen.dart`

### 数据来源

读取：
- `getWrongWords(filter)`
- `getWrongWordDetail(entryId)`

### 筛选值

当前可用：
- `all`
- `highPriority`
- `recent`

这些值已经和 Rust 当前支持对齐。

### 当前行为

- 顶部 hero 显示：
  - 错词总数
  - 累计错误
  - 平均优先级
- 当 filter 为 `all` 时，会额外显示“建议优先复习”分区
  - 这里会挑出优先级 >= 8 的词
  - 同时提供“开始错词强化”按钮
- 下方显示主错词列表
- 点开词条后会展示：
  - 释义
  - 错误历史
  - 风险拆解

### 当前审阅关注点

- “建议优先复习”分区是否足够有分区感
- “开始错词强化”是否符合你想要的强化入口语义
- 详情区是否已经足够像错词本，而不是调试面板

## Reports 页面

文件：
- `apps/flutter_mobile/lib/features/reports_screen.dart`

### 数据来源

读取：
- `getReportsOverview()`

### 当前行为

- 顶部 streak hero
- 指标卡片区
- 横向滚动的日趋势卡片
- 点击单日卡片后切换当日详情
- 模式拆分列表
- 点击模式项后切换模式详情
- 当前详情区已经补了分析文案，不再只有裸数字

### 当前审阅关注点

- 日趋势卡片的选中反馈是否明显
- 当日分析和模式分析文案是否有“报告页”感觉
- 当前交互是否已经足够接近 RN 的图表分析体验

## AI 页面

文件：
- `apps/flutter_mobile/lib/features/ai_screen.dart`

### 数据来源

读取：
- `getTodayAiPassageContext()`
- `getAiPassageHistory()`
- `getAiPassage()`

写入：
- `generateAiPassage()`

### 当前行为

- 顶部 hero 显示 AI 上下文：
  - 今日任务是否完成
  - 今日错词数
  - 历史篇数
- 生成按钮会先检查：
  - 今日任务是否完成
  - 今日是否有错词可用
- 最近一篇短文作为当前正文主展示
- 历史记录可切换
- 当前选中的历史项会高亮
- passage blocks 现在已经开始做 richer text 渲染：
  - 普通文本
  - 重点词高亮
  - gloss 样式区分

### 当前审阅关注点

- block 排版是否已经有阅读感
- 历史列表和当前正文之间的关系是否清楚
- 空态和失败文案是否足够自然

## 跨页面说明

### 诊断信息位置

当前原则：
- 诊断还保留
- 用户主路径优先
- 相比最早那版 Flutter 调试首页，已经更接近 RN 的产品意图

### 当前最值得优先验证的点

1. Today snapshot 是否随着“同步到 Today”立即正确生成
2. Resume session 是否能真的回到原会话
3. Plan 改动是否会可靠地影响 Today 主流程
4. Wrong Words 的强化入口是否符合预期
5. Reports / AI 是否已经有足够的产品态，而不再只是调试壳

# Phase 12 UI Spec - Flutter Parity Contract

## App Shell

The mini program must feel like the Flutter app, not a default WeChat page.

- Use a custom navigation shell with status-safe top padding, Chinese page title on the left, and circular avatar on the right.
- Remove native page-title chrome for Today, Plan, Wrong Words, Reports, and account/drawer entry surfaces.
- Use a custom bottom navigation with four tabs:
  - 今日
  - 计划
  - 错词
  - 报告
- Each tab has an icon and label. The selected tab is wrapped by a lavender rounded pill like Flutter.
- Account actions open a right-side drawer overlay. The drawer shows avatar, `king`, `已登录`, and the entries:
  - 新手引导 / 重新设置词数并回顾功能
  - 鳄bti 学习人格 / 测试适合你的新词、复习、混测和错题权重
  - 个人信息 / 昵称和头像
  - 排行榜 / 预留入口，后续接入云端排行
  - 设置 / 主题和偏好
  - Check for updates / Look for a newer APK release
  - 退出登录 / 保留本机学习数据

## Today

Hero card:

- Purple card with date, `学习新词`, `先把今天的新词任务推进起来。`, progress rail, `今日完成度 0%`, `剩余 20 个任务单位`, and white pill button `进入新词学习`.
- Use the same rounded radius, inner padding, and large typography scale as the Flutter screenshot.

Task breakdown card:

- Heading `今日任务拆解`, edit icon, `修改`.
- Rows:
  - Green dot, `新词学习`, `建立今日新词基础`, `0/20`, chevron, rail.
  - Blue dot, `复习`, `回顾旧词，巩固记忆`, `0/28`, chevron, rail.
  - Brown dot, `混合测试`, `综合检验`, `0/34`, chevron, rail.
  - Red dot, `错词强化`, `回收今天的薄弱点`, `0/26`, chevron, rail.

Lower Today sections:

- `今日奖励` card with a slot-machine illustration block on the left and a locked gradient reward card on the right.
- `开发诊断` collapsible card with the same row height and chevron rhythm as Flutter.

AI omission:

- Do not render the Flutter `AI 短文总结` card in the mini program v1 UI.
- Do not render any `生成 AI 短文` button, AI tab, or AI page.

## Plan

- Page title `计划` with avatar.
- Purple hero card `鳄甲卫`, descriptive copy, metric chips `20 新词/天`, `28 复习/天`, `1 词书`, and status `已微调书：KaoYan`.
- `计划名称` card with outlined input and `3/50` counter.
- `每日目标` card with rows for 新词学习, 复习, 混合测试, 错词强化, 词根词缀. Each row has label, description, minus button, numeric input box, plus button.
- `增长规则` card with switch, selected segmented button `全部共享`, `分别设置`, and unified growth-rule controls `增长间隔 7`, `增长增量 5`.
- `词书管理` card with radio rows CET-4, CET-6, KaoYan, Medical English.
- Sticky bottom action bar with purple `保存计划` and outlined `同步到今日`.

## Wrong Words

- Page title `错词本` with avatar.
- Purple summary card `错词本`, description, and metric chips: `198 错词数`, `48 词根词缀`, `391 累计错误`, `6.3 平均优先级`.
- Filter card `筛选` with selected chip `全部` and chips `高优先级`, `最近错误`, `高频出错`.
- Reinforcement card `强化入口` and lavender pill button `开始错词强化（最多 20 个）`.
- List card `错词列表`; word rows include word, phonetic, Chinese meaning, error count/priority, score pill, and red dot where the target shows one.

## Reports

- Page title `报告` with avatar.
- Purple streak hero `0 天连续学习`, descriptive copy.
- Metric grid: `16 学习天数`, `162 已学词数`, `802 总答题数`, `50% 整体正确率`; the accuracy tile uses the gray-highlight style.
- `每日正确率趋势` card with selected day summary and line chart, not the current bar placeholder.
- `按模式查看` card with one expanded gray selected mode and multiple compact mode cards. Preserve mode names, percent colors, progress rails, and mini chart treatment.

## Pixel QA Rules

- No English placeholder labels on the primary learning surfaces except the Flutter drawer row `Check for updates`.
- No AI-related visible UI appears in the mini program v1 shell.
- No green hero cards on Today/Reports/Account shell; the canonical hero color is purple.
- No nested card-in-card page sections except repeated item tiles and intentional inner metric chips.
- Text must not overlap at 390px logical width.
- Every visual page must include enough bottom padding to clear the custom tab bar and safe area.

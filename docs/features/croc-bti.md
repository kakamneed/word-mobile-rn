# Feature: Croc BTI

> Slug: `croc-bti`
> Status: `mobile_in_progress`
> Updated: `2026-08-11`

## Product Intent

鳄 bti 是一个带鳄鱼梗的人格化学习测试。它通过若干偏好题判断用户更适合“多学新词、稳态复习、混合测试、错词强化、语境理解/词根词缀”等哪种学习节奏，再把结果转成今日计划、题型占比和可解释的人格称号。

核心目标不是娱乐测试本身，而是让用户第一次配置计划、后续重新调计划时有一个更低压、更有趣、也更可解释的入口：用户看到“我为什么适合这个学习配比”，并能在应用前微调。

## UX Contract

- 入口放在手机端侧边栏，不放计划页；第一次引导流程里也提供一个可跳过的鳄 bti 步骤。
- 用户如果做过一次测试，之后再进入鳄 bti 默认展示已测出的人格和推荐结果；只有主动点击重测才重新答题。
- 测试结果页展示：人格代号、原人格名称、对应鳄鱼形象图、合并后的中文描述、学习模式推荐、今日计划推荐、题型推荐占比、推荐原因、应用按钮。
- 人格名称使用原版本，例如 `鳄卷师`、`鳄骑兵`、`鳄渊者`、`鳄语师`、`鳄刑官` 等；图片职业名只作为视觉资产，不覆盖结果名称。
- 人格描述用“原学习说明 + 鳄味句子”合并成一段。例如：`你适合一边扩充词库，一边把词义、例句和搭配卷进自己的知识卷轴里。卷轴一摊，词义、例句和搭配都被它卷进随身小本本里。`
- 鳄味命名可带西幻/中式玄幻感；`不鳄客` 是谐音梗，`全员鳄玉` 来自“全员恶玉”的变体梗，但目前这些只用于视觉/描述趣味，不应替代原人格 title。
- 图片应使用本地资产 `assets/croc_bti/`，结果图保持透明/贴纸风格展示。
- 应用结果时应提示成功，并让今日首页和计划页同步反映新计划；计划名称改为当前学习人格名称。

## Shared Domain/Data Contract

- 四轴人格代码由 `vc`、`io`、`nr`、`at` 组成：
  - `V/C`: 词库优先 vs 语境优先。
  - `I/O`: 识别理解 vs 主动输出。
  - `N/R`: 新词推进 vs 复习稳固。
  - `A/T`: 理解分析 vs 测试暴露。
- `CrocBtiResult` 至少包含：`code`、`title`、`summary`、`advice`、`assetPath`、`flavor`、`axisScores`、`weights`、`planInput`、`questionTypeWeightsByMode`。
- 评分和推荐逻辑必须保持纯函数，便于 Flutter、Tauri、测试和未来 Rust 共享/迁移。
- 计划推荐的单位统一为“题”，不再让“新词/复习”在 UI 上隐含不同倍率。
- 每日学习时间按平均一题 15 秒计算：10 分钟约 40 题，20 分钟约 80 题，以此类推。
- 新词学习仍保持原四题型固定顺序，因此新词题数必须是 4 的倍数。
- 题型定制只作用于复习、混测、错词强化；新词学习不应用人格题型占比，词根词缀保持老流程。
- `questionTypeWeightsByMode` 只保留可定制的模式，必须过滤 `newWord` 和 `rootAffix`。
- 可加入的新题型包括 `exampleToCnChoiceNoTranslation` 和 `wordSkeletonInput`。
- 应用到计划时，复用已有 plan save/apply 合同，不能绕过 active plan、Today target 或计划页同步机制。

## Flutter Mobile Route

- Owner screen/widget: `croc_bti_screen.dart`、`croc_bti_model.dart`、`account_drawer.dart`、`mobile_root_shell.dart`、`onboarding_flow.dart`。
- SDK/bridge calls: 当前计划读取/保存通过 plan SDK/bridge；Croc BTI profile 同步通道有 `getCrocBtiProfile` / `saveCrocBtiProfile` 客户端封装。
- Loading/cache/reload behavior: 进入页面时如已有完整答案默认展示结果；点击重测清理保存答案；应用结果后必须刷新 Today 和 Plan。
- Orientation/gesture constraints: 手机端用纵向滚动卡片承载问卷、结果、计划滑条、题型滑条；滑条视觉需要避免大段空白，thumb 和进度条必须按同一比例计算。
- First implementation slice: Flutter 已有侧边栏入口、首次引导入口、结果页、计划推荐、题型占比、图片展示和本地保存逻辑。
- Current status: Mobile core implemented; verification needs re-run after Flutter toolchain 恢复正常。

## Tauri Desktop Route

- Owner view/window: 计划/设置中的“鳄 bti 学习人格”面板，或新手引导中的可跳过步骤。
- Shared APIs to reuse: 复用四轴评分、模式权重、每日时间换算、题型占比归一化、plan apply 合同。
- Desktop-specific layout: 桌面不必照搬手机分步问卷；建议左侧题目列表，右侧实时人格/计划/题型预览。
- Mobile assumptions to avoid: 不要复制手机端滑条宽度/触控尺寸；不要依赖侧边栏抽屉作为唯一入口；不要把手机端 SharedPreferences 作为桌面事实来源。
- First parity slice: 读取/填写问卷、展示结果、展示图片、推荐今日计划和题型占比、用户微调、应用到 active plan。
- Current status: Planned; 尚未实现桌面 UI。

## Sync And Storage

- Flutter 当前保存完整测试答案和每日学习时间，用于下次进入默认展示结果。
- 保存 key 必须按账号 scope 隔离；游客和登录用户不能互相污染。
- 旧的未分 scope Croc BTI key 在 guest 模式下应忽略，避免历史数据误套到当前用户。
- 结果应用后的长期业务状态是 active plan；测试答案/profile 是辅助偏好数据。
- Supabase/cloud 同步如后续开启，应同步 profile/answers 的稳定结构，且与 plan 应用保持单一写入路径，避免云端 profile 与本地 plan 不一致。

## AI Or Provider Implications

当前功能不依赖 AI/provider。AI 可用于未来生成更个性化的推荐解释，但必须不改变纯函数输出的核心权重，不影响题型占比和计划数值的可重复性，失败时回退到本地固定解释。

## Implementation Log

- `2026-08-11`: Diagnosis only: the observed wrong-word reinforcement ceiling at 30 and review `28 -> 25` Today display mismatch were not caused directly by Croc BTI daily-time allocation. The direct causes were the Plan screen wrong-word target cap and the Today available-pool target rewrite in the shared mobile bridge.
- `2026-08-06`: Added `highFrequencyPerDay` to Croc BTI recommendations and the editable plan sliders. The recommendation splits high-frequency questions out of the prior review allocation so the selected daily-time question budget remains unchanged rather than adding extra work.
- `2026-05-06`: 加入 Croc BTI 问题、评分、标题、推荐逻辑、测试、移动端入口，并镜像到 Flutter shell。
- `2026-05`: 入口从计划页改为侧边栏；首次引导加入可跳过的鳄 bti 流程。
- `2026-05`: 增加“做过一次后默认展示结果，手动重测才重新答题”。
- `2026-05`: 扩展推荐结果：每日学习时间问题、今日计划题数、题型推荐占比、推荐原因、应用前微调。
- `2026-05`: 明确题量单位统一为“题”；10 分钟约 40 题，20 分钟约 80 题；新词必须 4 的倍数。
- `2026-05`: 明确题型定制只作用于复习、混测、错词；新词保持四题型顺序，词根词缀保持原流程。
- `2026-05`: 修复计划应用后 Today 与计划页不同步的问题，计划名称应改为当前学习人格。
- `2026-05`: 调整推荐条/滑条视觉，减少空白，并修复题型滑动点与进度条不对齐。
- `2026-05`: 加入 16 张鳄 bti 人格形象图到 Flutter 资产目录，并在结果页展示。
- `2026-05`: 人格名称改回原版本；图片职业名不覆盖 title；描述改为原 summary 与鳄味 flavor 合并。
- `2026-06-25`: 按 cross-platform-feature-ledger 整理并扩充本功能账本。

## Mobile Lessons Learned

- 入口位置很重要：放计划页容易埋，侧边栏更像“个人学习人格”的长期入口。
- 首次引导必须可跳过，避免新用户还没理解产品就被完整问卷拦住。
- 测过一次后默认看结果，比每次进来重答更符合用户预期。
- 应用结果要同时更新 Today 和 Plan；只刷新首页会造成“今日计划变了、计划页没变”的错觉。
- 题量单位必须对用户透明；复习不能再偷偷按旧倍率变成 12/48 这种用户难理解的数量。
- 新词题型顺序是学习流程的一部分，不能被人格占比打乱。
- 词根词缀模式不按普通题型拆分，继续走老流程。
- 题型推荐要写推荐原因，否则用户只看到百分比，不知道为什么要接受。
- 视觉滑条不能只调数值，还要核对 thumb、进度条、轨道比例是否一致。
- AI 生图提示词不稳定，最终应优先使用用户挑好的本地形象图。

## Desktop Follow-Up Notes

- 桌面端应从这份账本复刻“意图与合同”，不要从手机 UI 逐像素移植。
- 桌面更适合实时预览：答题时右侧即时显示可能人格、计划题量和题型占比变化。
- 推荐原因可以更详细，但应用前仍需给用户明确微调入口。
- 图片资产应共用或从同一 source manifest 生成，避免 Flutter/Tauri 人格形象不一致。
- 若桌面先做只读结果页，也要保留重测和应用到计划的路由位置。

## Route Changes

- `2026-05`: 入口从计划页迁移到侧边栏。
- `2026-05`: 首次引导加入可跳过 Croc BTI。
- `2026-05`: 计划推荐从模式权重扩展为“每日时间 -> 题数 -> 用户微调 -> 应用”。
- `2026-05`: 题型推荐从展示型扩展为可应用的 `questionTypeWeightsByMode`。
- `2026-05`: 视觉职业名与人格 title 解耦，title 保持原人格名。

## Known Pitfalls

- Do not blame Croc BTI minute budgeting for manual Plan caps or Today target shrinkage without tracing the Plan save path and Today target seed path first.
- 不要在现有 Croc BTI 模式配额之外直接追加高频词题量；应从相近的复习配额拆分，否则“10 分钟约 40 题”的总预算会漂移。
- 不要让 Croc BTI 结果直接绕过 plan save/apply 写 Today 或 Supabase。
- 不要只更新今日首页而忘记计划页。
- 不要让计划名称继续保持旧计划名；应用 Croc BTI 后应使用当前人格名称。
- 不要把 `newWord` 放入 `questionTypeWeightsByMode`；新词学习必须保持固定四题型顺序。
- 不要把 `rootAffix` 放入普通题型占比；词根词缀保持旧流程。
- 不要让题型备选项重复率变高或出现乱码。
- 不要破坏例句高亮；新增无翻译例句题时尤其要复查高亮区间。
- 不要让实际学习进度与首页进度分开计算。
- 不要让计划页显示旧计划，首页显示新计划。
- 不要在图片生成不稳定时继续依赖提示词；使用本地确认过的形象资产。
- 不要让滑条的 thumb 和进度条使用不同归一化标准。

## Verification

- Mobile/shared (`2026-08-11`): Diagnosis was covered by the learning-flow regressions for Plan target caps and Today target seed preservation; no Croc BTI scoring/model code was changed in this turn.
- Mobile (`2026-08-06`): `flutter test test\croc_bti_model_test.dart test\today_task_breakdown_test.dart` passed 17/17. The daily-minutes regression again asserts totals of 40 and 80 questions after high-frequency allocation is split from review.
- Mobile (`2026-08-06`): targeted `flutter analyze --no-pub` over the eight changed Plan/Croc/Study/Today/Reports Dart files passed with no issues.
- Mobile: 历史 `flutter analyze --no-pub` 通过；历史 `flutter test test/croc_bti_model_test.dart` 覆盖人格映射、题量时间换算、题型占比归一化、账号 scope 隔离、重测清理。最近一次本线程中 `git diff --check` 通过；`dart analyze` 曾因 Flutter/Dart 工具卡住超时，未给出诊断。
- Desktop: Pending; 尚无 Tauri UI 验证。
- Shared/domain: 需要持续覆盖四轴评分、模式权重总和 100、每日时间题量、`newWordsPerDay` 为 4 的倍数、过滤 `newWord/rootAffix`、应用后 Today/Plan 同步。

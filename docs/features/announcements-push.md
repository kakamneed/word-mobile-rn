# Feature: Announcements And Push

> Slug: `announcements-push`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

让运营公告不需要发布新版 App 就能到达用户。根据当前实现和对话结论，第一阶段是应用内公告：用户打开或刷新 App 时，从云端读取当前有效公告。系统推送是后续扩展，用于用户不打开 App 时也能收到提醒。

## UX Contract

- 应用内公告显示在 Today 首页，以非阻塞 banner 形式出现。
- 公告请求失败、Supabase 未配置、离线、RLS 拒绝或云端不可用时，用户仍然可以正常学习。
- 用户可以关闭某条公告；关闭状态先保存在本机。
- 当前移动端只展示最高优先级的一条可见公告，不是公告历史列表。
- 旧公告是否继续可见由云端 `is_active`、时间窗、平台、优先级共同决定。
- 发布新公告时，推荐先让旧公告 `is_active = false`，再插入一条新公告。
- 系统推送不属于第一版公告 UX，不要求通知权限，也不要求 FCM/APNs。

## Shared Domain/Data Contract

公告源是 Supabase `public.announcements`，不是 Rust 本地学习真相。

Shared fields:

- `id`: 公告唯一 ID；客户端本地 dismiss 也按这个 ID 记录。
- `title`, `body`: 用户可见内容。
- `level`: `info`, `success`, `warning`, `critical`。
- `priority`: 数字越大越优先展示。
- `is_active`: 云端总开关。
- `published_at`, `starts_at`, `ends_at`: 发布时间和有效时间窗。
- `target_platform`: `all`, `android`, `ios`。
- `min_app_version`, `max_app_version`: 预留给后续版本定向。

Client visibility rule:

- 只读取/展示 `is_active = true` 的公告。
- `published_at <= now()`。
- `starts_at` 为空或已经开始。
- `ends_at` 为空或尚未过期。
- `target_platform` 为当前平台或 `all`。
- 本机未 dismiss。
- 当前移动端取 `priority desc, published_at desc` 后的第一条。

Operator write path:

- 当前使用 Supabase SQL Editor 或可信 service-role 路径写入。
- 移动端只使用 anon client 读取，不允许写公告。
- 参考 `supabase/announcements-admin.sql` 发布、隐藏、查询公告。

Recommended publish flow:

```sql
update public.announcements
set is_active = false
where is_active = true;

insert into public.announcements (
  title,
  body,
  level,
  priority,
  is_active,
  target_platform,
  published_at
)
values (
  '新的公告标题',
  '新的公告内容',
  'info',
  10,
  true,
  'all',
  now()
);
```

Do not reuse an old announcement ID for new content. If a user dismissed that ID locally, they may not see the updated content.

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- SDK/service calls: `apps/flutter_mobile/lib/supabase/announcement_service.dart`
- Loading/cache/reload behavior:
  - Today bundle 加载时 best-effort 拉取公告。
  - Supabase 未配置时返回空列表。
  - 拉取失败不阻塞 Today。
  - 本机 `SharedPreferences` 保存 dismissed announcement ids。
- Orientation/gesture constraints:
  - 手机竖屏优先，banner 必须紧凑。
  - 关闭按钮必须容易点，不应覆盖学习主操作。
- First implementation slice:
  - Supabase 表/RLS。
  - Flutter service。
  - Today banner。
  - 本机 dismiss。
- Current status:
  - 应用内公告已实现。
  - 自动 Flutter analyze/test 曾受 Windows Dart 子进程权限问题阻塞，需要在可运行工具链环境里重跑。

## Tauri Desktop Route

- Owner view/window: desktop home/dashboard 顶部公告区，或全局消息中心。
- Shared APIs to reuse:
  - Supabase `public.announcements` contract。
  - 与移动端相同的 active/time/platform/version filtering。
  - 本地 dismissed ids。
- Desktop-specific layout:
  - 顶部 banner 可更宽，支持多行文本。
  - 可以比移动端更早引入公告中心列表，但第一 parity slice 仍只需要显示最高优先级当前公告。
- Mobile assumptions to avoid:
  - 不要把公告绑定到 Today 卡片布局。
  - 不要依赖移动端的下拉刷新手势。
  - 不要复用移动端小屏 spacing。
- First parity slice:
  - 桌面启动或 home refresh 时读取 active announcements。
  - 显示最高优先级一条。
  - 支持本地 dismiss。
- Current status:
  - Planned。

## Sync And Storage

- 公告内容由云端负责，客户端不把公告写入 Rust/SQLite 学习数据。
- Dismissed IDs 先本地保存；这意味着同一用户换设备后可能再次看到同一公告。
- 如果未来需要跨设备已读状态，可新增 user-scoped `announcement_receipts` 表。
- 旧公告失效推荐做法：先把旧公告 `is_active = false`，再插入新公告；不要复用旧 ID，否则点过关闭的用户可能看不到新内容。
- 公告 history/list 不是第一版能力；如果需要，应作为公告中心扩展。

## AI Or Provider Implications

无 AI/provider 依赖。公告内容由运营直接提供。若未来用 AI 辅助写公告，只能作为后台运营工具，不应进入客户端运行路径。

## Implementation Log

- `2026-05-05`: 添加 Supabase `announcements` migration、Flutter `AnnouncementService`、本地 dismiss、Today banner、测试和 Supabase 文档。
- `2026-05-05`: `flutter test` / `flutter analyze` 曾被 Windows Dart subprocess 权限问题阻塞；`dart.exe format` 和 `git diff --check` 可通过。
- `2026-06-25`: 补全云端 setup：`supabase/cloud-setup.sql`、`supabase/announcements-admin.sql`、`docs/supabase/announcements.md`。
- `2026-06-25`: 根据对话历史整理跨平台 feature ledger，明确应用内公告和 OS push 的边界。
- `2026-06-25`: 明确发布新公告的推荐方式：旧公告置 inactive，然后 insert 新 row。

## Mobile Lessons Learned

- 公告必须 best-effort，不能阻塞 Today 加载。
- 移动端只展示一条最高优先级公告，避免首页噪音。
- 本地 dismiss 绑定公告 `id`，所以新公告应插入新 row，不应复用旧 row。
- RLS 必须允许普通客户端读取 active/published rows，但不能允许普通客户端写公告。
- `target_platform` 需要客户端再次过滤，不能只依赖 SQL 查询。

## Desktop Follow-Up Notes

桌面端应复用同一云端表和可见性规则，但 UI 不必复制移动端 Today banner。优先做 home/dashboard banner，后续再考虑消息中心。桌面端如果引入公告历史列表，要保持“当前公告”和“历史公告”概念分离，避免移动端第一版 UX 被误读为完整公告系统。

## Route Changes

- `2026-06-25`: Push 被明确降级为后续扩展；当前 feature 的 shipped slice 是 in-app announcements。
- `2026-06-25`: 发布新公告推荐从“更新旧 row”改为“旧 row 置 inactive + 插入新 row”，避免本地 dismissed ID 影响新内容。

## Known Pitfalls

- 不要把 service-role key 放进 Flutter 或 Tauri 客户端。
- 不要因为公告请求失败影响本地学习。
- 不要把 OS push 和应用内公告混成同一个 first slice；推送需要通知权限、设备 token、FCM/APNs 或平台服务，复杂度完全不同。
- 不要复用已被用户 dismiss 的公告 ID 发布新内容。
- 不要默认展示所有历史公告；当前移动端 UX 只展示当前最高优先级公告。

## Verification

- Mobile:
  - `apps/flutter_mobile/test/announcement_service_test.dart` 覆盖时间窗、平台过滤、本地 dismiss。
  - 需要在 Dart/Flutter 子进程权限正常的环境里重跑 `flutter analyze --no-pub` 和相关 test。
- Desktop:
  - Pending。首个 parity slice 完成后应验证桌面 home/banner 拉取、过滤、dismiss。
- Shared/domain:
  - `supabase/migrations/202605050002_cloud_announcements.sql` 定义表、索引、trigger、RLS。
  - `tests/supabase/announcements_rls_smoke.sql` 记录 RLS smoke 预期。
  - 需要在目标 Supabase 项目执行 migration 或 `supabase/cloud-setup.sql` 后，用 `supabase/announcements-admin.sql` 做云端 smoke。

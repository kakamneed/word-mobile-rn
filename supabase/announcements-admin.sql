-- Operator snippets for cloud announcements.
-- Run in Supabase Dashboard SQL Editor or through a trusted service-role path.
-- Do not run these from the mobile app.

-- 1. Publish a currently visible announcement.
-- Replace the title and body with the message you want users to see.
insert into public.announcements (
  title,
  body,
  level,
  priority,
  is_active,
  target_platform,
  published_at,
  starts_at,
  ends_at
)
values (
  'Announcement title',
  'Announcement body. Users will see this after opening or refreshing the app.',
  'info',
  10,
  true,
  'all',
  now(),
  null,
  null
)
returning id, title, level, priority, is_active, published_at;

-- 2. Hide an announcement after it is no longer needed.
-- update public.announcements
-- set is_active = false
-- where id = '<announcement-id>';

-- 3. Verify what ordinary clients are allowed to read.
select
  id,
  title,
  body,
  level,
  priority,
  target_platform,
  published_at,
  starts_at,
  ends_at
from public.announcements
where
  is_active = true
  and published_at <= now()
  and (starts_at is null or starts_at <= now())
  and (ends_at is null or ends_at > now())
order by priority desc, published_at desc
limit 10;

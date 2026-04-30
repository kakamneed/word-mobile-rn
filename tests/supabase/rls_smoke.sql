-- Smoke checks for the initial Supabase RLS contract.
-- Run after `supabase db reset` with a test harness that can create two auth
-- users and set request.jwt.claim.sub for each user.

-- Required paths:
-- - unauthenticated select from public.profiles returns no protected rows
-- - user A cannot select rows where user_id = user B
-- - user A cannot insert rows with user_id = user B
-- - user A can call public.register_device for its own auth.uid()
-- - public.sync_dead_letters has no ordinary client insert/update/delete path

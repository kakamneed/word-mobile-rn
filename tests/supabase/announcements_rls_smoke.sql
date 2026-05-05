-- Smoke checks for public cloud announcements.
-- Run after applying `202605050002_cloud_announcements.sql`.

-- Expected behavior:
-- - anonymous and authenticated clients can select active rows whose publish
--   window includes now()
-- - inactive, future, and expired rows are hidden from ordinary clients
-- - ordinary clients cannot insert, update, or delete announcement rows
-- - operator writes happen through the Supabase dashboard, CLI, or another
--   service-role controlled path, never from the mobile app

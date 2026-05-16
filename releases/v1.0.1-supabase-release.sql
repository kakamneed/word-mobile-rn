insert into public.app_releases (
  platform,
  runtime,
  channel,
  version_name,
  version_code,
  min_supported_code,
  force_update,
  download_url,
  sha256,
  release_notes,
  rollout_percent,
  enabled
) values (
  'android',
  'flutter',
  'stable',
  '1.0.1',
  2,
  1,
  false,
  'https://github.com/kakamneed/word-mobile-rn/releases/download/v1.0.1/word-mobile-1.0.1%2B2.apk',
  '7489e99900a296ff5d950383d01bd10e25f62807514a2c6a05ba2b740b036fa0',
  'Adds Supabase-driven APK update checks and a manual Check for updates action in the account drawer.',
  100,
  true
);

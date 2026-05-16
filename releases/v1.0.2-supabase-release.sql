delete from public.app_releases
where platform = 'android'
  and runtime = 'flutter'
  and channel = 'stable'
  and version_name = '1.0.2'
  and version_code = 3;

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
  '1.0.2',
  3,
  1,
  false,
  'https://github.com/kakamneed/word-mobile-rn/releases/download/v1.0.2/word-mobile-1.0.2%2B3.apk',
  '6231349135b914b4e49b4b0a800ee26a01b9f540b15ad73a7b676aad20f5531b',
  'Preserves authoritative study choice identity, tightens malformed choice DTO handling, aligns Today fallback targets, and improves AI wrong-word import fallback coverage.',
  100,
  true
);

notify pgrst, 'reload schema';

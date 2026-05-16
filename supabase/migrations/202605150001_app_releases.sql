set check_function_bodies = off;

create table if not exists public.app_releases (
  id uuid primary key default gen_random_uuid(),
  platform text not null,
  runtime text not null default 'flutter',
  channel text not null default 'stable',
  version_name text not null check (char_length(trim(version_name)) > 0),
  version_code integer not null check (version_code > 0),
  min_supported_code integer not null default 0 check (min_supported_code >= 0),
  force_update boolean not null default false,
  apk_path text,
  download_url text,
  sha256 text,
  release_notes text not null default '',
  rollout_percent integer not null default 100 check (rollout_percent between 0 and 100),
  enabled boolean not null default false,
  published_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint app_releases_platform_check
    check (platform in ('android', 'ios')),
  constraint app_releases_runtime_check
    check (runtime in ('flutter')),
  constraint app_releases_channel_check
    check (channel in ('stable', 'beta', 'internal')),
  constraint app_releases_android_artifact_check
    check (
      platform <> 'android'
      or apk_path is not null
      or download_url is not null
    ),
  constraint app_releases_sha256_check
    check (sha256 is null or sha256 ~ '^[A-Fa-f0-9]{64}$')
);

create index if not exists app_releases_lookup_idx
  on public.app_releases (
    platform,
    runtime,
    channel,
    enabled,
    published_at desc,
    version_code desc
  );

drop trigger if exists app_releases_set_updated_at
  on public.app_releases;

create trigger app_releases_set_updated_at
before update on public.app_releases
for each row execute function public.set_updated_at();

alter table public.app_releases enable row level security;

drop policy if exists app_releases_public_enabled_select
  on public.app_releases;

create policy app_releases_public_enabled_select
on public.app_releases
for select
using (
  enabled = true
  and published_at <= now()
);

notify pgrst, 'reload schema';

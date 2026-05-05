set check_function_bodies = off;

create table if not exists public.announcements (
  id uuid primary key default gen_random_uuid(),
  title text not null check (char_length(trim(title)) > 0),
  body text not null check (char_length(trim(body)) > 0),
  level text not null default 'info',
  priority integer not null default 0,
  is_active boolean not null default false,
  published_at timestamptz not null default now(),
  starts_at timestamptz,
  ends_at timestamptz,
  target_platform text not null default 'all',
  min_app_version text,
  max_app_version text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint announcements_level_check
    check (level in ('info', 'success', 'warning', 'critical')),
  constraint announcements_target_platform_check
    check (target_platform in ('all', 'android', 'ios')),
  constraint announcements_time_window_check
    check (ends_at is null or starts_at is null or ends_at > starts_at)
);

create index if not exists announcements_active_lookup_idx
  on public.announcements (
    is_active,
    target_platform,
    priority desc,
    published_at desc
  );

create index if not exists announcements_active_window_idx
  on public.announcements (published_at, starts_at, ends_at)
  where is_active = true;

create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

drop trigger if exists announcements_set_updated_at
  on public.announcements;

create trigger announcements_set_updated_at
before update on public.announcements
for each row execute function public.set_updated_at();

alter table public.announcements enable row level security;

drop policy if exists announcements_public_active_select
  on public.announcements;

create policy announcements_public_active_select
on public.announcements
for select
using (
  is_active = true
  and published_at <= now()
  and (starts_at is null or starts_at <= now())
  and (ends_at is null or ends_at > now())
);

notify pgrst, 'reload schema';

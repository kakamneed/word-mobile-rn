set check_function_bodies = off;

create extension if not exists pgcrypto;

create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create table public.profiles (
  user_id uuid primary key references auth.users(id) on delete cascade,
  display_name text,
  locale text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.devices (
  device_id uuid primary key,
  user_id uuid not null references auth.users(id) on delete cascade,
  platform text not null,
  device_label text,
  app_version text,
  last_seen_at timestamptz not null default now(),
  revoked_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.plan_configs (
  plan_id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  name text not null default 'Default plan',
  new_words_per_day integer not null default 0 check (new_words_per_day >= 0),
  review_words_per_day integer not null default 0 check (review_words_per_day >= 0),
  mixed_test_per_day integer not null default 0 check (mixed_test_per_day >= 0),
  wrong_word_test_per_day integer not null default 0 check (wrong_word_test_per_day >= 0),
  root_affix_per_day integer check (root_affix_per_day is null or root_affix_per_day >= 0),
  growth_rule_mode text,
  shared_growth_rule jsonb,
  growth_rules_by_mode jsonb,
  version bigint not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.wordbook_preferences (
  user_id uuid not null references auth.users(id) on delete cascade,
  wordbook_id bigint not null,
  is_active boolean not null default false,
  updated_at timestamptz not null default now(),
  primary key (user_id, wordbook_id)
);

create table public.study_events (
  event_id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  device_id uuid not null references public.devices(device_id),
  session_id text not null,
  event_type text not null check (
    event_type in (
      'session_started',
      'answer_submitted',
      'session_completed',
      'session_cancelled'
    )
  ),
  payload_json jsonb not null,
  occurred_at timestamptz not null,
  ingested_at timestamptz not null default now(),
  idempotency_key text not null unique
);

create table public.study_word_points (
  user_id uuid not null references auth.users(id) on delete cascade,
  point_date date not null,
  entry_id bigint not null,
  mode text not null,
  question_type text not null default 'unknown',
  attempt_count integer not null default 0 check (attempt_count >= 0),
  correct_count integer not null default 0 check (correct_count >= 0),
  wrong_count integer not null default 0 check (wrong_count >= 0),
  total_response_time_ms bigint not null default 0 check (total_response_time_ms >= 0),
  last_answered_at timestamptz not null,
  updated_at timestamptz not null default now(),
  primary key (user_id, point_date, entry_id, mode, question_type)
);

create table public.sync_cursors (
  user_id uuid not null references auth.users(id) on delete cascade,
  device_id uuid not null references public.devices(device_id),
  last_pushed_event_at timestamptz,
  last_pulled_server_cursor text,
  updated_at timestamptz not null default now(),
  primary key (user_id, device_id)
);

create table public.sync_dead_letters (
  dead_letter_id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  device_id uuid not null references public.devices(device_id),
  idempotency_key text not null,
  payload_json jsonb not null,
  error_code text not null,
  created_at timestamptz not null default now()
);

create table public.wrong_word_entries (
  user_id uuid not null references auth.users(id) on delete cascade,
  entry_id bigint not null,
  error_count integer not null default 0 check (error_count >= 0),
  last_wrong_at timestamptz not null,
  priority_score numeric not null default 0,
  hint_text text not null default '',
  hint_source text not null default '',
  hint_updated_at timestamptz,
  projection_version bigint not null default 1,
  updated_at timestamptz not null default now(),
  primary key (user_id, entry_id)
);

create table public.ai_passages (
  passage_id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  title text,
  payload_json jsonb not null,
  validation_status text not null default 'pending',
  generated_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.report_snapshots (
  user_id uuid not null references auth.users(id) on delete cascade,
  snapshot_date date not null,
  payload_json jsonb not null,
  updated_at timestamptz not null default now(),
  primary key (user_id, snapshot_date)
);

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

create index devices_user_id_idx on public.devices(user_id);
create index devices_user_active_idx on public.devices(user_id, revoked_at);
create index plan_configs_user_id_idx on public.plan_configs(user_id);
create unique index if not exists plan_configs_user_id_unique on public.plan_configs(user_id);
create index study_events_user_time_idx on public.study_events(user_id, occurred_at);
create index study_events_device_time_idx on public.study_events(device_id, occurred_at);
create index study_word_points_user_date_idx on public.study_word_points(user_id, point_date);
create index sync_dead_letters_user_created_idx on public.sync_dead_letters(user_id, created_at);
create index ai_passages_user_generated_idx on public.ai_passages(user_id, generated_at);
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

create trigger profiles_set_updated_at
before update on public.profiles
for each row execute function public.set_updated_at();

create trigger devices_set_updated_at
before update on public.devices
for each row execute function public.set_updated_at();

create trigger plan_configs_set_updated_at
before update on public.plan_configs
for each row execute function public.set_updated_at();

create trigger study_word_points_set_updated_at
before update on public.study_word_points
for each row execute function public.set_updated_at();

create trigger wrong_word_entries_set_updated_at
before update on public.wrong_word_entries
for each row execute function public.set_updated_at();

create trigger announcements_set_updated_at
before update on public.announcements
for each row execute function public.set_updated_at();

create or replace function public.handle_new_user_profile()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  insert into public.profiles (user_id, display_name, locale)
  values (
    new.id,
    nullif(new.raw_user_meta_data ->> 'display_name', ''),
    nullif(new.raw_user_meta_data ->> 'locale', '')
  )
  on conflict (user_id) do nothing;
  return new;
end;
$$;

create trigger on_auth_user_created_profile
after insert on auth.users
for each row execute function public.handle_new_user_profile();

create or replace function public.register_device(
  p_device_id uuid,
  p_platform text,
  p_device_label text default null,
  p_app_version text default null
)
returns public.devices
language plpgsql
security invoker
as $$
declare
  v_device public.devices;
begin
  if auth.uid() is null then
    raise exception 'register_device requires authentication'
      using errcode = '28000';
  end if;

  insert into public.devices (
    device_id,
    user_id,
    platform,
    device_label,
    app_version,
    last_seen_at,
    revoked_at
  )
  values (
    p_device_id,
    auth.uid(),
    p_platform,
    p_device_label,
    p_app_version,
    now(),
    null
  )
  on conflict (device_id) do update
  set
    platform = excluded.platform,
    device_label = excluded.device_label,
    app_version = excluded.app_version,
    last_seen_at = now(),
    revoked_at = null
  where public.devices.user_id = auth.uid()
  returning * into v_device;

  if v_device.device_id is null then
    raise exception 'device belongs to another user'
      using errcode = '42501';
  end if;

  return v_device;
end;
$$;

create or replace function public.revoke_device(p_device_id uuid)
returns public.devices
language plpgsql
security invoker
as $$
declare
  v_device public.devices;
begin
  if auth.uid() is null then
    raise exception 'revoke_device requires authentication'
      using errcode = '28000';
  end if;

  update public.devices
  set revoked_at = now(), last_seen_at = now()
  where device_id = p_device_id and user_id = auth.uid()
  returning * into v_device;

  if v_device.device_id is null then
    raise exception 'device not found for current user'
      using errcode = '42501';
  end if;

  return v_device;
end;
$$;

alter table public.profiles enable row level security;
alter table public.devices enable row level security;
alter table public.plan_configs enable row level security;
alter table public.wordbook_preferences enable row level security;
alter table public.study_events enable row level security;
alter table public.study_word_points enable row level security;
alter table public.sync_cursors enable row level security;
alter table public.sync_dead_letters enable row level security;
alter table public.wrong_word_entries enable row level security;
alter table public.ai_passages enable row level security;
alter table public.report_snapshots enable row level security;
alter table public.announcements enable row level security;

create policy profiles_owner_select on public.profiles
for select using (auth.uid() = user_id);

create policy profiles_owner_insert on public.profiles
for insert with check (auth.uid() = user_id);

create policy profiles_owner_update on public.profiles
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy profiles_owner_delete on public.profiles
for delete using (auth.uid() = user_id);

create policy devices_owner_select on public.devices
for select using (auth.uid() = user_id);

create policy devices_owner_insert on public.devices
for insert with check (auth.uid() = user_id);

create policy devices_owner_update on public.devices
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy plan_configs_owner_select on public.plan_configs
for select using (auth.uid() = user_id);

create policy plan_configs_owner_insert on public.plan_configs
for insert with check (auth.uid() = user_id);

create policy plan_configs_owner_update on public.plan_configs
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy plan_configs_owner_delete on public.plan_configs
for delete using (auth.uid() = user_id);

create policy wordbook_preferences_owner_select on public.wordbook_preferences
for select using (auth.uid() = user_id);

create policy wordbook_preferences_owner_insert on public.wordbook_preferences
for insert with check (auth.uid() = user_id);

create policy wordbook_preferences_owner_update on public.wordbook_preferences
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy wordbook_preferences_owner_delete on public.wordbook_preferences
for delete using (auth.uid() = user_id);

create policy study_events_owner_select on public.study_events
for select using (auth.uid() = user_id);

create policy study_events_owner_insert on public.study_events
for insert with check (
  auth.uid() = user_id
  and exists (
    select 1
    from public.devices
    where devices.device_id = study_events.device_id
      and devices.user_id = auth.uid()
      and devices.revoked_at is null
  )
);

create policy study_word_points_owner_select on public.study_word_points
for select using (auth.uid() = user_id);

create policy study_word_points_owner_insert on public.study_word_points
for insert with check (auth.uid() = user_id);

create policy study_word_points_owner_update on public.study_word_points
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy sync_cursors_owner_select on public.sync_cursors
for select using (auth.uid() = user_id);

create policy sync_cursors_owner_insert on public.sync_cursors
for insert with check (auth.uid() = user_id);

create policy sync_cursors_owner_update on public.sync_cursors
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy wrong_word_entries_owner_select on public.wrong_word_entries
for select using (auth.uid() = user_id);

create policy wrong_word_entries_owner_insert on public.wrong_word_entries
for insert with check (auth.uid() = user_id);

create policy wrong_word_entries_owner_update on public.wrong_word_entries
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy ai_passages_owner_select on public.ai_passages
for select using (auth.uid() = user_id);

create policy ai_passages_owner_insert on public.ai_passages
for insert with check (auth.uid() = user_id);

create policy ai_passages_owner_update on public.ai_passages
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy ai_passages_owner_delete on public.ai_passages
for delete using (auth.uid() = user_id);

create policy report_snapshots_owner_select on public.report_snapshots
for select using (auth.uid() = user_id);

create policy report_snapshots_owner_insert on public.report_snapshots
for insert with check (auth.uid() = user_id);

create policy report_snapshots_owner_update on public.report_snapshots
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy announcements_public_active_select on public.announcements
for select using (
  is_active = true
  and published_at <= now()
  and (starts_at is null or starts_at <= now())
  and (ends_at is null or ends_at > now())
);

create table if not exists public.word_comments (
  id uuid primary key default gen_random_uuid(),
  entry_source_id text not null check (char_length(trim(entry_source_id)) > 0),
  word text not null default '',
  user_id uuid not null references public.profiles(user_id) on delete cascade,
  body text not null check (
    char_length(trim(body)) > 0
    and char_length(body) <= 500
  ),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists word_comments_entry_created_idx
  on public.word_comments (entry_source_id, created_at desc);

create index if not exists word_comments_user_idx
  on public.word_comments (user_id, created_at desc);

drop trigger if exists word_comments_set_updated_at
  on public.word_comments;

create trigger word_comments_set_updated_at
before update on public.word_comments
for each row execute function public.set_updated_at();

alter table public.word_comments enable row level security;

drop policy if exists word_comments_select_all
  on public.word_comments;

create policy word_comments_select_all
on public.word_comments
for select
using (true);

drop policy if exists word_comments_insert_own
  on public.word_comments;

create policy word_comments_insert_own
on public.word_comments
for insert
with check (auth.uid() = user_id);

drop policy if exists word_comments_update_own
  on public.word_comments;

create policy word_comments_update_own
on public.word_comments
for update
using (auth.uid() = user_id)
with check (auth.uid() = user_id);

drop policy if exists word_comments_delete_own
  on public.word_comments;

create policy word_comments_delete_own
on public.word_comments
for delete
using (auth.uid() = user_id);

insert into storage.buckets (id, name, public)
values ('ai-passages', 'ai-passages', false)
on conflict (id) do nothing;

create policy ai_passages_bucket_owner_select on storage.objects
for select using (
  bucket_id = 'ai-passages'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy ai_passages_bucket_owner_insert on storage.objects
for insert with check (
  bucket_id = 'ai-passages'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy ai_passages_bucket_owner_update on storage.objects
for update using (
  bucket_id = 'ai-passages'
  and auth.uid()::text = (storage.foldername(name))[1]
) with check (
  bucket_id = 'ai-passages'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy ai_passages_bucket_owner_delete on storage.objects
for delete using (
  bucket_id = 'ai-passages'
  and auth.uid()::text = (storage.foldername(name))[1]
);

notify pgrst, 'reload schema';

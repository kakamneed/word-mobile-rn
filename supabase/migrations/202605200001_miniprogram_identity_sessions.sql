set check_function_bodies = off;

create extension if not exists pgcrypto;

create table if not exists public.internal_accounts (
  internal_user_id uuid primary key default gen_random_uuid(),
  status text not null default 'active' check (
    status in ('active', 'disabled', 'deleted_pending', 'deleted')
  ),
  primary_identity text not null default 'wechat_mp' check (
    primary_identity in ('wechat_mp', 'wechat_app', 'email')
  ),
  supabase_owner_user_id uuid references auth.users(id) on delete set null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  last_login_at timestamptz
);

create table if not exists public.user_identities (
  identity_id uuid primary key default gen_random_uuid(),
  internal_user_id uuid not null references public.internal_accounts(internal_user_id) on delete cascade,
  provider text not null check (provider in ('wechat_mp', 'wechat_app', 'email')),
  provider_subject text not null,
  provider_subject_secondary text,
  supabase_user_id uuid references auth.users(id) on delete set null,
  normalized_email text,
  is_verified boolean not null default false,
  metadata_json jsonb not null default '{}'::jsonb,
  bound_at timestamptz not null default now(),
  last_seen_at timestamptz not null default now(),
  unique (provider, provider_subject)
);

create unique index if not exists user_identities_email_verified_unique
  on public.user_identities (normalized_email)
  where provider = 'email' and is_verified = true and normalized_email is not null;

create unique index if not exists user_identities_wechat_unionid_unique
  on public.user_identities (provider_subject_secondary)
  where provider_subject_secondary is not null and is_verified = true;

create table if not exists public.miniprogram_sessions (
  session_id uuid primary key default gen_random_uuid(),
  internal_user_id uuid not null references public.internal_accounts(internal_user_id) on delete cascade,
  refresh_token_hash text not null unique,
  audience text not null default 'word-mobile-miniprogram',
  issued_at timestamptz not null default now(),
  expires_at timestamptz not null,
  revoked_at timestamptz,
  client_json jsonb not null default '{}'::jsonb,
  last_seen_at timestamptz not null default now()
);

create table if not exists public.email_bind_challenges (
  challenge_id uuid primary key default gen_random_uuid(),
  internal_user_id uuid not null references public.internal_accounts(internal_user_id) on delete cascade,
  normalized_email text not null,
  otp_hash text not null,
  expires_at timestamptz not null,
  consumed_at timestamptz,
  created_at timestamptz not null default now(),
  attempt_count integer not null default 0 check (attempt_count >= 0)
);

create table if not exists public.account_merge_decisions (
  merge_decision_id uuid primary key default gen_random_uuid(),
  source_internal_user_id uuid not null references public.internal_accounts(internal_user_id) on delete cascade,
  target_internal_user_id uuid not null references public.internal_accounts(internal_user_id) on delete cascade,
  state text not null default 'pending' check (
    state in ('pending', 'confirmed', 'cancelled', 'expired')
  ),
  preview_json jsonb not null,
  created_at timestamptz not null default now(),
  decided_at timestamptz
);

alter table public.internal_accounts enable row level security;
alter table public.user_identities enable row level security;
alter table public.miniprogram_sessions enable row level security;
alter table public.email_bind_challenges enable row level security;
alter table public.account_merge_decisions enable row level security;

drop policy if exists "internal accounts no direct client access" on public.internal_accounts;
create policy "internal accounts no direct client access"
  on public.internal_accounts
  for all
  using (false)
  with check (false);

drop policy if exists "user identities no direct client access" on public.user_identities;
create policy "user identities no direct client access"
  on public.user_identities
  for all
  using (false)
  with check (false);

drop policy if exists "miniprogram sessions no direct client access" on public.miniprogram_sessions;
create policy "miniprogram sessions no direct client access"
  on public.miniprogram_sessions
  for all
  using (false)
  with check (false);

drop policy if exists "email bind challenges no direct client access" on public.email_bind_challenges;
create policy "email bind challenges no direct client access"
  on public.email_bind_challenges
  for all
  using (false)
  with check (false);

drop policy if exists "account merge decisions no direct client access" on public.account_merge_decisions;
create policy "account merge decisions no direct client access"
  on public.account_merge_decisions
  for all
  using (false)
  with check (false);

create index if not exists internal_accounts_supabase_owner_idx
  on public.internal_accounts (supabase_owner_user_id);

create index if not exists user_identities_internal_user_idx
  on public.user_identities (internal_user_id);

create index if not exists miniprogram_sessions_internal_user_idx
  on public.miniprogram_sessions (internal_user_id);

create index if not exists email_bind_challenges_internal_user_idx
  on public.email_bind_challenges (internal_user_id);

drop trigger if exists internal_accounts_set_updated_at on public.internal_accounts;
create trigger internal_accounts_set_updated_at
  before update on public.internal_accounts
  for each row execute function public.set_updated_at();

select pg_notify('pgrst', 'reload schema');

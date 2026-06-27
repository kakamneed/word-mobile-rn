create table if not exists public.daily_reward_claims (
  user_id uuid not null references auth.users(id) on delete cascade,
  reward_date date not null,
  reward_id text not null,
  reward_title text not null default '',
  reward_image_url text not null default '',
  reward_mime_type text not null default 'image/webp',
  claimed_at timestamptz not null default now(),
  primary key (user_id, reward_date)
);

create index if not exists daily_reward_claims_user_claimed_idx
  on public.daily_reward_claims (user_id, claimed_at desc);

alter table public.daily_reward_claims enable row level security;

drop policy if exists daily_reward_claims_owner_select on public.daily_reward_claims;
drop policy if exists daily_reward_claims_owner_insert on public.daily_reward_claims;

create policy daily_reward_claims_owner_select on public.daily_reward_claims
for select using (auth.uid() = user_id);

create policy daily_reward_claims_owner_insert on public.daily_reward_claims
for insert with check (auth.uid() = user_id);

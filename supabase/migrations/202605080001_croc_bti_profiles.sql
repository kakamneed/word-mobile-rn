set check_function_bodies = off;

create or replace function public.set_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create table if not exists public.croc_bti_profiles (
  user_id uuid primary key references auth.users(id) on delete cascade,
  result_code text not null,
  title text not null default '',
  summary text not null default '',
  advice text not null default '',
  answers_json jsonb not null default '{}'::jsonb,
  axis_scores_json jsonb not null default '{}'::jsonb,
  weights_json jsonb not null default '{}'::jsonb,
  plan_input_json jsonb not null default '{}'::jsonb,
  question_type_weights_json jsonb not null default '{}'::jsonb,
  daily_learning_minutes integer not null default 40 check (
    daily_learning_minutes between 10 and 240
  ),
  source text not null default 'croc_bti',
  version bigint not null default 1,
  evaluated_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint croc_bti_profiles_result_code_valid check (
    result_code ~ '^[VC][IO][NR][AT]$'
  )
);

drop trigger if exists croc_bti_profiles_set_updated_at
  on public.croc_bti_profiles;

create trigger croc_bti_profiles_set_updated_at
before update on public.croc_bti_profiles
for each row execute function public.set_updated_at();

alter table public.croc_bti_profiles enable row level security;

drop policy if exists croc_bti_profiles_owner_select
  on public.croc_bti_profiles;
drop policy if exists croc_bti_profiles_owner_insert
  on public.croc_bti_profiles;
drop policy if exists croc_bti_profiles_owner_update
  on public.croc_bti_profiles;
drop policy if exists croc_bti_profiles_owner_delete
  on public.croc_bti_profiles;

create policy croc_bti_profiles_owner_select
on public.croc_bti_profiles
for select using (auth.uid() = user_id);

create policy croc_bti_profiles_owner_insert
on public.croc_bti_profiles
for insert with check (auth.uid() = user_id);

create policy croc_bti_profiles_owner_update
on public.croc_bti_profiles
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

create policy croc_bti_profiles_owner_delete
on public.croc_bti_profiles
for delete using (auth.uid() = user_id);

notify pgrst, 'reload schema';

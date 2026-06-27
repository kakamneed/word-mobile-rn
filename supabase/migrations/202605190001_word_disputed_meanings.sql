set check_function_bodies = off;

create table if not exists public.word_disputed_meanings (
  id uuid primary key default gen_random_uuid(),
  entry_source_id text not null check (char_length(trim(entry_source_id)) > 0),
  word text not null default '',
  user_id uuid not null references public.profiles(user_id) on delete cascade,
  submitted_meaning text not null check (
    char_length(trim(submitted_meaning)) > 0
    and char_length(submitted_meaning) <= 200
  ),
  question_id text not null default '',
  question_type text not null default '',
  source text not null default 'user_dispute',
  status text not null default 'pending'
    check (status in ('pending', 'accepted', 'rejected')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists word_disputed_meanings_entry_created_idx
  on public.word_disputed_meanings (entry_source_id, created_at desc);

create index if not exists word_disputed_meanings_user_idx
  on public.word_disputed_meanings (user_id, created_at desc);

drop trigger if exists word_disputed_meanings_set_updated_at
  on public.word_disputed_meanings;

create trigger word_disputed_meanings_set_updated_at
before update on public.word_disputed_meanings
for each row execute function public.set_updated_at();

alter table public.word_disputed_meanings enable row level security;

drop policy if exists word_disputed_meanings_select_own
  on public.word_disputed_meanings;

create policy word_disputed_meanings_select_own
on public.word_disputed_meanings
for select
using (auth.uid() = user_id);

drop policy if exists word_disputed_meanings_insert_own
  on public.word_disputed_meanings;

create policy word_disputed_meanings_insert_own
on public.word_disputed_meanings
for insert
with check (auth.uid() = user_id);

drop policy if exists word_disputed_meanings_update_own
  on public.word_disputed_meanings;

create policy word_disputed_meanings_update_own
on public.word_disputed_meanings
for update
using (auth.uid() = user_id)
with check (auth.uid() = user_id);

notify pgrst, 'reload schema';

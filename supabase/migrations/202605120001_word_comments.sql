set check_function_bodies = off;

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

notify pgrst, 'reload schema';

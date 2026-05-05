create table if not exists public.study_word_points (
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

create index if not exists study_word_points_user_date_idx
on public.study_word_points(user_id, point_date);

drop trigger if exists study_word_points_set_updated_at on public.study_word_points;

create trigger study_word_points_set_updated_at
before update on public.study_word_points
for each row execute function public.set_updated_at();

alter table public.study_word_points enable row level security;

drop policy if exists study_word_points_owner_select on public.study_word_points;
drop policy if exists study_word_points_owner_insert on public.study_word_points;
drop policy if exists study_word_points_owner_update on public.study_word_points;

create policy study_word_points_owner_select on public.study_word_points
for select using (auth.uid() = user_id);

create policy study_word_points_owner_insert on public.study_word_points
for insert with check (auth.uid() = user_id);

create policy study_word_points_owner_update on public.study_word_points
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

notify pgrst, 'reload schema';

-- Web accepted answers remain immutable portable events. This compatibility
-- projection increments the existing mobile aggregate schema exactly once when
-- such an event is first inserted.

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

create table if not exists public.wrong_word_entries (
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

alter table public.study_word_points enable row level security;
alter table public.wrong_word_entries enable row level security;

do $$
begin
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'study_word_points' and policyname = 'study_word_points_owner_select') then
    create policy study_word_points_owner_select on public.study_word_points for select to authenticated using ((select auth.uid()) = user_id);
  end if;
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'study_word_points' and policyname = 'study_word_points_owner_insert') then
    create policy study_word_points_owner_insert on public.study_word_points for insert to authenticated with check ((select auth.uid()) = user_id);
  end if;
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'study_word_points' and policyname = 'study_word_points_owner_update') then
    create policy study_word_points_owner_update on public.study_word_points for update to authenticated using ((select auth.uid()) = user_id) with check ((select auth.uid()) = user_id);
  end if;
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'wrong_word_entries' and policyname = 'wrong_word_entries_owner_select') then
    create policy wrong_word_entries_owner_select on public.wrong_word_entries for select to authenticated using ((select auth.uid()) = user_id);
  end if;
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'wrong_word_entries' and policyname = 'wrong_word_entries_owner_insert') then
    create policy wrong_word_entries_owner_insert on public.wrong_word_entries for insert to authenticated with check ((select auth.uid()) = user_id);
  end if;
  if not exists (select 1 from pg_policies where schemaname = 'public' and tablename = 'wrong_word_entries' and policyname = 'wrong_word_entries_owner_update') then
    create policy wrong_word_entries_owner_update on public.wrong_word_entries for update to authenticated using ((select auth.uid()) = user_id) with check ((select auth.uid()) = user_id);
  end if;
end
$$;

revoke all on table public.study_word_points from public, anon;
revoke all on table public.wrong_word_entries from public, anon;
grant select, insert, update on table public.study_word_points to authenticated;
grant select, insert, update on table public.wrong_word_entries to authenticated;

create or replace function public.mobile_entry_id_for_source(source_id text, book_id text)
returns bigint
language plpgsql
immutable
strict
set search_path = public
as $$
declare
  source_index bigint;
begin
  if source_id ~ '^CET4_3_[1-9][0-9]*$' and book_id = 'cet4' then
    source_index := substring(source_id from '^CET4_3_([1-9][0-9]*)$')::bigint;
    if source_index between 1 and 2607 then return source_index; end if;
  elsif source_id ~ '^CET6_3_[1-9][0-9]*$' and book_id = 'cet6' then
    source_index := substring(source_id from '^CET6_3_([1-9][0-9]*)$')::bigint;
    if source_index between 1 and 2345 then return 2607 + source_index; end if;
  elsif source_id ~ '^KaoYan_3_[1-9][0-9]*$' and book_id = 'kaoyan' then
    source_index := substring(source_id from '^KaoYan_3_([1-9][0-9]*)$')::bigint;
    if source_index between 1 and 4417 then return 4952 + source_index; end if;
  end if;
  return null;
exception when numeric_value_out_of_range then
  return null;
end;
$$;

create or replace function public.mobile_question_type(raw_type text)
returns text
language sql
immutable
strict
set search_path = public
as $$
  select case raw_type
    when 'enToCnChoice' then raw_type
    when 'exampleToCnChoice' then raw_type
    when 'exampleComprehensionChoice' then 'enToCnChoice'
    when 'exampleToCnChoiceNoTranslation' then raw_type
    when 'cnToEnChoice' then raw_type
    when 'enToCnInput' then raw_type
    when 'wordSkeletonInput' then raw_type
    when 'glossToRootInput' then raw_type
    when 'rootToGlossInput' then raw_type
    else null
  end
$$;

create or replace function public.project_accepted_event_to_mobile()
returns trigger
language plpgsql
security invoker
set search_path = public
as $$
declare
  event jsonb;
  mobile_entry_id bigint;
  normalized_question_type text;
  point_day date;
  answered_at timestamptz;
  elapsed_ms bigint;
  event_mode text;
  event_outcome text;
begin
  if new.domain <> 'acceptedEvent' or new.tombstone or new.payload ->> 'kind' <> 'acceptedAnswer' then
    return new;
  end if;

  event := new.payload -> 'event';
  if jsonb_typeof(event) is distinct from 'object'
    or event ->> 'entrySourceId' is distinct from event #>> '{source,entrySourceId}' then
    return new;
  end if;

  mobile_entry_id := public.mobile_entry_id_for_source(event ->> 'entrySourceId', event #>> '{source,bookId}');
  normalized_question_type := public.mobile_question_type(event ->> 'questionType');
  event_mode := event ->> 'mode';
  event_outcome := event ->> 'outcome';
  if mobile_entry_id is null
    or normalized_question_type is null
    or event #>> '{source,version}' <> '2026.1'
    or event_mode not in ('newWord', 'review', 'mixedTest', 'wrongWordReinforcement', 'highFrequency', 'rootAffix')
    or event_outcome not in ('correct', 'fuzzyCorrect', 'incorrect', 'skipped')
    or coalesce(event ->> 'localDay', '') !~ '^\d{4}-\d{2}-\d{2}$'
    or jsonb_typeof(event -> 'elapsedMs') <> 'number'
    or (event ->> 'elapsedMs') !~ '^\d+$' then
    return new;
  end if;

  begin
    point_day := (event ->> 'localDay')::date;
    answered_at := (event ->> 'acceptedAt')::timestamptz;
    elapsed_ms := (event ->> 'elapsedMs')::bigint;
  exception when others then
    return new;
  end;

  insert into public.study_word_points (
    user_id, point_date, entry_id, mode, question_type,
    attempt_count, correct_count, wrong_count, total_response_time_ms, last_answered_at
  ) values (
    new.user_id, point_day, mobile_entry_id, event_mode, normalized_question_type,
    1,
    case when event_outcome in ('correct', 'fuzzyCorrect') then 1 else 0 end,
    case when event_outcome in ('incorrect', 'skipped') then 1 else 0 end,
    elapsed_ms, answered_at
  )
  on conflict (user_id, point_date, entry_id, mode, question_type) do update set
    attempt_count = public.study_word_points.attempt_count + 1,
    correct_count = public.study_word_points.correct_count + excluded.correct_count,
    wrong_count = public.study_word_points.wrong_count + excluded.wrong_count,
    total_response_time_ms = public.study_word_points.total_response_time_ms + excluded.total_response_time_ms,
    last_answered_at = greatest(public.study_word_points.last_answered_at, excluded.last_answered_at),
    updated_at = now();

  if event_outcome = 'incorrect' then
    insert into public.wrong_word_entries (
      user_id, entry_id, error_count, last_wrong_at, priority_score
    ) values (
      new.user_id, mobile_entry_id, 1, answered_at, 1
    )
    on conflict (user_id, entry_id) do update set
      error_count = public.wrong_word_entries.error_count + 1,
      last_wrong_at = greatest(public.wrong_word_entries.last_wrong_at, excluded.last_wrong_at),
      priority_score = public.wrong_word_entries.error_count + 1,
      projection_version = public.wrong_word_entries.projection_version + 1,
      updated_at = now();
  end if;

  return new;
end;
$$;

drop trigger if exists portable_envelopes_project_mobile on public.portable_envelopes;
create trigger portable_envelopes_project_mobile
after insert on public.portable_envelopes
for each row execute function public.project_accepted_event_to_mobile();

revoke all on function public.mobile_entry_id_for_source(text, text) from public, anon;
revoke all on function public.mobile_question_type(text) from public, anon;
revoke all on function public.project_accepted_event_to_mobile() from public, anon;
grant execute on function public.mobile_entry_id_for_source(text, text) to authenticated;
grant execute on function public.mobile_question_type(text) to authenticated;

notify pgrst, 'reload schema';

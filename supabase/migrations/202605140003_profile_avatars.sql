set check_function_bodies = off;

alter table public.profiles
  add column if not exists avatar_storage_path text,
  add column if not exists avatar_url text,
  add column if not exists avatar_mime_type text;

alter table public.profiles
  drop constraint if exists profiles_avatar_mime_type_check;

alter table public.profiles
  add constraint profiles_avatar_mime_type_check
  check (
    avatar_mime_type is null
    or avatar_mime_type in ('image/jpeg', 'image/png', 'image/webp')
  );

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'avatars',
  'avatars',
  true,
  524288,
  array['image/jpeg', 'image/png', 'image/webp']
)
on conflict (id) do update
set
  public = excluded.public,
  file_size_limit = excluded.file_size_limit,
  allowed_mime_types = excluded.allowed_mime_types;

drop policy if exists avatars_bucket_public_select on storage.objects;
drop policy if exists avatars_bucket_owner_insert on storage.objects;
drop policy if exists avatars_bucket_owner_update on storage.objects;
drop policy if exists avatars_bucket_owner_delete on storage.objects;

create policy avatars_bucket_public_select on storage.objects
for select using (bucket_id = 'avatars');

create policy avatars_bucket_owner_insert on storage.objects
for insert with check (
  bucket_id = 'avatars'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy avatars_bucket_owner_update on storage.objects
for update using (
  bucket_id = 'avatars'
  and auth.uid()::text = (storage.foldername(name))[1]
) with check (
  bucket_id = 'avatars'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy avatars_bucket_owner_delete on storage.objects
for delete using (
  bucket_id = 'avatars'
  and auth.uid()::text = (storage.foldername(name))[1]
);

drop function if exists public.get_leaderboard(text, integer, text, date);

create function public.get_leaderboard(
  p_metric text default 'totalQuestions',
  p_limit integer default 50,
  p_period text default 'weekly',
  p_period_start date default current_date
)
returns table (
  rank integer,
  is_current_user boolean,
  user_id uuid,
  display_name text,
  avatar_url text,
  total_questions integer,
  correct_count integer,
  accuracy_percent numeric,
  mixed_test_total_questions integer,
  mixed_test_correct_count integer,
  mixed_test_accuracy_percent numeric,
  learned_words integer,
  current_streak_days integer,
  updated_at timestamptz
)
language sql
stable
security definer
set search_path = public
as $$
  with requested as (
    select
      coalesce(nullif(p_period, ''), 'weekly') as period,
      coalesce(
        p_period_start,
        case coalesce(nullif(p_period, ''), 'weekly')
          when 'all_time' then date '1970-01-01'
          else current_date
        end
      ) as period_start
  ),
  normalized as (
    select
      period,
      case when period = 'all_time' then date '1970-01-01' else period_start end as period_start
    from requested
    where period in ('weekly', 'monthly', 'all_time')
  ),
  scored as (
    select
      s.user_id,
      coalesce(nullif(s.display_name, ''), nullif(p.display_name, ''), 'Word learner') as display_name,
      p.avatar_url,
      s.total_questions,
      s.correct_count,
      case
        when s.total_questions > 0 then round((s.correct_count::numeric * 100.0) / s.total_questions, 1)
        else 0
      end as accuracy_percent,
      s.mixed_test_total_questions,
      s.mixed_test_correct_count,
      case
        when s.mixed_test_total_questions > 0 then round((s.mixed_test_correct_count::numeric * 100.0) / s.mixed_test_total_questions, 1)
        else 0
      end as mixed_test_accuracy_percent,
      s.learned_words,
      s.current_streak_days,
      s.updated_at,
      case p_metric
        when 'accuracy' then case when s.total_questions >= 20 then (s.correct_count::numeric * 100.0) / s.total_questions else -1 end
        when 'mixedAccuracy' then case when s.mixed_test_total_questions >= 10 then (s.mixed_test_correct_count::numeric * 100.0) / s.mixed_test_total_questions else -1 end
        when 'currentStreak' then s.current_streak_days::numeric
        else s.total_questions::numeric
      end as score,
      case p_metric
        when 'mixedAccuracy' then s.mixed_test_total_questions
        else s.total_questions
      end as tie_questions
    from public.leaderboard_stats s
    join normalized n
      on n.period = s.period
     and n.period_start = s.period_start
    left join public.profiles p on p.user_id = s.user_id
  ),
  ranked as (
    select
      row_number() over (
        order by score desc, tie_questions desc, updated_at asc, user_id asc
      )::integer as rank,
      user_id,
      display_name,
      avatar_url,
      total_questions,
      correct_count,
      accuracy_percent,
      mixed_test_total_questions,
      mixed_test_correct_count,
      mixed_test_accuracy_percent,
      learned_words,
      current_streak_days,
      updated_at
    from scored
    where score >= 0
  )
  select
    ranked.rank,
    ranked.user_id = auth.uid() as is_current_user,
    ranked.user_id,
    ranked.display_name,
    ranked.avatar_url,
    ranked.total_questions,
    ranked.correct_count,
    ranked.accuracy_percent,
    ranked.mixed_test_total_questions,
    ranked.mixed_test_correct_count,
    ranked.mixed_test_accuracy_percent,
    ranked.learned_words,
    ranked.current_streak_days,
    ranked.updated_at
  from ranked
  where ranked.rank <= greatest(1, least(coalesce(p_limit, 50), 100))
    or ranked.user_id = auth.uid()
  order by ranked.rank asc;
$$;

select pg_notify('pgrst', 'reload schema');

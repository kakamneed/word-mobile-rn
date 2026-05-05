set check_function_bodies = off;

alter table public.leaderboard_stats
  add column if not exists period text not null default 'all_time',
  add column if not exists period_start date not null default date '1970-01-01',
  add column if not exists mixed_test_total_questions integer not null default 0 check (mixed_test_total_questions >= 0),
  add column if not exists mixed_test_correct_count integer not null default 0 check (mixed_test_correct_count >= 0);

alter table public.leaderboard_stats
  drop constraint if exists leaderboard_stats_period_check;

alter table public.leaderboard_stats
  add constraint leaderboard_stats_period_check
  check (period in ('weekly', 'monthly', 'all_time'));

alter table public.leaderboard_stats
  drop constraint if exists leaderboard_mixed_correct_lte_total;

alter table public.leaderboard_stats
  add constraint leaderboard_mixed_correct_lte_total
  check (mixed_test_correct_count <= mixed_test_total_questions);

alter table public.leaderboard_stats
  drop constraint if exists leaderboard_stats_pkey;

alter table public.leaderboard_stats
  add constraint leaderboard_stats_pkey primary key (user_id, period, period_start);

drop index if exists leaderboard_stats_total_questions_idx;
drop index if exists leaderboard_stats_accuracy_idx;
drop index if exists leaderboard_stats_learned_words_idx;
drop index if exists leaderboard_stats_current_streak_idx;
drop index if exists leaderboard_stats_mixed_accuracy_idx;

create index if not exists leaderboard_stats_total_questions_idx
  on public.leaderboard_stats (period, period_start, total_questions desc, updated_at desc);

create index if not exists leaderboard_stats_accuracy_idx
  on public.leaderboard_stats (
    period,
    period_start,
    ((case when total_questions >= 20 then correct_count::numeric / nullif(total_questions, 0) else 0 end)) desc,
    total_questions desc
  );

create index if not exists leaderboard_stats_mixed_accuracy_idx
  on public.leaderboard_stats (
    period,
    period_start,
    ((case when mixed_test_total_questions >= 10 then mixed_test_correct_count::numeric / nullif(mixed_test_total_questions, 0) else 0 end)) desc,
    mixed_test_total_questions desc
  );

create index if not exists leaderboard_stats_current_streak_idx
  on public.leaderboard_stats (period, period_start, current_streak_days desc, updated_at desc);

create or replace function public.refresh_leaderboard_summary(
  p_display_name text default null,
  p_total_questions integer default 0,
  p_correct_count integer default 0,
  p_current_streak_days integer default 0,
  p_summary_key text default null,
  p_summary_at timestamptz default now(),
  p_period text default 'weekly',
  p_period_start date default current_date,
  p_mixed_test_total_questions integer default 0,
  p_mixed_test_correct_count integer default 0
)
returns public.leaderboard_stats
language plpgsql
security invoker
as $$
declare
  v_period text := coalesce(nullif(p_period, ''), 'weekly');
  v_period_start date := coalesce(
    p_period_start,
    case coalesce(nullif(p_period, ''), 'weekly')
      when 'all_time' then date '1970-01-01'
      else current_date
    end
  );
  v_row public.leaderboard_stats;
begin
  if auth.uid() is null then
    raise exception 'refresh_leaderboard_summary requires authentication'
      using errcode = '28000';
  end if;

  if v_period not in ('weekly', 'monthly', 'all_time') then
    raise exception 'invalid leaderboard period'
      using errcode = '22023';
  end if;

  if v_period = 'all_time' then
    v_period_start := date '1970-01-01';
  end if;

  if p_total_questions < 0
    or p_correct_count < 0
    or p_mixed_test_total_questions < 0
    or p_mixed_test_correct_count < 0
    or p_current_streak_days < 0
    or p_correct_count > p_total_questions
    or p_mixed_test_correct_count > p_mixed_test_total_questions then
    raise exception 'invalid leaderboard summary counters'
      using errcode = '22023';
  end if;

  insert into public.leaderboard_stats (
    user_id,
    period,
    period_start,
    display_name,
    total_questions,
    correct_count,
    learned_words,
    mixed_test_total_questions,
    mixed_test_correct_count,
    current_streak_days,
    last_summary_key,
    last_summary_at,
    updated_at
  )
  values (
    auth.uid(),
    v_period,
    v_period_start,
    nullif(p_display_name, ''),
    p_total_questions,
    p_correct_count,
    0,
    p_mixed_test_total_questions,
    p_mixed_test_correct_count,
    p_current_streak_days,
    nullif(p_summary_key, ''),
    p_summary_at,
    now()
  )
  on conflict (user_id, period, period_start) do update
  set
    display_name = coalesce(nullif(excluded.display_name, ''), leaderboard_stats.display_name),
    total_questions = excluded.total_questions,
    correct_count = excluded.correct_count,
    mixed_test_total_questions = excluded.mixed_test_total_questions,
    mixed_test_correct_count = excluded.mixed_test_correct_count,
    current_streak_days = excluded.current_streak_days,
    last_summary_key = coalesce(excluded.last_summary_key, leaderboard_stats.last_summary_key),
    last_summary_at = greatest(
      coalesce(leaderboard_stats.last_summary_at, '-infinity'::timestamptz),
      coalesce(excluded.last_summary_at, '-infinity'::timestamptz)
    ),
    updated_at = now()
  returning * into v_row;

  return v_row;
end;
$$;

create or replace function public.get_leaderboard(
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
security invoker
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
      coalesce(nullif(s.display_name, ''), 'Word learner') as display_name,
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
  ),
  ranked as (
    select
      row_number() over (
        order by score desc, tie_questions desc, updated_at asc, user_id asc
      )::integer as rank,
      user_id,
      display_name,
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

notify pgrst, 'reload schema';

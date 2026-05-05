set check_function_bodies = off;

create table if not exists public.leaderboard_stats (
  user_id uuid primary key references auth.users(id) on delete cascade,
  display_name text,
  total_questions integer not null default 0 check (total_questions >= 0),
  correct_count integer not null default 0 check (correct_count >= 0),
  learned_words integer not null default 0 check (learned_words >= 0),
  current_streak_days integer not null default 0 check (current_streak_days >= 0),
  last_summary_key text,
  last_summary_at timestamptz,
  updated_at timestamptz not null default now(),
  constraint leaderboard_correct_lte_total check (correct_count <= total_questions)
);

create index if not exists leaderboard_stats_total_questions_idx
  on public.leaderboard_stats (total_questions desc, updated_at desc);

create index if not exists leaderboard_stats_accuracy_idx
  on public.leaderboard_stats (
    ((case when total_questions >= 20 then correct_count::numeric / nullif(total_questions, 0) else 0 end)) desc,
    total_questions desc
  );

create index if not exists leaderboard_stats_learned_words_idx
  on public.leaderboard_stats (learned_words desc, updated_at desc);

create index if not exists leaderboard_stats_current_streak_idx
  on public.leaderboard_stats (current_streak_days desc, updated_at desc);

create or replace function public.refresh_leaderboard_summary(
  p_display_name text default null,
  p_total_questions integer default 0,
  p_correct_count integer default 0,
  p_learned_words integer default 0,
  p_current_streak_days integer default 0,
  p_summary_key text default null,
  p_summary_at timestamptz default now()
)
returns public.leaderboard_stats
language plpgsql
security invoker
as $$
declare
  v_row public.leaderboard_stats;
begin
  if auth.uid() is null then
    raise exception 'refresh_leaderboard_summary requires authentication'
      using errcode = '28000';
  end if;

  if p_total_questions < 0
    or p_correct_count < 0
    or p_learned_words < 0
    or p_current_streak_days < 0
    or p_correct_count > p_total_questions then
    raise exception 'invalid leaderboard summary counters'
      using errcode = '22023';
  end if;

  insert into public.leaderboard_stats (
    user_id,
    display_name,
    total_questions,
    correct_count,
    learned_words,
    current_streak_days,
    last_summary_key,
    last_summary_at,
    updated_at
  )
  values (
    auth.uid(),
    nullif(p_display_name, ''),
    p_total_questions,
    p_correct_count,
    p_learned_words,
    p_current_streak_days,
    nullif(p_summary_key, ''),
    p_summary_at,
    now()
  )
  on conflict (user_id) do update
  set
    display_name = coalesce(nullif(excluded.display_name, ''), leaderboard_stats.display_name),
    total_questions = greatest(leaderboard_stats.total_questions, excluded.total_questions),
    correct_count = case
      when excluded.total_questions >= leaderboard_stats.total_questions then excluded.correct_count
      else leaderboard_stats.correct_count
    end,
    learned_words = greatest(leaderboard_stats.learned_words, excluded.learned_words),
    current_streak_days = greatest(leaderboard_stats.current_streak_days, excluded.current_streak_days),
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
  p_limit integer default 50
)
returns table (
  rank integer,
  is_current_user boolean,
  user_id uuid,
  display_name text,
  total_questions integer,
  correct_count integer,
  accuracy_percent numeric,
  learned_words integer,
  current_streak_days integer,
  updated_at timestamptz
)
language sql
stable
security invoker
as $$
  with scored as (
    select
      s.user_id,
      coalesce(nullif(s.display_name, ''), 'Word learner') as display_name,
      s.total_questions,
      s.correct_count,
      case
        when s.total_questions > 0 then round((s.correct_count::numeric * 100.0) / s.total_questions, 1)
        else 0
      end as accuracy_percent,
      s.learned_words,
      s.current_streak_days,
      s.updated_at,
      case p_metric
        when 'accuracy' then case when s.total_questions >= 20 then (s.correct_count::numeric * 100.0) / s.total_questions else -1 end
        when 'learnedWords' then s.learned_words::numeric
        when 'currentStreak' then s.current_streak_days::numeric
        else s.total_questions::numeric
      end as score
    from public.leaderboard_stats s
  ),
  ranked as (
    select
      row_number() over (
        order by score desc, total_questions desc, updated_at asc, user_id asc
      )::integer as rank,
      user_id,
      display_name,
      total_questions,
      correct_count,
      accuracy_percent,
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
    ranked.learned_words,
    ranked.current_streak_days,
    ranked.updated_at
  from ranked
  where ranked.rank <= greatest(1, least(coalesce(p_limit, 50), 100))
    or ranked.user_id = auth.uid()
  order by ranked.rank asc;
$$;

alter table public.leaderboard_stats enable row level security;

drop policy if exists leaderboard_stats_owner_select on public.leaderboard_stats;
drop policy if exists leaderboard_stats_owner_insert on public.leaderboard_stats;
drop policy if exists leaderboard_stats_owner_update on public.leaderboard_stats;

create policy leaderboard_stats_public_select on public.leaderboard_stats
for select using (true);

create policy leaderboard_stats_owner_insert on public.leaderboard_stats
for insert with check (auth.uid() = user_id);

create policy leaderboard_stats_owner_update on public.leaderboard_stats
for update using (auth.uid() = user_id) with check (auth.uid() = user_id);

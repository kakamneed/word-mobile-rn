set check_function_bodies = off;

create or replace function public.get_reward_image_vote_leaderboard(
  p_week_start date default current_date,
  p_limit integer default 50
)
returns table (
  rank integer,
  image_id uuid,
  owner_id uuid,
  display_name text,
  storage_path text,
  public_url text,
  original_filename text,
  mime_type text,
  vote_count integer,
  voted_by_me boolean,
  selected_as_tag boolean,
  created_at timestamptz
)
language sql
stable
security definer
set search_path = public
as $$
  with normalized as (
    select coalesce(
      p_week_start,
      (current_date - ((extract(isodow from current_date)::int - 1) * interval '1 day'))::date
    ) as week_start
  ),
  scored as (
    select
      i.image_id,
      i.owner_id,
      coalesce(nullif(p.display_name, ''), 'Word learner') as display_name,
      i.storage_path,
      i.public_url,
      i.original_filename,
      i.mime_type,
      count(v.image_id)::integer as vote_count,
      exists (
        select 1
        from public.reward_image_votes my_vote
        where my_vote.image_id = i.image_id
          and my_vote.week_start = n.week_start
          and my_vote.voter_id = auth.uid()
      ) as voted_by_me,
      i.selected_as_tag,
      i.created_at
    from public.reward_images i
    left join public.profiles p on p.user_id = i.owner_id
    cross join normalized n
    left join public.reward_image_votes v
      on v.image_id = i.image_id
     and v.week_start = n.week_start
    where i.moderation_status = 'approved'
      and i.is_withdrawn = false
    group by
      i.image_id,
      i.owner_id,
      p.display_name,
      i.storage_path,
      i.public_url,
      i.original_filename,
      i.mime_type,
      n.week_start,
      i.selected_as_tag,
      i.created_at
  ),
  ranked as (
    select
      row_number() over (order by vote_count desc, created_at asc, image_id asc)::integer as rank,
      *
    from scored
  )
  select
    ranked.rank,
    ranked.image_id,
    ranked.owner_id,
    ranked.display_name,
    ranked.storage_path,
    ranked.public_url,
    ranked.original_filename,
    ranked.mime_type,
    ranked.vote_count,
    ranked.voted_by_me,
    ranked.selected_as_tag,
    ranked.created_at
  from ranked
  where ranked.rank <= greatest(1, least(coalesce(p_limit, 50), 100))
  order by ranked.rank asc;
$$;

select pg_notify('pgrst', 'reload schema');

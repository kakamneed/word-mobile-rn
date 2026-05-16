set check_function_bodies = off;

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'reward-images',
  'reward-images',
  true,
  524288,
  array['image/jpeg', 'image/png', 'image/webp']
)
on conflict (id) do update
set
  public = excluded.public,
  file_size_limit = excluded.file_size_limit,
  allowed_mime_types = excluded.allowed_mime_types;

create table if not exists public.reward_images (
  image_id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references auth.users(id) on delete cascade,
  storage_path text not null unique,
  public_url text,
  original_filename text not null default '',
  mime_type text not null default 'image/jpeg',
  moderation_status text not null default 'pending',
  moderation_reason text not null default '',
  moderation_checked_at timestamptz,
  selected_as_tag boolean not null default false,
  draw_pool_eligible boolean not null default false,
  is_withdrawn boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint reward_images_mime_check check (mime_type in ('image/jpeg', 'image/png', 'image/webp')),
  constraint reward_images_moderation_check check (moderation_status in ('pending', 'approved', 'rejected')),
  constraint reward_images_storage_path_check check (length(trim(storage_path)) > 0)
);

create table if not exists public.reward_image_votes (
  image_id uuid not null references public.reward_images(image_id) on delete cascade,
  voter_id uuid not null references auth.users(id) on delete cascade,
  week_start date not null,
  created_at timestamptz not null default now(),
  primary key (image_id, voter_id, week_start)
);

create index if not exists reward_images_public_idx
  on public.reward_images (moderation_status, is_withdrawn, selected_as_tag, created_at desc);

create index if not exists reward_images_owner_idx
  on public.reward_images (owner_id, moderation_status, created_at desc);

create index if not exists reward_image_votes_week_idx
  on public.reward_image_votes (week_start, image_id);

drop trigger if exists reward_images_set_updated_at on public.reward_images;
create trigger reward_images_set_updated_at
before update on public.reward_images
for each row execute function public.set_updated_at();

alter table public.reward_images enable row level security;
alter table public.reward_image_votes enable row level security;

drop policy if exists reward_images_owner_select on public.reward_images;
drop policy if exists reward_images_public_select on public.reward_images;
drop policy if exists reward_images_owner_insert on public.reward_images;
drop policy if exists reward_images_owner_update on public.reward_images;

create policy reward_images_owner_select on public.reward_images
for select using (auth.uid() = owner_id);

create policy reward_images_public_select on public.reward_images
for select using (
  moderation_status = 'approved'
  and is_withdrawn = false
);

create policy reward_images_owner_insert on public.reward_images
for insert with check (auth.uid() = owner_id);

create policy reward_images_owner_update on public.reward_images
for update using (auth.uid() = owner_id) with check (auth.uid() = owner_id);

drop policy if exists reward_image_votes_public_select on public.reward_image_votes;
drop policy if exists reward_image_votes_owner_insert on public.reward_image_votes;

create policy reward_image_votes_public_select on public.reward_image_votes
for select using (true);

create policy reward_image_votes_owner_insert on public.reward_image_votes
for insert with check (auth.uid() = voter_id);

drop policy if exists reward_images_bucket_public_select on storage.objects;
drop policy if exists reward_images_bucket_owner_insert on storage.objects;
drop policy if exists reward_images_bucket_owner_update on storage.objects;
drop policy if exists reward_images_bucket_owner_delete on storage.objects;

create policy reward_images_bucket_public_select on storage.objects
for select using (bucket_id = 'reward-images');

create policy reward_images_bucket_owner_insert on storage.objects
for insert with check (
  bucket_id = 'reward-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy reward_images_bucket_owner_update on storage.objects
for update using (
  bucket_id = 'reward-images'
  and auth.uid()::text = (storage.foldername(name))[1]
) with check (
  bucket_id = 'reward-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

create policy reward_images_bucket_owner_delete on storage.objects
for delete using (
  bucket_id = 'reward-images'
  and auth.uid()::text = (storage.foldername(name))[1]
);

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

create or replace function public.vote_reward_image(
  p_image_id uuid,
  p_week_start date default current_date
)
returns table (
  image_id uuid,
  week_start date,
  inserted boolean,
  vote_count integer
)
language plpgsql
security invoker
as $$
declare
  v_week_start date := coalesce(
    p_week_start,
    (current_date - ((extract(isodow from current_date)::int - 1) * interval '1 day'))::date
  );
  v_inserted boolean := false;
  v_count integer := 0;
begin
  if auth.uid() is null then
    raise exception 'vote_reward_image requires authentication'
      using errcode = '28000';
  end if;

  if not exists (
    select 1
    from public.reward_images i
    where i.image_id = p_image_id
      and i.moderation_status = 'approved'
      and i.is_withdrawn = false
  ) then
    raise exception 'reward image is not available for voting'
      using errcode = '22023';
  end if;

  insert into public.reward_image_votes (image_id, voter_id, week_start)
  values (p_image_id, auth.uid(), v_week_start)
  on conflict do nothing;

  get diagnostics v_count = row_count;
  v_inserted := v_count > 0;

  select count(*)::integer
  into v_count
  from public.reward_image_votes v
  where v.image_id = p_image_id
    and v.week_start = v_week_start;

  return query select p_image_id, v_week_start, v_inserted, v_count;
end;
$$;

select pg_notify('pgrst', 'reload schema');

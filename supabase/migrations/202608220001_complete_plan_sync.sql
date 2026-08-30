alter table public.plan_configs
  add column if not exists high_frequency_per_day integer not null default 0
    check (high_frequency_per_day >= 0),
  add column if not exists question_type_weights_by_mode jsonb;

create or replace function public.project_future_plan_to_plan_config()
returns trigger
language plpgsql
security invoker
set search_path = public
as $$
declare
  draft jsonb;
  targets jsonb;
  ratio_item jsonb;
  ratio_map jsonb := '{}'::jsonb;
  plan_version bigint;
begin
  if new.domain <> 'planPreference'
    or new.tombstone
    or new.payload ->> 'kind' <> 'futurePlan'
    or new.payload ->> 'schemaVersion' <> '1'
  then
    return new;
  end if;

  draft := new.payload -> 'draft';
  targets := draft -> 'targets';
  if jsonb_typeof(draft) <> 'object'
    or jsonb_typeof(targets) <> 'object'
    or jsonb_typeof(draft -> 'questionTypeRatios') <> 'array'
  then
    return new;
  end if;

  for ratio_item in select value from jsonb_array_elements(draft -> 'questionTypeRatios')
  loop
    if coalesce(ratio_item ->> 'mode', '') <> ''
      and jsonb_typeof(ratio_item -> 'weights') = 'object'
    then
      ratio_map := ratio_map || jsonb_build_object(ratio_item ->> 'mode', ratio_item -> 'weights');
    end if;
  end loop;

  begin
    plan_version := floor(extract(epoch from (new.payload ->> 'updatedAt')::timestamptz) * 1000)::bigint;
  exception when others then
    plan_version := floor(extract(epoch from new.updated_at) * 1000)::bigint;
  end;

  insert into public.plan_configs (
    user_id,
    name,
    new_words_per_day,
    review_words_per_day,
    mixed_test_per_day,
    wrong_word_test_per_day,
    high_frequency_per_day,
    root_affix_per_day,
    growth_rule_mode,
    shared_growth_rule,
    growth_rules_by_mode,
    question_type_weights_by_mode,
    version
  ) values (
    new.user_id,
    'Word Net plan',
    greatest(0, coalesce((targets ->> 'newWord')::integer, 0)),
    greatest(0, coalesce((targets ->> 'review')::integer, 0)),
    greatest(0, coalesce((targets ->> 'mixedTest')::integer, 0)),
    greatest(0, coalesce((targets ->> 'wrongWordReinforcement')::integer, 0)),
    greatest(0, coalesce((targets ->> 'highFrequency')::integer, 0)),
    greatest(0, coalesce((targets ->> 'rootAffix')::integer, 0)),
    draft ->> 'growthRuleMode',
    draft -> 'sharedGrowthRule',
    draft -> 'growthRulesByMode',
    ratio_map,
    plan_version
  )
  on conflict (user_id) do update set
    new_words_per_day = excluded.new_words_per_day,
    review_words_per_day = excluded.review_words_per_day,
    mixed_test_per_day = excluded.mixed_test_per_day,
    wrong_word_test_per_day = excluded.wrong_word_test_per_day,
    high_frequency_per_day = excluded.high_frequency_per_day,
    root_affix_per_day = excluded.root_affix_per_day,
    growth_rule_mode = coalesce(excluded.growth_rule_mode, public.plan_configs.growth_rule_mode),
    shared_growth_rule = coalesce(excluded.shared_growth_rule, public.plan_configs.shared_growth_rule),
    growth_rules_by_mode = coalesce(excluded.growth_rules_by_mode, public.plan_configs.growth_rules_by_mode),
    question_type_weights_by_mode = excluded.question_type_weights_by_mode,
    version = excluded.version
  where public.plan_configs.version <= excluded.version;

  return new;
end;
$$;

drop trigger if exists portable_future_plan_projection on public.portable_envelopes;
create trigger portable_future_plan_projection
after insert or update on public.portable_envelopes
for each row execute function public.project_future_plan_to_plan_config();

update public.portable_envelopes set updated_at = updated_at
where domain = 'planPreference' and not tombstone;

notify pgrst, 'reload schema';

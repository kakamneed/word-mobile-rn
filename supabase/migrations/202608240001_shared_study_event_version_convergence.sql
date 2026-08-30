-- Content versions are release identities, not protocol versions. Keep the
-- shared study-event payload source-bound while accepting every non-empty
-- version admitted by the Web content catalog.

create or replace function public.project_accepted_event_to_study_event()
returns trigger
language plpgsql
security invoker
set search_path = public
as $$
declare
  event jsonb;
  source jsonb;
  feedback jsonb;
  mobile_entry_id bigint;
  normalized_question_type text;
  web_device_id uuid;
  shared_event_id uuid;
  occurred_at timestamptz;
  response_time_ms bigint;
  canonical_answer text;
begin
  if new.domain <> 'acceptedEvent' or new.tombstone or new.payload ->> 'kind' <> 'acceptedAnswer' then
    return new;
  end if;

  event := new.payload -> 'event';
  source := event -> 'source';
  feedback := event -> 'feedback';
  if jsonb_typeof(event) is distinct from 'object'
    or jsonb_typeof(source) is distinct from 'object'
    or jsonb_typeof(feedback) is distinct from 'object'
    or event ->> 'entrySourceId' is distinct from source ->> 'entrySourceId'
    or jsonb_typeof(event -> 'hintUsed') is distinct from 'boolean' then
    return new;
  end if;

  mobile_entry_id := public.mobile_entry_id_for_source(event ->> 'entrySourceId', source ->> 'bookId');
  normalized_question_type := public.mobile_question_type(event ->> 'questionType');
  canonical_answer := coalesce(nullif(feedback ->> 'correctAnswer', ''), nullif(feedback ->> 'correctChoiceLabel', ''));
  if mobile_entry_id is null
    or normalized_question_type is null
    or coalesce(source ->> 'version', '') = ''
    or event ->> 'mode' not in ('newWord', 'review', 'mixedTest', 'wrongWordReinforcement', 'highFrequency', 'rootAffix')
    or event ->> 'outcome' not in ('correct', 'fuzzyCorrect', 'incorrect', 'skipped')
    or coalesce(event ->> 'eventId', '') = ''
    or coalesce(event ->> 'sessionId', '') = ''
    or coalesce(event ->> 'questionId', '') = ''
    or coalesce(event ->> 'localDay', '') !~ '^\d{4}-\d{2}-\d{2}$'
    or jsonb_typeof(event -> 'elapsedMs') <> 'number'
    or (event ->> 'elapsedMs') !~ '^\d+$'
    or canonical_answer is null then
    return new;
  end if;

  begin
    occurred_at := (event ->> 'acceptedAt')::timestamptz;
    response_time_ms := (event ->> 'elapsedMs')::bigint;
    web_device_id := md5(new.user_id::text || ':word-net-web-pwa')::uuid;
    shared_event_id := md5(new.user_id::text || ':' || (event ->> 'eventId'))::uuid;
  exception when others then
    return new;
  end;

  insert into public.devices (
    device_id, user_id, platform, device_label, app_version, last_seen_at, revoked_at
  ) values (
    web_device_id, new.user_id, 'web-pwa', 'Word Net Web/PWA', 'phase61-shared-study-v1', now(), null
  )
  on conflict (device_id) do update set
    last_seen_at = now(),
    revoked_at = null,
    app_version = excluded.app_version
  where public.devices.user_id = new.user_id;

  insert into public.study_events (
    event_id, user_id, device_id, session_id, event_type,
    payload_json, occurred_at, idempotency_key
  ) values (
    shared_event_id,
    new.user_id,
    web_device_id,
    event ->> 'sessionId',
    'answer_submitted',
    jsonb_build_object(
      'schemaVersion', 1,
      'eventId', event ->> 'eventId',
      'questionId', event ->> 'questionId',
      'entryId', mobile_entry_id,
      'entrySourceId', event ->> 'entrySourceId',
      'bookId', source ->> 'bookId',
      'bookVersion', source ->> 'version',
      'mode', event ->> 'mode',
      'questionType', normalized_question_type,
      'outcome', event ->> 'outcome',
      'userResponse', coalesce(event ->> 'userResponse', ''),
      'canonicalAnswer', canonical_answer,
      'responseTimeMs', response_time_ms,
      'hintUsed', (event ->> 'hintUsed')::boolean,
      'localDay', event ->> 'localDay',
      'legacyPointProjection', false
    ),
    occurred_at,
    'word-net-web:' || new.user_id::text || ':' || (event ->> 'eventId')
  )
  on conflict (idempotency_key) do nothing;

  return new;
end;
$$;

drop trigger if exists portable_envelopes_project_study_event on public.portable_envelopes;
create trigger portable_envelopes_project_study_event
after insert on public.portable_envelopes
for each row execute function public.project_accepted_event_to_study_event();

revoke all on function public.project_accepted_event_to_study_event() from public, anon;

notify pgrst, 'reload schema';

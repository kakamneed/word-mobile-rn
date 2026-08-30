-- Portable accepted answers written before the shared event trigger existed
-- were already counted by the legacy study_word_points trigger. Backfill only
-- missing shared events and mark that provenance so readers can subtract the
-- legacy aggregate contribution before adding the immutable event.

with candidates as (
  select distinct
    envelope.user_id,
    md5(envelope.user_id::text || ':word-net-web-pwa')::uuid as device_id
  from public.portable_envelopes envelope
  where envelope.domain = 'acceptedEvent'
    and not envelope.tombstone
    and envelope.payload ->> 'kind' = 'acceptedAnswer'
)
insert into public.devices (
  device_id, user_id, platform, device_label, app_version, last_seen_at, revoked_at
)
select device_id, user_id, 'web-pwa', 'Word Net Web/PWA', 'phase61-shared-study-v1', now(), null
from candidates
on conflict (device_id) do update set
  last_seen_at = now(),
  revoked_at = null,
  app_version = excluded.app_version
where public.devices.user_id = excluded.user_id;

with candidates as (
  select
    envelope.user_id,
    envelope.payload -> 'event' as event,
    envelope.payload #> '{event,source}' as source,
    envelope.payload #> '{event,feedback}' as feedback
  from public.portable_envelopes envelope
  where envelope.domain = 'acceptedEvent'
    and not envelope.tombstone
    and envelope.payload ->> 'kind' = 'acceptedAnswer'
), valid as (
  select
    user_id,
    event,
    source,
    feedback,
    public.mobile_entry_id_for_source(event ->> 'entrySourceId', source ->> 'bookId') as entry_id,
    public.mobile_question_type(event ->> 'questionType') as question_type,
    coalesce(nullif(feedback ->> 'correctAnswer', ''), nullif(feedback ->> 'correctChoiceLabel', '')) as canonical_answer
  from candidates
  where jsonb_typeof(event) = 'object'
    and jsonb_typeof(source) = 'object'
    and jsonb_typeof(feedback) = 'object'
    and event ->> 'entrySourceId' = source ->> 'entrySourceId'
    and jsonb_typeof(event -> 'hintUsed') = 'boolean'
    and coalesce(event ->> 'eventId', '') <> ''
    and coalesce(event ->> 'sessionId', '') <> ''
    and coalesce(event ->> 'questionId', '') <> ''
    and coalesce(event ->> 'localDay', '') ~ '^\d{4}-\d{2}-\d{2}$'
    and pg_input_is_valid(event ->> 'acceptedAt', 'timestamp with time zone')
    and jsonb_typeof(event -> 'elapsedMs') = 'number'
    and (event ->> 'elapsedMs') ~ '^\d+$'
    and source ->> 'version' = '2026.1'
    and event ->> 'mode' in ('newWord', 'review', 'mixedTest', 'wrongWordReinforcement', 'highFrequency', 'rootAffix')
    and event ->> 'outcome' in ('correct', 'fuzzyCorrect', 'incorrect', 'skipped')
)
insert into public.study_events (
  event_id, user_id, device_id, session_id, event_type,
  payload_json, occurred_at, idempotency_key
)
select
  md5(user_id::text || ':' || (event ->> 'eventId'))::uuid,
  user_id,
  md5(user_id::text || ':word-net-web-pwa')::uuid,
  event ->> 'sessionId',
  'answer_submitted',
  jsonb_build_object(
    'schemaVersion', 1,
    'eventId', event ->> 'eventId',
    'questionId', event ->> 'questionId',
    'entryId', entry_id,
    'entrySourceId', event ->> 'entrySourceId',
    'bookId', source ->> 'bookId',
    'bookVersion', source ->> 'version',
    'mode', event ->> 'mode',
    'questionType', question_type,
    'outcome', event ->> 'outcome',
    'userResponse', coalesce(event ->> 'userResponse', ''),
    'canonicalAnswer', canonical_answer,
    'responseTimeMs', (event ->> 'elapsedMs')::bigint,
    'hintUsed', (event ->> 'hintUsed')::boolean,
    'localDay', event ->> 'localDay',
    'legacyPointProjection', true
  ),
  (event ->> 'acceptedAt')::timestamptz,
  'word-net-web:' || user_id::text || ':' || (event ->> 'eventId')
from valid
where entry_id is not null
  and question_type is not null
  and canonical_answer is not null
on conflict (idempotency_key) do nothing;

notify pgrst, 'reload schema';
